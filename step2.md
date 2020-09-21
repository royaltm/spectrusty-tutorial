SPECTRUSTY Tutorial
===================

This is a part of the [tutorial] for the [SPECTRUSTY] library.

Step 2 - Buzz on
----------------

Wouldn't it be nice if our Spectrum could at least `BEEP` and make some key stroke noises?

Unfortunately synthesizing and playing audio stream isn't very straightforward. That's why this whole step is solely dedicated to make your emulator buzzing.

There are two challenges:

* First we need to produce some audio samples from emulator's frame pass data.
* Next we need to seamlessly direct those samples into some audio output device.

Let's focus on the first one as it's not trivial as it would seem. If perhaps, you already have some knowledge about audio signal processing, just skip the following 4 paragraphs, and get straight to the solution.

Bare ZX Spectrum produces sound by alternating voltage on its EAR and MIC output lines. Only two states are possible: high or low. If it alternates this state fast enough (but not too fast) we could hear some tone. But the shape of such a sound wave is not the natural sinusoid but is rather square.

When SPECTRUSTY runs, it records the changes to the EAR and MIC states as T-state timestamps. The frequency of its CPU clock is around 3.5 MHz. That is around `3_500_000 / 44100 ~ 79` times more than the typical audio sample frequency. If you have never heard about [Nyquist frequency] or you have no idea about sound harmonics you might probably wonder what could possibly go wrong if we just assign the high output state to some amplitude A1 and the low output state to another amplitude A2 and just divide the T-states so the timestamps would match the sample time.

Well... you definitely could, but I don't recommend it. I must admit, I did something like that myself just to hear the difference. Even though my hearing abilities are below average (I barely hear anything above 9kHZ) to my ears even the simple BEEP in such a naive implementation was just muddy and unpleasant. But when I loaded something more complicated, like the TRAP DOOR game music, what I was hearing was just a noisy trash.

The perfect square wave can be represented as an infinite sum of sinusoidal waves. The problem (definitely oversimplyfied here) is that the frequency of those waves tends to infinity. The digitization of sound is limited by the finite sample frequency and the maximum frequency that can be sampled is called the [Nyquist frequency], which is exactly half of the sampling frequency. Square waves that sound "clear" should be constructed from a limited number of sinusoidal waves, but the computation of such wave could be costly.

Fortunately some clever people have noticed that the pattern of the square steps are independent from the sampling frequency or the frequency of these steps are bound to the sample frames. Armed with this knowledge it should be quite easy to pre-calculate this pattern and apply it to the steps with a little bit of scaling. This technique is sometimes called [Hard sync]. We implement this as a so called Bandwidth-Limited Pulse Buffer (`BLEP`). If you want to know more about the technique, please indulge yourself with an excellent [sync tutorial].

### Synthesizing

In SPECTRUSTY the process of generating audio samples follows this steps, each frame:

1. add some amplitude steps to the `BLEP` that correspond to the `EAR/MIC` line changes (or other sources),
2. finalize the `BLEP` frame,
3. render the `BLEP` frame as audio samples,
4. prepare the `BLEP` for the next frame.

For the sake of flexibility the [Blep] trait has been introduced, such that many different `BLEP` implementations can be used with SPECTRUSTY. In the [spectrusty-audio] module one such implementation exists out of the box.

Let's now adapt your emulator:

First we need to enable audio feature to access the [audio::synth] module:

```toml
[dependencies]
spectrusty = { version = "0.1", features = ["audio"] }
```

```rust
// Yassss... more imports coming your way...
use spectrusty::audio::{
    Blep, AudioFrame, EarMicOutAudioFrame,
    AudioSample, FromSample, EarMicAmps4,
    synth::BandLimited
};

// the type of the Blep implementation amplitude delta
type BlepDelta = f32; // i16 i32

fn main() -> Result<()> {
    //... ✂
    //... later in main

    // first let's get an instance of some audio device implementation
    let audio = create_audio();
    // second the Bandwidth-Limited Pulse Buffer implementation with a single channel
    let mut blep = BandLimited::<BlepDelta>::new(1);
    // we need a sample frequency of the audio output
    let sample_rate: u32 = audio.sample_rate().into();
    // ensure the Blep implementation is prepared for pulses
    spectrum.ula.ensure_audio_frame_time(&mut blep, sample_rate);
    //... ✂
}

fn run<C: Cpu, M: ZxMemory>(
        spectrum: &mut ZxSpectrum<C, M>,
        env: HostEnvironment,
    ) -> Result<Option<ModelReq>>
{
    let HostEnvironment { audio, blep, border, ... } = env;
    let (width, height) = <Ula<M> as Video>::render_size_pixels(border);
    //... ✂
    while is_running() {
        spectrum.update_keyboard( update_keys );

        spectrum.run_frame();

        let (video_buffer, pitch) = acquire_video_buffer(width, height);
        spectrum.render_video::<SpectrumPalRGB24>(video_buffer, pitch, border);
        // (1) and (2)
        spectrum.render_audio(&mut blep);

        update_display();

        // (3) render the BLEP frame as audio samples
        produce_audio_frame(audio.channels(), audio.frame_buffer(), &mut blep);
        // somehow play the rendered buffer
        audio.play_frame()?;

        // (4) prepare the BLEP for the next frame.
        blep.next_frame();

        //... ✂
    }
}
```

Let's look closer at steps 1 and 2:

```rust
impl<C: Cpu, M: ZxMemory> ZxSpectrum<C, M> {
    // a generic function that will accept any Blep implementation
    fn render_audio<B: Blep<SampleDelta=BlepDelta>>(
            &mut self,
            blep: &mut B
        ) -> usize
    {
        // (1) add some amplitude steps to the BLEP that correspond to the EAR/MIC line changes
        self.ula.render_earmic_out_audio_frame::<EarMicAmps4<BlepDelta>>(blep, 0);
        // (2) finalize the BLEP frame
        self.ula.end_audio_frame(blep)
    }
}
```

[EarMicAmps4] is an implementation of [AmpLevels] trait that is responsible for translating the linear digital levels to the sample amplitudes which should scale exponentialy in case there are more than 2 digital levels.

We can interpret EAR and MIC lines as 4 digital levels:

```
EAR  MIC  level
 0    0     0
 0    1     1
 1    0     2
 1    1     3
```

And that is exactly what [EarMicOutAudioFrame::render_earmic_out_audio_frame] method does. If we only wanted to hear the EAR changes and ignore the MIC changes we can replace [EarMicAmps4] with [EarOutAmps4].

Now, into the part 3. In this example we assume `audio` is capable of providing some audio buffer as a [`Vec<T>`]
of audio samples `T`:

```rust
fn produce_audio_frame<T: AudioSample + FromSample<BlepDelta>>(
        output_channels: usize,
        outbuf: &mut Vec<T>,
        blep: &mut BandLim
    )
{
    // the BLEP buffer summing iterator of the channel 0
    let sample_iter = blep.sum_iter::<T>(0);
    // the number of samples that the iterator will generate
    let frame_sample_count = sample_iter.len();
    // ensure the size of the audio frame buffer is exactly as we need it
    outbuf.resize(frame_sample_count * output_channels, T::silence());
    // render each sample
    for (chans, sample) in outbuf.chunks_mut(output_channels).zip(sample_iter) {
        // write sample to each channel
        for p in chans.iter_mut() {
            *p = sample;
        }
    }
}

```

The generic audio sample type `T` may be different from the `BlepDelta` as long as it can be converted from `BlepDelta` with the [FromSample] trait.

Part 4 is pretty obvious.

That basically covers the topic of creating audio samples.


### Streaming

How to play audio seamlessly is another cookie to crunch.

Most of the audio frameworks usually spawn a dedicated thread which periodically calls your function in order to fill the output buffer just in time to be played. To our emulator's loop this is a completely asynchronous process.

To overcome this hurdle, Spectrusty introduces the [Carousel]. It consists of an interconnected pair of an audio producer and a consumer. The consumer lives in the audio thread. The producer is available in our loop. The producer can send the audio frame buffer of the arbitrary size to the consumer and consumer when called by the audio framework fills the audio output buffer with our audio frame data and then sends it back to be filled again. We can control how many buffers are in the circulation thus influencing the latency and stability of the sound playback stream.

To simplify this task even more, there are some platform specific features that can be enabled:

```toml
[dependencies]
spectrusty = { version = "0.1", features = ["audio", "cpal"] } # or "sdl2"
```

providing the complete carousel solutions for [cpal] audio layer or [SDL2].

I won't wander into details on how to implement carousel for any particular framework, because a quick look to the [spectrusty-audio::host] should reveal all its secrets.


### Example

The [example][step2.rs] program using [minifb] and [cpal], covering the scope of this tutorial can be run with:

```sh
cargo run --bin step2 --release
```

[SPECTRUSTY]: https://royaltm.github.io/spectrusty/
[tutorial]: https://royaltm.github.io/spectrusty-tutorial/
[step2.rs]: https://github.com/royaltm/spectrusty-tutorial/blob/master/src/bin/step2.rs
[minifb]: https://crates.io/crates/minifb
[cpal]: https://crates.io/crates/cpal
[Nyquist frequency]: https://en.wikipedia.org/wiki/Nyquist_frequency
[Hard sync]: https://www.cs.cmu.edu/~eli/papers/icmc01-hardsync.pdf
[sync tutorial]: http://www.slack.net/~ant/bl-synth
[SDL2]: https://www.libsdl.org/index.php
[Blep]: https://docs.rs/spectrusty/*/spectrusty/audio/trait.Blep.html
[spectrusty-audio]: https://docs.rs/spectrusty-audio/
[EarMicOutAudioFrame::render_earmic_out_audio_frame]: https://docs.rs/spectrusty-core/*/spectrusty_core/audio/trait.EarMicOutAudioFrame.html#tymethod.render_earmic_out_audio_frame
[EarMicAmps4]: https://docs.rs/spectrusty/*/spectrusty/audio/struct.EarMicAmps4.html
[EarOutAmps4]: https://docs.rs/spectrusty/*/spectrusty/audio/struct.EarOutAmps4.html
[AmpLevels]: https://docs.rs/spectrusty/*/spectrusty/audio/trait.AmpLevels.html
[`Vec<T>`]: https://doc.rust-lang.org/std/vec/struct.Vec.html
[FromSample]: https://docs.rs/spectrusty/*/spectrusty/audio/trait.FromSample.html
[Carousel]: https://docs.rs/spectrusty/*/spectrusty/audio/carousel/index.html
[audio::synth]: https://docs.rs/spectrusty/0.1.0/spectrusty/audio/synth/index.html
[spectrusty-audio::host]: https://docs.rs/spectrusty-audio/0.1.0/spectrusty_audio/host/index.html