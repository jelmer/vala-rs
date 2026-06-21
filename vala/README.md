# vala

Safe Rust bindings to [libvala](https://wiki.gnome.org/Projects/Vala), the Vala
compiler library, built on top of [`vala-sys`](https://crates.io/crates/vala-sys).

The compiler class hierarchy (~150 types) is generated at build time from the
installed `.vapi`; high-value entry points have hand-written methods. See the
[repository README](https://github.com/jelmer/vala-rs) for details and examples.

## License

LGPL-2.1-or-later, matching libvala.
