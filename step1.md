SPECTRUSTY Tutorial
===================

This is a part of the [tutorial] for the [SPECTRUSTY] library.

Step 1 - Baby steps
-------------------

First, let's [prepare your Rust crate](https://doc.rust-lang.org/cargo/getting-started/first-steps.html) and add the following to the `Cargo.toml` of your project:

```toml
[dependencies]
spectrusty = "0.1"
```

To make this super easy let's say we want to build a bare ZX Spectrum 16k without any peripherals.

What we need is: a `CPU`, some memory and a chipset.

To nicely organize code we'll define a struct for holding our components:

```rust
use spectrusty::z80emu::{Cpu, Z80NMOS};
use spectrusty::chip::{ControlUnit, MemoryAccess, ula::UlaPAL};
use spectrusty::memory::Memory16k;

struct ZxSpectrum16k {
    cpu: Z80NMOS,
    ula: UlaPAL<Memory16k>
}
```

As you can see, there is lot of imports already... and brace yourself for much, much more. You could use an import all `*` facility instead, but this way I can show where each components comes from.

Ok, so far we have added a CPU implementation and a chipset with the memory type declared as its generic parameter.
[Ula] implements the "heart" of one of ZX Spectrum 16k or 48k version and [UlaPAL] is a slightly more specialized type for the 50Hz PAL version.

But what if we wanted to use another type of memory or a CPU and resuse the same code?

We can refactor slightly our struct so it'll also accept generic parameters:

```rust
// ... more imports going in order
use spectrusty::memory::{ZxMemory, Memory16k, Memory48k};

#[derive(Default)]
struct ZxSpectrum<C: Cpu, M: ZxMemory> {
    cpu: C,
    ula: UlaPAL<M>
}

// Let's create some sugar definitions
type ZxSpectrum16k<C> = ZxSpectrum<C, Memory16k>;
type ZxSpectrum48k<C> = ZxSpectrum<C, Memory48k>;
```

Sooo.. we can now not only have a different memory but also a different CPU variant. For example: [Z80CMOS].

Now for the `main` dish:

```rust
// we can always work on errors later, this is always an easy default
type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let spectrum = ZxSpectrum16k::<Z80NMOS>::default();
    //...
    Ok(())
}
```

Later I'll show you how to make this struct also accept other types of chipsets.

Finally, we have created an instance of our Spectrum.

But how exactly do we "run" it?

Our `spectrum` represents a kind of a Finite State Machine, perhaps even a [Turing Machine]. By "running" it, we mean that the machine can alter its state by executing instructions. The state of this particular FSM is being altered by executing Z80 machine code that reside in its memory. So... we'd better have some code loaded into memory first, before we try to execute it. Otherwise we'll be executing a series of `NOP`s (0x00) followed by `RST 38` (0xFF) in the upper 32kb RAM.

A significant part of what makes Sinclair ZX Spectrum is not its hardware but rather its software.
We'll start with a program that resides in Spectrum's [ROM]:

```rust
    let rom_file = std::fs::File::open("resources/roms/48.rom")?;
    spectrum.ula.memory_mut().load_into_rom(rom_file)?;
```

You may get a copy of a [48.rom] file as Amstrad has kindly given an open permition to re-distribute the Spectrum's ROM content.

Now, can we finally run it?

Short ansert: yes.

Long answer: yes, but how do we know if something is even happening "inside" the F.S.M? I think that you'd like to be able to experience the side-effects of changes of the Spectrum's internal state as video and audio effects. It would be even better if we could provide it with some input, e.g. keyboard presses.

So not only we need to execute Z80 code, but at the same time we also need to render some video and audio. How?

We can simply take advantage of the way `ULA` generates video signal. The electron beam is scanning each line of the tube from the left to the right then retracing to the beginning of the next line again and again from the top to the bottom of the screen (described from the user perspective looking at the monitor).

When the beam reaches the bottom of the screen, the process is being repeated. Such a pass of a monitor beam is called a frame (at least in this tutorial). Additionally the number of CPU cycles (or `T-states` that are being used as time units here) of every frame is always the same.

Knowing this, the answer is obvious: let's run our emulator in a loop. First we execute code for the constant number of T-states, then from the collected video and audio data we render video as still images and audio as short audio samples, then we collect the user inputs and pass it to the state machine. Rinse and repeat.

The order of the above steps, or rather the point when the loop begins is not really that important unless we want to save a snapshot of our FSM. (more on that topic later, I promise).

```rust
// emhhhhh....ok, let's go
use spectrusty::video::{
    Video, Palette, PixelBuffer, BorderSize,
    pixel::{PixelBufA24, SpectrumPalRGB24}
};
use spectrusty::peripherals::{KeyboardInterface, ZXKeyboardMap};

fn main() -> Result<()> {
    //... ✂
    //... later in main()

    // here we select the largest possible border,
    // we can ask user about it but it's out of this tutorial's scope
    let border = BorderSize::Full;

    while is_running() {
        spectrum.update_keyboard( update_keys );

        spectrum.run_frame();

        let (width, height) = <UlaPAL<Memory16k> as Video>::render_size_pixels(border);
        let (video_buffer, pitch) = acquire_video_buffer(width, height);
        spectrum.render_video::<SpectrumPalRGB24>(video_buffer, pitch, border);

        update_display();

        synchronize_frame();
    }
}
```

The [PixelBuffer] trait consists of methods for handling the way pixel colors are being placed into the raw slices of bytes, sometimes called video frame buffers. There are a few implementations of `PixelBuffer` available in [video::pixel] module for most common pixel formats. For the purpose of this example we will use [PixelBufA24] which defines a single color pixel as an array of 3 bytes representing: red, green and blue channels. The [SpectrumPalRGB24] is an implementation of the `Palette` trait providing particular colors.

Functions such as: `is_running`, `update_keys`, `acquire_video_buffer` and `update_display` depend solely on the emulator host environment and is out of the scope of the Spectrusty's library (with some notable exceptions). The implementation of this should be provided by the emulator builder. However the [spectrusty-utils] crate provides some helper methods to ease this task even further.

You should definitely check the implementation of this tutorial [step1.rs] or the examples directory of SPECTRUSTY repository to see how it can be done for [SDL2] or even a [web browser].

Now let's look inside our new `ZxSpectrum`'s methods:

```rust
// the type of PixelBuffer (so we can easily replace it).
type PixelBuf<'a> = PixelBufA24<'a>;
// the type of PixelBuffer::Pixel.
type Pixel<'a> = <PixelBuf<'a> as PixelBuffer<'a>>::Pixel;

impl<C: Cpu, M: ZxMemory> ZxSpectrum<C, M> {
    fn update_keyboard<F: FnOnce(ZXKeyboardMap) -> ZXKeyboardMap>(
            &mut self,
            update_keys: F)
    {
        let keymap = update_keys( self.ula.get_key_state() );
        self.ula.set_key_state(keymap);
    }
    // this one looks very simple, however we will add more to this function later
    // so let's not give up on it yet.
    fn run_frame(&mut self) {
        self.ula.execute_next_frame(&mut self.cpu);
    }
    // `video_buffer` is a mutable slice of bytes.
    // `pitch` is the number of bytes that the single row of pixels occupy.
    // `border` determines the size of the rendered screen.
    fn render_video<'a, P: Palette<Pixel=Pixel<'a>>>(
        &mut self,
        buffer: &mut [u8],
        pitch: usize,
        border: BorderSize)
    {
        self.ula.render_video_frame::<PixelBuf, P>(buffer, pitch, border);
    }
    // so we can reset our Spectrum
    fn reset(&mut self, hard: bool) {
        self.ula.reset(&mut self.cpu, hard)
    }
    // so we can trigger NMI
    fn trigger_nmi(&mut self) -> bool {
        self.ula.nmi(&mut self.cpu)
    }
}
```

The [Palette] trait is for retrieving actual pixel colors corresponding to Spectrum's color palette.
You may define a palette implementation yourself or use one of palettes defined in the [video::pixel] module. Remember that the associated [Palette::Pixel] type should match the associated type of [PixelBuffer::Pixel].

The last missing part is synchronization. After each iteration of our loop we need to pause the running thread for some time to match the rate of Spectrum's CPU clock.

For that we'll be using [ThreadSyncTimer]:

```rust
// just one new import?
use spectrusty::chip::ThreadSyncTimer;

fn main() -> Result<()> {
    //... ✂
    //... later in main()

    let frame_duration_nanos = <UlaPAL<Memory16k> as HostConfig>::frame_duration_nanos();
    let mut sync = ThreadSyncTimer::new(frame_duration_nanos);
    let mut synchronize_frame = || {
        if let Err(missed) = sync.synchronize_thread_to_frame() {
            println!("*** lagging behind: {} frames ***", missed);
        }
    };

    while is_running() {
        //... ✂
        synchronize_frame();
    }
}
```

Assuming you have taken care of the host environment, this is enough to run your ZX Spectrum emulator. At this point, you should be able to write BASIC programs using a keyboard.


### Some entropy

To make your Spectrum feel slightly more real, let's initialize Spectrum's memory with some entropy when it's being initialized.

First, you need to add the [rand] crate to your `Cargo.toml`:

```toml
[dependencies]
spectrusty = "0.1"
rand = "0.7"
```

then add:

```rust
use rand::prelude::*;

fn main() -> Result<()> {
    //... ✂
    // some entropy in memory for nice visuals at boot
    spectrum.ula.memory_mut().fill_mem(.., random)?;
    // get the software
    let rom_file = std::fs::File::open("resources/48.rom")?;
    // put the software into the hardware
    spectrum.ula.memory_mut().load_into_rom(rom_file)?;
    //... ✂
}
```

### Generalized solution

Up to this point we assumed only one specific Spectrum type can be used in your program. What if you'd like to be able to switch the Spectrum model run-time?

For that, we'll have to slightly pivot and embrace generics a little bit more. If you are new to Rust, it would be wise to read more about generics now, [here in this book](https://doc.rust-lang.org/book/ch10-00-generics.html).

First let's move out the emulator loop part to the separate, polymorphic function:

```rust
fn run<C: Cpu, M: ZxMemory>(
        spectrum: &mut ZxSpectrum<C, M>,
        env: HostEnvironment,
    ) -> Result<Action>
{
    let HostEnvironment { border, ... } = env;
    let (width, height) = <Ula<M> as Video>::render_size_pixels(border);

    let mut sync = ThreadSyncTimer::new(UlaPAL::<M>::frame_duration_nanos());
    let mut synchronize_frame = || {
        if let Err(missed) = sync.synchronize_thread_to_frame() {
            println!("*** paused for: {} frames ***", missed);
        }
    };

    // emulator main loop
    while is_running() {
        //... ✂
        if Some(model) = user_want_to_switch_model() {
            return Ok(Action::ChangeModel(model))
        }

        synchronize_frame();
    }

    Ok(Action::Exit)
}
```

The generic parameters are exactly the same as used with `ZxSpectrum` struct definition, so there's nothing very interesting happening here yet.

Now the `HostEnvironment` struct will wrap everything that is needed to run our emulator in the host environment, like a window handle, video buffer, audio host, event pump, etc. The new function accepts a mutable reference to our Spectrum and returns an action request, which may look like this:

```rust
#[derive(Debug, Clone, Copy)]
enum Action {
    ChangeModel(ModelReq),
    Exit
}

#[derive(Debug, Clone, Copy)]
enum ModelReq {
    Spectrum16,
    Spectrum48,
}
```

Then in the `main` we add a loop that calls `run` and changes the model according to the request or just quits:

```rust
    let mut spec16 = ZxSpectrum16k::<Z80NMOS>::default();
    //... ✂
    let mut spectrum = ZxSpectrumModel::Spectrum16(spec16);

    loop {
        use ZxSpectrumModel::*;
        let env = bundle_environment(/*...*/);
        let req = match &mut spectrum {
            Spectrum16(spec16) => run(spec16, env)?,
            Spectrum48(spec48) => run(spec48, env)?
        };

        spectrum = match req {
            Action::ChangeModel(spec) => spectrum.change_model(spec),
            Action::Exit => break
        };
    }
```

The only missing part yet is the `ZxSpectrumModel` enum which should be rather straightforward:

```rust
enum ZxSpectrumModel<C: Cpu> {
    Spectrum16(ZxSpectrum16k<C>),
    Spectrum48(ZxSpectrum48k<C>),
}
```
... and its implementation which we can make a little more interesting by implementing model hot-swapping (at least with regards to its CPU and memory).

```rust
impl<C: Cpu> ZxSpectrumModel<C> {
    fn into_cpu(self) -> C {
        match self {
            ZxSpectrumModel::Spectrum16(spec16) => spec16.cpu,
            ZxSpectrumModel::Spectrum48(spec48) => spec48.cpu,
        }        
    }
    fn as_mem_ref(&self) -> &[u8] {
        match self {
            ZxSpectrumModel::Spectrum16(spec16) => spec16.ula.memory_ref().mem_ref(),
            ZxSpectrumModel::Spectrum48(spec48) => spec48.ula.memory_ref().mem_ref(),
        }
    }
    fn border_color(&self) -> u8  {
        match self {
            ZxSpectrumModel::Spectrum16(spec16) => spec16.ula.border_color(),
            ZxSpectrumModel::Spectrum48(spec48) => spec48.ula.border_color(),
        }
    }
    // hot-swap hardware models
    fn change_model(self, request: ModelReq) -> Self {
        use ZxSpectrumModel::*;
        match (&self, request) {
            (Spectrum16(..), ModelReq::Spectrum16)|
            (Spectrum48(..), ModelReq::Spectrum48) => return self,
            _ => {}
        }
        let mem = self.as_mem_ref();
        let border = self.border_color();
        match request {
            ModelReq::Spectrum16 => {
                let mut spec16 = ZxSpectrum16k::<C>::default();
                let mem16 = spec16.ula.memory_mut().mem_mut();
                let len = mem16.len().min(mem.len());
                mem16[..len].copy_from_slice(&mem[..len]);
                spec16.cpu = self.into_cpu();
                spec16.ula.set_border_color(border);
                Spectrum16(spec16)
            }
            ModelReq::Spectrum48 => {
                let mut spec48 = ZxSpectrum48k::<C>::default();
                let mem48 = spec48.ula.memory_mut().mem_mut();
                let len = mem48.len().min(mem.len());
                mem48[..len].copy_from_slice(&mem[..len]);
                spec48.cpu = self.into_cpu();
                spec48.ula.set_border_color(border);
                Spectrum48(spec48)
            }
        }
    }
}
```

Well, I think that's already enough for this step. More models will come in the future and we'll extend the `run` function yet many times over, that's for sure. Just stay with me a little longer.


### Example

The [example][step1.rs] program using [minifb] and covering the scope of this tutorial can be run with:

```sh
cargo run --bin step1 --release
```

### Next

[Step 2 - Buzz on](step2.md).

Back to [index][tutorial].

<script>var clicky_site_ids = clicky_site_ids || []; clicky_site_ids.push(101270192);</script>
<script async src="//static.getclicky.com/js"></script>

[SPECTRUSTY]: https://royaltm.github.io/spectrusty/
[tutorial]: https://royaltm.github.io/spectrusty-tutorial/
[step1.rs]: https://github.com/royaltm/spectrusty-tutorial/blob/master/src/bin/step1.rs
[minifb]: https://crates.io/crates/minifb
[Turing Machine]: https://cs.stackexchange.com/questions/16729/are-real-computers-finite-state-machines
[prepare your Rust crate]: https://doc.rust-lang.org/cargo/getting-started/first-steps.html
[spectrusty-utils]: https://crates.io/crates/spectrusty-utils
[ROM]: https://github.com/royaltm/spectrusty/tree/master/resources/
[48.rom]: https://github.com/royaltm/spectrusty/tree/master/resources/roms/48.rom
[SDL2]: https://github.com/royaltm/spectrusty/tree/master/examples/sdl2-zxspectrum/
[web browser]: https://github.com/royaltm/spectrusty/tree/master/examples/web-zxspectrum/
[minifb]: https://crates.io/crates/minifb
[rand]: https://crates.io/crates/rand
[Ula]: https://docs.rs/spectrusty/*/spectrusty/chip/ula/struct.Ula.html
[UlaPAL]: https://docs.rs/spectrusty/*/spectrusty/chip/ula/type.UlaPAL.html
[Z80CMOS]: https://docs.rs/z80emu/*/z80emu/z80/type.Z80CMOS.html
[PixelBuffer]: https://docs.rs/spectrusty/*/spectrusty/video/pixel/trait.PixelBuffer.html
[PixelBuffer::Pixel]: https://docs.rs/spectrusty/*/spectrusty/video/pixel/trait.PixelBuffer.html#associatedtype.Pixel
[Palette]: https://docs.rs/spectrusty/*/spectrusty/video/pixel/trait.Palette.html
[Palette::Pixel]: https://docs.rs/spectrusty/*/spectrusty/video/pixel/trait.Palette.html#associatedtype.Pixel
[video::pixel]: https://docs.rs/spectrusty/*/spectrusty/video/pixel/index.html
[PixelBufA24]: https://docs.rs/spectrusty/*/spectrusty/video/pixel/struct.PixelBufA24.html
[SpectrumPalRGB24]: https://docs.rs/spectrusty/*/spectrusty/video/pixel/struct.SpectrumPalRGB24.html
[ThreadSyncTimer]: https://docs.rs/spectrusty/*/spectrusty/chip/struct.ThreadSyncTimer.html