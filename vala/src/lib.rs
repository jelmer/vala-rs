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

pub mod collections;
pub mod object;
pub mod visitor;

pub use object::{Cast, Implements, IsA, RawWrapper};

// The generated hierarchy: one wrapper per libvala class, the `iface` markers,
// and the `IsA`/`Implements`/`AsRef`/`Deref` graph.
include!(concat!(env!("OUT_DIR"), "/generated.rs"));

mod containers;
mod context;
mod node;
mod parser;
mod report;
mod source;
mod symbol;

pub use collections::{List, ListIter};
pub use context::{Profile, SourceFileType};
pub use report::ReportLevel;
pub use source::SourceLocation;
pub use symbol::Accessibility;
pub use visitor::{walk, walk_source_file, Visitor, Walker};

/// The libvala API version this crate was built against (e.g. `"0.56"`).
///
/// Captured at build time from the libvala that vala-sys probed.
pub const API_VERSION: &str = env!("VALA_API_VERSION");

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
    fn build_version_matches_api_version() {
        let v = build_version();
        let prefix = format!("{API_VERSION}.");
        assert!(
            v.starts_with(&prefix),
            "build version {v} not in the {API_VERSION} series"
        );
    }
}
