//! Curated methods on [`Report`], libvala's diagnostics sink.

use vala_sys as ffi;

use crate::object::RawWrapper;
use crate::Report;

/// Severity of a libvala diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportLevel {
    /// Informational note.
    Note,
    /// Deprecation notice.
    Deprecated,
    /// Experimental-feature notice.
    Experimental,
    /// Warning.
    Warning,
    /// Error.
    Error,
}

impl Report {
    /// Number of errors reported so far.
    pub fn errors(&self) -> i32 {
        unsafe { ffi::vala_report_get_errors(self.as_raw()) }
    }

    /// Number of warnings reported so far.
    pub fn warnings(&self) -> i32 {
        unsafe { ffi::vala_report_get_warnings(self.as_raw()) }
    }

    /// Whether any errors have been reported.
    pub fn has_errors(&self) -> bool {
        self.errors() > 0
    }
}
