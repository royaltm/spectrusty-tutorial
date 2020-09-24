SPECTRUSTY Tutorial
===================

Example programs from the [tutorial] for building your own emulators with the [SPECTRUSTY] library.

This supplementary repository contains Rust crate with sources built from each of the tutorial steps.


Prerequisites
-------------

The [Rust] language compiler with the [Cargo] package manager.

Both are best served with a [RUSTUP] utility. If you don't like the language scoped version managers, some Linux distributions and 3rd party packaging systems also provide appropriate Rust and Cargo packages.

To check if you can continue, you should be able to run the `cargo` utility by creating a new repository for your emulator program:

```rust
cargo new my-spectrum-emu
```

If you see a message:

```
Created binary (application) `my-spectrum-emu` package
```

then you are good to [go][tutorial].


Licensing
---------

This tutorial and example sources in this repository are free to use under the terms of the Blue Oak Model License 1.0.0.
See: [https://blueoakcouncil.org/license/1.0.0](https://blueoakcouncil.org/license/1.0.0).

Some TAP files in the [resources](resources/) directory are ZX Spectrum games that are made available for free download and distribution. Check out [worldofspectrum.org](https://worldofspectrum.org/) for more information, programs and games.

[SPECTRUSTY]: https://royaltm.github.io/spectrusty/
[tutorial]: https://royaltm.github.io/spectrusty-tutorial/
[Rust]: https://www.rust-lang.org/
[Cargo]: https://crates.io/
[RUSTUP]: https://www.rust-lang.org/learn/get-started#installing-rust