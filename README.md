# vala-rs

Rust bindings to [libvala](https://wiki.gnome.org/Projects/Vala), the Vala
compiler library (the `libvala-dev` package on Debian/Ubuntu).

Two crates:

- **`vala-sys`** -- raw FFI bindings, generated at build time with `bindgen`
  against the installed `libvala` headers, located via `pkg-config`.
- **`vala`** -- a safe layer. The class hierarchy (~150 compiler types) is
  generated at build time from the installed `.vapi`; high-value entry points
  (`CodeContext`, `SourceFile`, `Parser`, `Report`, `Symbol`) have hand-written
  methods.

libvala's compiler classes are GLib *fundamental* reference-counted types, not
`GObject`s. Each wrapper is a ref-counted handle: cloning bumps the refcount and
dropping releases it. Up- and downcasting follow the libvala inheritance graph
via the `Cast` trait, with downcasts checked at runtime against the GType.

## Requirements

- `libvala-<series>-dev`, e.g. `libvala-0.56-dev` (provides the headers,
  pkg-config file and vapi)
- `libclang` (for `bindgen` at build time)
- `pkg-config`

```sh
sudo apt install libvala-0.56-dev libclang-dev pkg-config
```

## Example

```rust
use vala::{CodeContext, Parser, SourceFile, SourceFileType};

let ctx = CodeContext::new();
ctx.with_current(|ctx| {
    let file = SourceFile::new(ctx, SourceFileType::Source, "hello.vala",
        Some("void main () { print (\"hi\"); }"));
    ctx.add_source_file(&file);

    let parser = Parser::new();
    parser.parse(ctx);

    assert_eq!(ctx.report().errors(), 0);
});
```

See `vala/examples/parse.rs` for a runnable version.

## Versioning

libvala bakes its API series into the pkg-config name (`libvala-0.56`,
`libvala-0.58`, ...). The build selects the highest series enabled through a
`v0_NN` feature on `vala-sys` (default `v0_56`); the series built against is
exposed as `vala::API_VERSION`, and the exact runtime library version via
`vala::build_version()`.

## License

LGPL-2.1-or-later, matching libvala.
