# vala-sys

Raw FFI bindings to [libvala](https://wiki.gnome.org/Projects/Vala), the Vala
compiler library. Bindings are generated at build time with `bindgen` against
the installed `libvala` headers, located via `pkg-config`.

For a safe interface, use the [`vala`](https://crates.io/crates/vala) crate.

libvala bakes its API series into the pkg-config name (`libvala-0.56`,
`libvala-0.58`, ...). The build picks the highest series enabled through a
`v0_NN` cargo feature; `v0_56` is the default. Enable a newer feature (e.g.
`v0_58`) to build against a newer series.

## Requirements

- `libvala-<series>-dev` (e.g. `libvala-0.56-dev`)
- `libclang` (for bindgen)
- `pkg-config`

## License

LGPL-2.1-or-later, matching libvala.
