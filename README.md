SPECTRUSTY Tutorial
===================

This is the repository of the [tutorial] for the [SPECTRUSTY] library.


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


Licensing
---------

This tutorial and example sources in this repository are free to use under the terms of the Blue Oak Model License 1.0.0.
See: [https://blueoakcouncil.org/license/1.0.0](https://blueoakcouncil.org/license/1.0.0).

Some TAP files in the [resources/] directory are ZX Spectrum games that are made available for free download and distribution. Check out [worldofspectrum.org](https://worldofspectrum.org/) for more information, programs and games.

[SPECTRUSTY]: https://royaltm.github.io/spectrusty/
[tutorial]: https://royaltm.github.io/spectrusty-tutorial/
[Rust]: https://www.rust-lang.org/
[Cargo]: https://crates.io/
[RUSTUP]: https://www.rust-lang.org/learn/get-started#installing-rust