//! Low-level FFI bindings to [libvala](https://wiki.gnome.org/Projects/Vala),
//! the Vala compiler library.
//!
//! These bindings are generated at build time with `bindgen` against the
//! installed `libvala` headers, located via `system-deps`/`pkg-config`. They are
//! unsafe and map directly onto the C API; for a safe interface use the `vala`
//! crate.
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::all)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
