SPECTRUSTY Tutorial
===================

[Sinclair ZX Spectrum] is an 8-bit computer that consists of a Central Processing Unit, a clock, some memory, and a custom Sinclair Uncommitted Logic Array (ULA) chip. The chip is responsible for I/O with peripherals such as the keyboard and for generating audio and video output. ULA also generates interrupts. When it needs to read pixel data from video memory, it can pause the clock when the CPU is accessing the same part of RAM (memory contention). Moreover, the raw address, data, and control lines of Z80 CPU (in SPECTRUSTY called "the BUS") are being exposed to allow external devices to be attached.

[![S P E C T R U S T Y][SPECTRUSTY img]][SPECTRUSTY]

[SPECTRUSTY] is a set of components designed in a way that mimics Spectrum's hardware parts and peripherals.
The components are [structs] and [enums] that are often having [generic types] in their definitions. Every such a generic stand-in needs to be substituted by the concrete sub-component. Functions of components are realized via Rust's [trait system].

In SPECTRUSTY, the base part of the emulated computer is its control chip (e.g. [Ula] or [Ula128]).
Here is the list of the most important traits:

- [ControlUnit] to execute Z80 code via [Cpu] and access peripheral devices;
- [FrameState] to access the clock counters;
- [MemoryAccess] and [ZxMemory] to modify or read the content of the emulator's memory;
- [KeyboardInterface] to change the state of the Spectrum's keyboard;
- [MicOut] to read signal from MIC OUT lines;
- [EarIn] to feed the EAR IN lines with external input;
- [EarMicOutAudioFrame] and [EarInAudioFrame] to help generating sound from EAR IN/OUT and MIC OUT lines;
- [Video] and [VideoFrame] for rendering video output;

Other notable traits for peripherals, such as printers, joysticks, serial ports, sound chipsets, microdrives e.t.c. are:

- [BusDevice] implemented by devices attached to the I/O BUS;
- [MemoryExtension] implemented by devices that page in external ROM memory;


Prerequisites
-------------

You'll need the [Rust] language compiler with the [Cargo] package manager.

Both are best served with a [RUSTUP] utility. If you don't like the language scoped version managers, some Linux distributions and 3rd party packaging systems also provide appropriate Rust and Cargo packages.

To check if you can continue, you should be able to run the `cargo` utility by creating a new repository for your emulator program:

```rust
cargo new my-spectrum-emu
```

If you see a message:

```
Created binary (application) `my-spectrum-emu` package
```

then you are good to go.


Tutorial steps
--------------

* [Step 1 - Baby steps](step1.md)
* [Step 2 - Buzz on](step2.md)
* [Step 3 - R Tape loading error](step3.md)
* Step 4 - soon
* Step 5 - soon

[![ZX Spectrum keyboard layout][keyboard]][keyboard]


Licensing
---------

This tutorial and example sources in this repository are free to use under the terms of the Blue Oak Model License 1.0.0.
See: [https://blueoakcouncil.org/license/1.0.0](https://blueoakcouncil.org/license/1.0.0).

<script>var clicky_site_ids = clicky_site_ids || []; clicky_site_ids.push(101270192);</script>
<script async src="//static.getclicky.com/js"></script>

[Sinclair ZX Spectrum]: https://en.wikipedia.org/wiki/ZX_Spectrum
[SPECTRUSTY img]: spectrusty.png
[keyboard]: keyboard48.jpg
[SPECTRUSTY]: https://royaltm.github.io/spectrusty/
[tutorial]: https://royaltm.github.io/spectrusty-tutorial/
[Rust]: https://www.rust-lang.org/
[Cargo]: https://crates.io/
[RUSTUP]: https://www.rust-lang.org/learn/get-started#installing-rust
[trait system]: https://doc.rust-lang.org/book/ch10-02-traits.html
[structs]: https://doc.rust-lang.org/book/ch05-01-defining-structs.html
[enums]: https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html
[generic types]: https://doc.rust-lang.org/book/ch10-00-generics.html
[Ula]: https://docs.rs/spectrusty/*/spectrusty/chip/ula/struct.Ula.html
[Cpu]: https://docs.rs/z80emu/*/z80emu/trait.Cpu.html
[BusDevice]: https://docs.rs/spectrusty/*/spectrusty/bus/trait.BusDevice.html
[ControlUnit]: https://docs.rs/spectrusty/*/spectrusty/chip/trait.ControlUnit.html
[EarIn]: https://docs.rs/spectrusty/*/spectrusty/chip/trait.EarIn.html
[EarMicOutAudioFrame]: https://docs.rs/spectrusty/*/spectrusty/audio/trait.EarMicOutAudioFrame.html
[EarInAudioFrame]: https://docs.rs/spectrusty/*/spectrusty/audio/trait.EarInAudioFrame.html
[FrameState]: https://docs.rs/spectrusty/*/spectrusty/chip/trait.FrameState.html
[KeyboardInterface]: https://docs.rs/spectrusty/*/spectrusty/peripherals/trait.KeyboardInterface.html
[MemoryAccess]: https://docs.rs/spectrusty/*/spectrusty/chip/trait.MemoryAccess.html
[MemoryExtension]: https://docs.rs/spectrusty/*/spectrusty/memory/trait.MemoryExtension.html
[MicOut]: https://docs.rs/spectrusty/*/spectrusty/chip/trait.MicOut.html
[Ula128]: https://docs.rs/spectrusty/*/spectrusty/chip/ula128/struct.Ula128.html
[Ula]: https://docs.rs/spectrusty/*/spectrusty/chip/ula/struct.Ula.html
[Video]: https://docs.rs/spectrusty/*/spectrusty/video/trait.Video.html
[VideoFrame]: https://docs.rs/spectrusty/*/spectrusty/video/trait.VideoFrame.html
[ZxMemory]: https://docs.rs/spectrusty/*/spectrusty/memory/trait.ZxMemory.html