//! Curated methods on [`Symbol`], the base of every named declaration.

use vala_sys as ffi;

use crate::object::{opt_string, RawWrapper};
use crate::{Comment, Symbol};

/// The declared visibility of a [`Symbol`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accessibility {
    /// `private` (the default for class members).
    Private,
    /// `internal`.
    Internal,
    /// `protected`.
    Protected,
    /// `public`.
    Public,
}

impl Symbol {
    /// The symbol's declared name, or `None` for anonymous symbols (such as the
    /// global root namespace).
    pub fn name(&self) -> Option<String> {
        unsafe { opt_string(ffi::vala_symbol_get_name(self.as_raw())) }
    }

    /// The declared visibility of this symbol.
    pub fn accessibility(&self) -> Accessibility {
        let access = unsafe { ffi::vala_symbol_get_access(self.as_raw()) };
        match access {
            ffi::VALA_SYMBOL_ACCESSIBILITY_PRIVATE => Accessibility::Private,
            ffi::VALA_SYMBOL_ACCESSIBILITY_INTERNAL => Accessibility::Internal,
            ffi::VALA_SYMBOL_ACCESSIBILITY_PROTECTED => Accessibility::Protected,
            ffi::VALA_SYMBOL_ACCESSIBILITY_PUBLIC => Accessibility::Public,
            other => panic!("unknown ValaSymbolAccessibility value {other}"),
        }
    }

    /// Whether this symbol comes from an external package (a `.vapi`) rather than
    /// from a source file being compiled.
    pub fn is_external_package(&self) -> bool {
        unsafe { ffi::vala_symbol_get_external_package(self.as_raw()) != glib_sys::GFALSE }
    }

    /// The documentation comment attached to this symbol, if any.
    pub fn comment(&self) -> Option<Comment> {
        unsafe { Comment::from_raw_none(ffi::vala_symbol_get_comment(self.as_raw())) }
    }
}

impl Comment {
    /// The raw text content of the comment, with comment markers stripped by
    /// libvala but otherwise unprocessed.
    pub fn content(&self) -> Option<String> {
        unsafe { opt_string(ffi::vala_comment_get_content(self.as_raw())) }
    }
}
