# vala-sys

Raw FFI bindings to [libvala](https://wiki.gnome.org/Projects/Vala), the Vala
compiler library. Bindings are generated at build time with `bindgen` against
the `libvala-0.56` headers located via `pkg-config`.

For a safe interface, use the [`vala`](https://crates.io/crates/vala) crate.

## Requirements

- `libvala-0.56-dev`
- `libclang` (for bindgen)
- `pkg-config`

## License

LGPL-2.1-or-later, matching libvala.
