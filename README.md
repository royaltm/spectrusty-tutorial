SPECTRUSTY Tutorial
===================

Example programs from the [tutorial] for building your own emulators with the [SPECTRUSTY] library.

This supplementary repository contains Rust crate with sources built from each step of the tutorial.

[![ZX Spectrum keyboard layout][keyboard]][keyboard]

Prerequisites
-------------

The [Rust] language compiler with the [Cargo] package manager.

Both are best served with a [RUSTUP] utility. If you don't like the language scoped version managers, some Linux distributions and 3rd party packaging systems also provide appropriate Rust and Cargo packages.

To check if you can continue, you should be able to run the `cargo` utility by creating a new repository for your emulator program:

```
cargo new my-spectrum-emu
```

If you see a message:

```
Created binary (application) `my-spectrum-emu` package
```

then you are good to [go][tutorial].


Compilation
-----------

To compile all example steps, type:

```
cargo build --bins --release
```

Depending on your operating system, you may need additional dependencies installed.

Please refer to the [minifb] and [cpal] crates documentation regarding those requirements.

On macOS and MS Windows, except [Rust], you shouldn't need anything else.

On Linux, some development packages are required.
E.g., on a fresh Ubuntu, I've managed to compile it after:

```
# some essential development libraries
sudo apt install gcc g++ libc6-dev libssl-dev

# ALSA for cpal
sudo apt install libasound2-dev

# Wayland for minifb
sudo apt install libxkbcommon-dev libwayland-cursor0 libwayland-dev
```

Unfortunately, menus are not supported on Linux.


Licensing
---------

This tutorial and example sources in this repository are free to use under the terms of the Blue Oak Model License 1.0.0.
See: [https://blueoakcouncil.org/license/1.0.0](https://blueoakcouncil.org/license/1.0.0).

Some TAP files in the [resources](resources/) directory are ZX Spectrum games that are made available for free download and distribution. Check out [worldofspectrum.org](https://worldofspectrum.org/) for more information, programs and games.

[SPECTRUSTY]: https://royaltm.github.io/spectrusty/
[tutorial]: https://royaltm.github.io/spectrusty-tutorial/
[keyboard]: resources/keyboard48.jpg
[Rust]: https://www.rust-lang.org/
[Cargo]: https://crates.io/
[RUSTUP]: https://www.rust-lang.org/learn/get-started#installing-rust
[cpal]: https://github.com/rustaudio/cpal
[minifb]: https://github.com/emoon/rust_minifb#build-instructions