//! Safe Rust bindings to [libvala](https://wiki.gnome.org/Projects/Vala), the
//! Vala compiler library.
//!
//! The crate is built in two layers:
//!
//! * a generated wrapper hierarchy mirroring libvala's ~170 compiler classes
//!   (parsed from the installed `.vapi` at build time), giving every class a
//!   ref-counted handle with [`Cast`]-style up/downcasting; and
//! * hand-written, curated methods on the high-value entry points
//!   ([`CodeContext`], [`SourceFile`], [`Parser`], [`Report`]).
//!
//! libvala's classes are GLib *fundamental* reference-counted types (not
//! `GObject`s); cloning a wrapper bumps the refcount and dropping releases it.
//!
//! ```no_run
//! use vala::{CodeContext, Cast};
//!
//! let ctx = CodeContext::new();
//! ctx.add_source_filename("hello.vala", true);
//! ```
#![warn(missing_docs)]

pub mod object;

pub use object::{Cast, Implements, IsA, RawWrapper};

// The generated hierarchy: one wrapper per libvala class, the `iface` markers,
// and the `IsA`/`Implements`/`AsRef`/`Deref` graph.
include!(concat!(env!("OUT_DIR"), "/generated.rs"));

mod context;
mod parser;
mod report;
mod source;
mod symbol;

pub use context::SourceFileType;

pub use report::ReportLevel;

/// The libvala API version this crate was built against (e.g. `"0.56"`).
pub const API_VERSION: &str = "0.56";

// Codegen must produce the broad hierarchy; a low count means the vapi parser
// silently dropped declarations.
const _: () = assert!(GENERATED_TYPE_COUNT >= 100);

/// Return the runtime libvala build version string, e.g. `"0.56.19"`.
pub fn build_version() -> String {
    unsafe {
        let ptr = vala_sys::vala_get_build_version();
        object::opt_string(ptr).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_version_is_0_56_series() {
        let v = build_version();
        assert!(v.starts_with("0.56."), "unexpected version: {v}");
    }
}
