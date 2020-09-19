SPECTRUSTY Tutorial
===================

Sinclair ZX Spectrum is a simple 8-bit computer that consist of Central Processing Unit, a clock, some memory and a custom Sinclair Uncommitted Logic Array (ULA) chip which controls computer's I/O such as keyboard, audio and is responsible for generating video output. ULA also generates interrupts and affects the CPU access to the lower part of RAM and can pause CPU's clock when it needs to read pixel data from video memory and detects that CPU is accessing the same part of RAM. Moreover a raw Z80 data, address and control lines (in SPECTRUSTY known as "the BUS") is being exposed to allow attaching devices that communicate via IN/OUT CPU instructions and may page-in an external ROM memory.

SPECTRUSTY is a set of components and interfaces designed in a way that mimicks Spectrum's hardware parts and peripherials.

Components of SPECTRUSTY interact with each other using Rust's trait system. When more than one component of a kind can be provided, its API exposes generic type parameters to allow for more flexibility, but at the same time taking advantage of monomorphization to generate optimized code.

SPECTRUSTY's API defines important traits, from emulator builder perspective, such as:

- [ControlUnit] to execute Spectrum's programs and access peripheral devices,
- [FrameState] to access the clock counters,
- [MemoryAccess] and [ZxMemory] to be able to modify or read content of the emulated memory,
- [KeyboardInterface] to be able to change state of the Spectrum's keyboard,
- [MicOut] to make use of generated MIC OUT lines signals,
- [EarIn] to feed the EAR IN lines with external signals,
- [EarMicAudioFrame] to help generating sound from EAR IN/OUT and MIC OUT line singals,
- [Video] and [VideoFrame] to allow rendering of output video,

that are implemented by the core chipset emulators, e.g. [Ula] and [Ula128].

Other notable traits, are:

- [BusDevice] to access devices attached to the I/O BUS and make use of their side effects,
- [MemoryExtension] to provide devices that pages external ROM memory depending on the address of the executed instruction;

which are implemented by external device emulators, such as printers, joysticks, serial ports, sound
chipsets, microdrives e.t.c.

Prerequisites
-------------

To make most of this tutorial you'll need the [Rust] language compiler and the [Cargo] package manager.

Both are best served with a [RUSTUP] utility, but some linuxes and 3rd party packaging systems also provide appropriate Rust + Cargo packages, if you don't like such language scoped version managers.

To check if you can continue, you should be able to run the `cargo` utility, by creating a new repository for your emulator program:

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

* [Step 1 - Baby steps][TUTORIAL_STEP1]
* [Step 2 - Buzz on][TUTORIAL_STEP2]
* [Step 3 - R Tape loading error][TUTORIAL_STEP3]
* Step 4
* Step 5


[Rust]: https://www.rust-lang.org/
[Cargo]: https://crates.io/
[RUSTUP]: https://www.rust-lang.org/learn/get-started#installing-rust