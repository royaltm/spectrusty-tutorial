SPECTRUSTY Tutorial
===================

This is a part of the [tutorial] for the [SPECTRUSTY] library.

In this step, you can learn how to add different ZX Spectrum models with more peripherals.

![Step 5](example-step5.png)


Step 5 - 128
------------

128 means 128kb RAM. It also means:

* 32kB ROM with 128k software,
* ULA128 with MMU and shadow screen memory,
* an [AY-3-8910 Programmable Sound Processor](https://en.wikipedia.org/wiki/General_Instrument_AY-3-8910)
* .. with an [RS-232](https://en.wikipedia.org/wiki/RS-232) and an [external keypad](http://www.fruitcake.plus.com/Sinclair/Spectrum128/Keypad/Spectrum128Keypad.htm).

We are going to skip the `RS-232`, but otherwise we'll add a 128k model to the emulator that features all of the above.

First, update imports:

```rust
use spectrusty::audio::{
    AudioSample, EarMicAmps4, EarOutAmps4, EarInAmps2,
    Blep, BlepStereo, FromSample, UlaAudioFrame,
    synth::BandLimited,
    carousel::AudioFrameResult,
    host::cpal::AudioHandleAnyFormat
};
use spectrusty::bus::{
    BusDevice, VFNullDevice, OptionalBusDevice,
    joystick::{
        MultiJoystickBusDevice, JoystickSelect,
        JoystickInterface
    }
};
use spectrusty::chip::{
    ControlUnit, HostConfig, MemoryAccess,
    UlaCommon, Ula128MemFlags, UlaControl,
    ThreadSyncTimer,
    ula::{UlaPAL, UlaVideoFrame},
    ula128::{Ula128, Ula128VidFrame}
};
use spectrusty::peripherals::{
    ZXKeyboardMap,
    serial::{SerialKeypad, KeypadKeys},
    ay::audio::AyAmps
};
use spectrusty_utils::{
    tap::{Tape, Tap},
    keyboard::minifb::{
        update_keymap, update_keypad_keys,
        update_joystick_from_key_event
    }
};
```

Next, we'll refactor the `ZxSpectrum` struct, by replacing the `UlaPAL` with a generic `U` parameter:

```rust
#[derive(Default)]
struct ZxSpectrum<C: Cpu, U> {
    cpu: C,
    ula: U,
    nmi_request: bool,
    reset_request: Option<bool>,
    state: EmulatorState
}
```

The `reset_request` property works similar to `nmi_request`.

We'll define a specialized [Ula128] with an implementation of AY-3-8910 PSG connected to the keypad via its IOA port.

```rust
// a specialized AY-3-8910 bus device with a keypad
use spectrusty::bus::ay::serial128::Ay3_8912Keypad;
// define Ula128 with a static mandatory device
type Ula128AyKeypad<D=VFNullDevice<Ula128VidFrame>> = Ula128<
                                        Ay3_8912Keypad<Ula128VidFrame, D>
                                    >;
```

We need to specify the [Ula128VidFrame] so the device can be used with [Ula128].
We'll also use [VFNullDevice] instead of `TerminatorDevice` from the previous step.

Now for the helper types:

```rust
type ZxSpectrum16k<C, D> = ZxSpectrum<C, UlaPAL<Memory16k, D>>;
type ZxSpectrum48k<C, D> = ZxSpectrum<C, UlaPAL<Memory48k, D>>;
type ZxSpectrum128k<C, D> = ZxSpectrum<C, Ula128AyKeypad<D>>;
```

Because we'll be using chipsets that are using different [VideoFrame] implementations, we can't use the same devices for all of them. This is because their [BusDevice::Timestamp] will be also different. Instead, let's define a new type for the pluggable joystick that better suits these circumstances:

```rust
// a pluggable joystick with run-time selectable joystick types
type PluggableMultiJoyBusDevice<V> = OptionalBusDevice<
                                        MultiJoystickBusDevice<
                                                VFNullDevice<V>>,
                                        VFNullDevice<V>
                                    >;
```

The `OptionalBusDevice` new-type defined in the previous chapter should be deleted.

For the sake of simplicity, let's make the model enum less generic, by removing the `D` parameter.

```rust
enum ZxSpectrumModel<C: Cpu> {
    Spectrum16(
        ZxSpectrum16k<C, PluggableMultiJoyBusDevice<UlaVideoFrame>>
    ),
    Spectrum48(
        ZxSpectrum48k<C, PluggableMultiJoyBusDevice<UlaVideoFrame>>
    ),
    Spectrum128(
        ZxSpectrum128k<C, PluggableMultiJoyBusDevice<Ula128VidFrame>>
    ),
}
```

... and add a new variant to `ModelReq`:

```rust
#[derive(Debug, Clone, Copy)]
enum ModelReq {
    Spectrum16,
    Spectrum48,
    Spectrum128,
}
```

Because 128k model uses different ROM, again for the sake of simplicity we'll embed the ROM binaries in the target executable, instead of loading them in run time.

```rust
// add ROMS to the binary resources
static ROM48: &[u8]    = include_bytes!("../resources/roms/48.rom");
static ROM128_0: &[u8] = include_bytes!("../resources/roms/128-0.rom");
static ROM128_1: &[u8] = include_bytes!("../resources/roms/128-1.rom");
```

Let's also create dedicated `new_with_rom` methods for initializing each of the spectrum model types:

```rust
impl<C: Cpu, M: ZxMemory, D: BusDevice> ZxSpectrum<C, UlaPAL<M, D>>
    where Self: Default
{
    fn new_with_rom() -> Self {
        let mut spectrum = Self::default();
        let mem = spectrum.ula.memory_mut();
        mem.fill_mem(.., random).unwrap();
        mem.load_into_rom(ROM48).unwrap();
        spectrum
    }
}

impl<C: Cpu, D: BusDevice> ZxSpectrum<C, Ula128AyKeypad<D>>
    where Self: Default
{
    fn new_with_rom() -> Self {
        let mut spectrum = Self::default();
        let mem = spectrum.ula.memory_mut();
        mem.fill_mem(.., random).unwrap();
        mem.load_into_rom_bank(0, ROM128_0).unwrap();
        mem.load_into_rom_bank(1, ROM128_1).unwrap();
        spectrum
    }
}
```

[![Splash](splash-step5.png)][sword-of-ianna]


### Device access

Perhaps it's time to refactor access to the joystick interface. We also need a way to access the [128 keypad][SerialKeypad] to connect it with the user keyboard.

The `JoystickAccess` trait hasn't changed, I'll just include it here for completness:

```rust
trait JoystickAccess {
    type JoystickInterface: JoystickInterface + ?Sized;
    // Universal joystick interface access
    fn joystick_interface(&mut self) -> Option<&mut Self::JoystickInterface> {
        None
    }
    // Does nothing by default.
    fn select_joystick(&mut self, _joy: usize) {}
    fn current_joystick(&self) -> Option<&str> {
        None
    }
}
```

We will be changing it's implementation though, that's for sure. But instead of implementing it for different types of `ZxSpectrum` we'll make a single generic implementation using an intermediate trait.

```rust
type SerialKeypad128 = SerialKeypad<VFrameTs<Ula128VidFrame>>;

trait DeviceAccess {
    type JoystickDevice;

    fn joystick_bus_device_mut(&mut self) -> Option<&mut Self::JoystickDevice> {
        None
    }
    fn joystick_bus_device_ref(&self) -> Option<&Self::JoystickDevice> {
        None
    }
    fn keypad128_mut(&mut self) -> Option<&mut SerialKeypad128> {
        None
    }
}
```

`DeviceAccess` will be implemented directly on the chipset instead of model struct and will give us a conditional access to joystick as well as to the [128 keypad][SerialKeypad].

Having an intermediate ready, we can now implement `JoystickAccess`.

```rust
impl<C: Cpu, U: UlaCommon> JoystickAccess for ZxSpectrum<C, U>
    where U: DeviceAccess<JoystickDevice = PluggableMultiJoyBusDevice<
                                            <U as Video>::VideoFrame
                                           >
             >
{
    type JoystickInterface = dyn JoystickInterface;

    fn joystick_interface(&mut self) -> Option<&mut Self::JoystickInterface> {
        let sub_joy = self.state.sub_joy;
        self.ula.joystick_bus_device_mut().and_then(|joy_bus_dev| {
            joy_bus_dev.as_deref_mut()
                       .and_then(|j| j.joystick_interface(sub_joy))
        })
    }

    fn select_joystick(&mut self, joy_index: usize) {
        if let Some(joy_bus_dev) = self.ula.joystick_bus_device_mut() {
            let (joy_dev, index) = JoystickSelect::new_with_index(joy_index)
                .map(|(joy_sel, index)| 
                    (Some(MultiJoystickBusDevice::new_with(joy_sel)), index)
                )
                .unwrap_or((None, 0));
            **joy_bus_dev = joy_dev;
            self.state.sub_joy = index;
        }
    }

    fn current_joystick(&self) -> Option<&str> {
        self.ula.joystick_bus_device_ref()
                .and_then(|jbd| jbd.as_deref().map(Into::into))
    }
}
```

The implementation is similar to the one used in the step 4. But this time we are using methods of `DeviceAccess` trait to get access to the very specific `DeviceAccess::JoystickDevice` implementation. This is reflected in the `where` condition set for the generic type `U`.

We'll see in a moment why this is needed, when we get to implementing the `DeviceAccess` trait.

```rust
// implement for Ula with a default device for completness
impl<M: ZxMemory> DeviceAccess for UlaPAL<M> {
    type JoystickDevice = PluggableMultiJoyBusDevice<UlaVideoFrame>;
}

// implement for Ula with a joystick device
impl<M: ZxMemory> DeviceAccess for UlaPAL<M, PluggableMultiJoyBusDevice<UlaVideoFrame>> {
    type JoystickDevice = PluggableMultiJoyBusDevice<UlaVideoFrame>;

    fn joystick_bus_device_mut(&mut self) -> Option<&mut Self::JoystickDevice> {
        Some(self.bus_device_mut())
    }
    fn joystick_bus_device_ref(&self) -> Option<&Self::JoystickDevice> {
        Some(self.bus_device_ref())
    }
}

// implement for Ula128 with a default device for completness
impl DeviceAccess for Ula128AyKeypad {
    type JoystickDevice = PluggableMultiJoyBusDevice<Ula128VidFrame>;

    fn keypad128_mut(&mut self) -> Option<&mut SerialKeypad128> {
        Some(&mut self.bus_device_mut().ay_io.port_a.serial1)
    }
}

// implement for Ula128 with a joystick device
impl DeviceAccess for Ula128AyKeypad<PluggableMultiJoyBusDevice<Ula128VidFrame>> {
    type JoystickDevice = PluggableMultiJoyBusDevice<Ula128VidFrame>;

    fn joystick_bus_device_mut(&mut self) -> Option<&mut Self::JoystickDevice> {
        Some(self.bus_device_mut().next_device_mut())
    }
    fn joystick_bus_device_ref(&self) -> Option<&Self::JoystickDevice> {
        Some(self.bus_device_ref().next_device_ref())
    }
    fn keypad128_mut(&mut self) -> Option<&mut SerialKeypad128> {
        Some(&mut self.bus_device_mut().ay_io.port_a.serial1)
    }
}
```

As you may see the joystick device for [Ula128] is positioned slightly deeper in the device chain, because we made its first device a sound processor. The [128 keypad][SerialKeypad] is connected to the PSG's IO port `A`. 128k ROM routines are using this port for connecting to both the keypad (AUX - serial port 1) and the [RS-232][Rs232Io] (SER - serial port 2). But in this example we won't be using the second serial port for the RS-232 connection. You can check [this example](https://github.com/royaltm/spectrusty/tree/master/examples/sdl2-zxspectrum) to see how to implement both.


### Hot-swap

Having dealt with the device access, now we can focus on the hot-swap function, as it will be slightly more challenging.

```rust
use std::io::{self, Read};

impl<C: Cpu, M> From<ZxSpectrumModel<C>> for ZxSpectrum<C, UlaPAL<M,
                                                PluggableMultiJoyBusDevice<
                                                    UlaVideoFrame>>>
    where M: ZxMemory + Default
{
    fn from(model: ZxSpectrumModel<C>) -> Self {
        let border = model.border_color();
        let mut spectrum = Self::new_with_rom();
        let mem_rd = model.read_ram();
        let _ = spectrum.ula.memory_mut()
                            .load_into_mem(M::PAGE_SIZE as u16.., mem_rd);
        let (cpu, joy, state) = model.into_cpu_joystick_and_state();
        spectrum.cpu = cpu;
        spectrum.state = state;
        spectrum.ula.set_border_color(border);
        **spectrum.ula.bus_device_mut() = joy.map(
                                        MultiJoystickBusDevice::new_with);
        spectrum
    }
}

impl<C: Cpu> From<ZxSpectrumModel<C>> for ZxSpectrum<C, Ula128AyKeypad<
                                                PluggableMultiJoyBusDevice<
                                                    Ula128VidFrame>>>
{
    fn from(model: ZxSpectrumModel<C>) -> Self {
        let border = model.border_color();
        let mut spectrum = Self::new_with_rom();
        let mem_rd = model.read_ram();
        let _ = spectrum.ula.memory_mut().load_into_mem(
                <Ula128 as MemoryAccess>::Memory::PAGE_SIZE as u16..,
                mem_rd);
        let (cpu, joy, state) = model.into_cpu_joystick_and_state();
        spectrum.cpu = cpu;
        spectrum.state = state;
        spectrum.ula.set_border_color(border);
        **spectrum.ula.bus_device_mut().next_device_mut() = joy.map(
                                        MultiJoystickBusDevice::new_with);
        // lock in 48k mode until reset
        spectrum.ula.set_ula128_mem_port_value(Ula128MemFlags::ROM_BANK
                                          |Ula128MemFlags::LOCK_MMU);
        spectrum
    }
}

```

We need to deal with the fact that the last bank of 128 memory can be swapped. Instead of copying slices of linear memory, we'll use a reader to copy the content of visible 3 pages of RAM. So the reader can have different implementation with regards to the model used.

Now for the implementation of model enum helpers:

```rust
impl<C: Cpu> ZxSpectrumModel<C> {
    fn into_cpu_joystick_and_state(
            self
        ) -> (C, Option<JoystickSelect>, EmulatorState)
    {
        match self {
            ZxSpectrumModel::Spectrum16(spec16) => (
                spec16.cpu,
                spec16.ula.into_bus_device().device.map(|d| d.joystick),
                spec16.state
            ),
            ZxSpectrumModel::Spectrum48(spec48) => (
                spec48.cpu,
                spec48.ula.into_bus_device().device.map(|d| d.joystick),
                spec48.state
            ),
            ZxSpectrumModel::Spectrum128(spec128) => (
                spec128.cpu,
                spec128.ula.into_bus_device()
                           .into_next_device().device.map(|d| d.joystick),
                spec128.state
            ),
        }        
    }
    // returns a dynamicly dispatched reader from RAM
    fn read_ram<'a>(&'a self) -> Box<dyn Read + 'a> {
        match self {
            ZxSpectrumModel::Spectrum16(spec16) =>
                Box::new(spec16.ula.memory_ref().ram_ref()
                                                .chain(io::repeat(!0))),
            ZxSpectrumModel::Spectrum48(spec48) =>
                Box::new(spec48.ula.memory_ref().ram_ref()),
            ZxSpectrumModel::Spectrum128(spec128) => {
                let mem = spec128.ula.memory_ref();
                // returns paged in RAM banks as a chained reader
                Box::new(mem.page_ref(1).unwrap()
                    .chain(mem.page_ref(2).unwrap())
                    .chain(mem.page_ref(3).unwrap()))
            }
        }
    }

    fn border_color(&self) -> BorderColor  {
        match self {
            ZxSpectrumModel::Spectrum16(spec16) =>
                                        spec16.ula.border_color(),
            ZxSpectrumModel::Spectrum48(spec48) =>
                                        spec48.ula.border_color(),
            ZxSpectrumModel::Spectrum128(spec128) =>
                                        spec128.ula.border_color(),
        }
    }
    // hot-swap hardware models
    fn change_model(self, request: ModelReq) -> Self {
        use ZxSpectrumModel::*;
        match (&self, request) {
            (Spectrum16(..), ModelReq::Spectrum16)|
            (Spectrum48(..), ModelReq::Spectrum48)|
            (Spectrum128(..), ModelReq::Spectrum128) => return self,
            _ => {}
        }
        match request {
            ModelReq::Spectrum16 => Spectrum16(self.into()),
            ModelReq::Spectrum48 => Spectrum48(self.into()),
            ModelReq::Spectrum128 => Spectrum128(self.into())
        }
    }
}
```

[![Iana](menu-step5.png)][sword-of-ianna]


### Keypad

There are a few things that need to be updated in the `run` method:

```rust
fn run<C: Cpu, U>(
        spectrum: &mut ZxSpectrum<C, U>,
        env: HostEnvironment,
    ) -> Result<Action>
    where U: UlaCommon + UlaAudioFrame<BandLim> + DeviceAccess + HostConfig,
          ZxSpectrum<C, U>: JoystickAccess
{
    //... ✂
    // ensure the Blep implementation is prepared for pulses
    spectrum.ula.ensure_audio_frame_time(
                            blep, audio.sample_rate(), U::CPU_HZ as f64);
    //... ✂
    let mut sync = ThreadSyncTimer::new(U::frame_duration_nanos());
    //... ✂
    'main: while is_running() {
        process_keyboard_events(
            |KeyEvent { key, pressed, shift_down, ctrl_down }| {
            if !update_joystick_from_key_event(key, pressed, FIRE_KEY,
                                    || spectrum.joystick_interface())
            {
                spectrum.update_keyboard(|keymap|
                    update_keymap(
                        keymap, key, pressed, shift_down, ctrl_down)
                );
                spectrum.update_keypad128_keys(|padmap|
                    update_keypad_keys(
                        padmap, key, pressed, shift_down || ctrl_down)
                );
            }
        });
        //... ✂
    }
    //... ✂
}
```

Its signature has changed and the more elaborate constraint is needed as we are now using a generic `U` type instead of `UlaPAL` struct. Next, we have to make room for the audio frames in the `blep` buffer. Different chipsets can have different number of cycles per frame, and so the duration of the single frame can change. Lastly, we need to add a way to pass keyboard events to the [128 keypad][SerialKeypad].

A few methods in `ZxSpectrum` implementation only changes slightly and we have a new method: `update_keypad128_keys`.
The signature of course reflects the changes:

```rust
impl<C: Cpu, U> ZxSpectrum<C, U>
    where U: UlaCommon,
          Self: JoystickAccess
{
    //... ✂
    fn update_keypad128_keys<F: FnOnce(KeypadKeys) -> KeypadKeys>(
            &mut self,
            update_keys: F
        )
        where U: DeviceAccess
    {
        if let Some(keypad) = self.ula.keypad128_mut() {
            let padmap = update_keys( keypad.get_key_state() );
            keypad.set_key_state(padmap);
        }
    }

    fn run_frame(&mut self) -> Result<(FTs, bool)> {
        //... ✂
        if self.nmi_request {
            if self.ula.nmi(&mut self.cpu) {
                self.nmi_request = false;
            }
        }
        if let Some(hard) = self.reset_request.take() {
            self.ula.reset(&mut self.cpu, hard);
        }
        self.ula.execute_next_frame(&mut self.cpu);
        //... ✂
    }

    fn reset(&mut self, hard: bool) {
        self.reset_request = Some(hard);
    }

    fn update_on_user_request(
            &mut self,
            input: InputRequest
        ) -> Result<Option<Action>>
    {
        match menu_id {
            //... ✂
            Spectrum128 => return Ok(Some(Action::ChangeModel(
                                                ModelReq::Spectrum128))),
            //... ✂
        }
    }
    //... ✂
}
```


### Stereo

Because we have an additional source of the sound, we need to update the `render_audio` method.

```rust
impl<C: Cpu, U> ZxSpectrum<C, U>
    //... ✂
{
    fn render_audio<B: Blep<SampleDelta=BlepDelta>>(
            &mut self, blep: &mut B
        ) -> usize
        where U: UlaAudioFrame<B>
    {
        self.ula.render_ay_audio_frame::<AyAmps<BlepDelta>>(blep,
                                                            [0, 1, 2]);
        // (1) add some amplitude steps to the BLEP that correspond to the EAR/MIC line changes
        if self.state.audible_tape {
            // render both EAR/MIC OUT channel
            self.ula.render_earmic_out_audio_frame::<
                EarMicAmps4<BlepDelta>
            >(blep, 2);
            // and the EAR IN channel
            self.ula.render_ear_in_audio_frame::<
                EarInAmps2<BlepDelta>
            >(blep, 2);
        }
        else {
            // render only EAR OUT channel
            self.ula.render_earmic_out_audio_frame::<
                EarOutAmps4<BlepDelta>
            >(blep, 2);
        }
        // (2) finalize the BLEP frame
        self.ula.end_audio_frame(blep)
    }
    //... ✂
}
```

But our implementation of [Blep] has only a single channel...and now we use 3 different channels?
Yes, so we need to change it too:

```rust
// the type of the Blep implementation
type BandLim = BlepStereo<BandLimited<BlepDelta>>;
```

and initialize it:

```rust
fn main() -> Result<()> {
    //... ✂
    let mut blep = BlepStereo::build(0.8)(BandLimited::<BlepDelta>::new(2));
    //... ✂
}
```

But where the 3rd channel comes from? It is thanks to [BlepStereo], which takes a 2 channel [Blep] and any channel exceeding the 2 will be directed to both channels simultanously. So if channel `0` is left, channel `1` is right then `2` is the center.

The last audio related part to update is `produce_audio_frame`:

```rust
fn produce_audio_frame<T: AudioSample + FromSample<BlepDelta>>(
        output_channels: usize,
        outbuf: &mut Vec<T>,
        blep: &mut BandLim,
    )
{
    // the BLEP buffer summing iterator of the channel 0
    let sample_iter = blep.sum_iter::<T>(0);
    // the number of samples that the iterator will generate
    let frame_sample_count = sample_iter.len();
    // ensure the size of the audio frame buffer is exactly as we need it
    outbuf.resize(frame_sample_count * output_channels, T::silence());
    // zip with the other channel
    let sample_iter = sample_iter.zip(blep.sum_iter::<T>(1));
    // render each sample
    for (chans, (lsmp, rsmp)) in outbuf.chunks_mut(output_channels)
                                       .zip(sample_iter) {
        // write each sample to each channel
        for (ch, sample) in chans.iter_mut().zip(&[lsmp, rsmp]) {
            *ch = *sample;
        }
    }
}
```

We assume the `output_channels` for the host audio is 2 or more.

And for the last the `main` function. We'll also change the default model to 128k.

```rust
fn main() -> Result<()> {
    //... ✂
    // build the hardware
    let mut spec128 = ZxSpectrum128k::<Z80NMOS, _>::new_with_rom();
    // if the user provided the file name
    if let Some(file_name) = tap_file_name {
        //... ✂
        spec128.state.tape.tap = Some(Tap::Reader(iter_pulse));
        // or instead we could just write:
        // spec128.tape.insert_as_reader(tap_file);
        spec128.state.audible_tape = true;
        spec128.state.flash_tape = true;
    }
    //... ✂
    // width and height of the rendered frame image area in pixels
    let (width, height) = <Ula128 as Video>::render_size_pixels(border);
    //... ✂
    // initialize audio
    let frame_duration_nanos =
                        <Ula128 as HostConfig>::frame_duration_nanos();
    //... ✂
    let mut spectrum = ZxSpectrumModel::Spectrum128(spec128);

    loop {
        //... ✂
        let req = match &mut spectrum {
            Spectrum16(spec16) => run(spec16, env)?,
            Spectrum48(spec48) => run(spec48, env)?,
            Spectrum128(spec128) => run(spec128, env)?
        };
        //... ✂
    }

    Ok(())
}
```

[![Finish](ingame-step5.png)][sword-of-ianna]


### Example

The [example][step5.rs] program using [minifb] and [cpal], covering the scope of this tutorial can be run with:

```sh
cargo run --bin step5 --release -- resources/iana128.tap
```

Press `[ENTER]` and enjoy the 128k game.


### Next

Back to [index][tutorial].

<script>var clicky_site_ids = clicky_site_ids || []; clicky_site_ids.push(101270192);</script>
<script async src="//static.getclicky.com/js"></script>

[SPECTRUSTY]: https://royaltm.github.io/spectrusty/
[tutorial]: https://royaltm.github.io/spectrusty-tutorial/
[step5.rs]: https://github.com/royaltm/spectrusty-tutorial/blob/master/src/bin/step5.rs
[minifb]: https://crates.io/crates/minifb
[cpal]: https://crates.io/crates/cpal
[sword-of-ianna]: https://github.com/fjpena/sword-of-ianna-zx
[Blep]: https://docs.rs/spectrusty/*/spectrusty/audio/trait.Blep.html
[BlepStereo]: https://docs.rs/spectrusty/*/spectrusty/audio/struct.BlepStereo.html
[BusDevice::Timestamp]: https://docs.rs/spectrusty/*/spectrusty/bus/trait.BusDevice.html#associatedtype.Timestamp
[Rs232Io]: https://docs.rs/spectrusty/0.1.0/spectrusty/bus/ay/serial128/struct.Rs232Io.html
[SerialKeypad]: https://docs.rs/spectrusty/*/spectrusty/peripherals/serial/struct.SerialKeypad.html
[Ula128]: https://docs.rs/spectrusty/*/spectrusty/chip/ula128/struct.Ula128.html
[Ula128VidFrame]: https://docs.rs/spectrusty/*/spectrusty/chip/ula128/struct.Ula128VidFrame.html
[VFNullDevice]: https://docs.rs/spectrusty/*/spectrusty/bus/type.VFNullDevice.html
[VideoFrame]: https://docs.rs/spectrusty/*/spectrusty/video/trait.VideoFrame.html