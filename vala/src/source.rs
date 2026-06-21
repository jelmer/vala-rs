//! Curated methods on [`SourceFile`].

use std::ffi::CString;

use vala_sys as ffi;

use crate::context::SourceFileType;
use crate::object::{opt_string, RawWrapper};
use crate::{CodeContext, SourceFile};

impl SourceFile {
    /// Create a source file within `context`. `content` may be `None` to read
    /// the file from disk on demand.
    pub fn new(
        context: &CodeContext,
        file_type: SourceFileType,
        filename: &str,
        content: Option<&str>,
    ) -> Self {
        let cfilename = CString::new(filename).expect("filename contains NUL");
        let ccontent = content.map(|c| CString::new(c).expect("content contains NUL"));
        let content_ptr = ccontent.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());
        unsafe {
            Self::from_raw_full(ffi::vala_source_file_new(
                context.as_raw(),
                CodeContext::source_file_type(file_type),
                cfilename.as_ptr(),
                content_ptr,
                glib_sys::GFALSE,
            ))
            .expect("vala_source_file_new returned null")
        }
    }

    /// The file's path as known to libvala.
    pub fn filename(&self) -> Option<String> {
        unsafe { opt_string(ffi::vala_source_file_get_filename(self.as_raw())) }
    }
}

/// A position in a source file. Lines and columns are 1-based, matching
/// libvala (SCIP, by contrast, uses 0-based positions, so subtract one when
/// emitting SCIP ranges).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLocation {
    /// 1-based line number.
    pub line: i32,
    /// 1-based column number.
    pub column: i32,
}

impl SourceLocation {
    unsafe fn from_ffi(raw: ffi::ValaSourceLocation) -> Self {
        SourceLocation {
            line: raw.line,
            column: raw.column,
        }
    }
}

impl crate::SourceReference {
    /// The file this reference points into.
    pub fn file(&self) -> SourceFile {
        unsafe {
            SourceFile::from_raw_none(ffi::vala_source_reference_get_file(self.as_raw()))
                .expect("source reference had no file")
        }
    }

    /// The start position of the referenced range.
    pub fn begin(&self) -> SourceLocation {
        unsafe {
            let mut loc = std::mem::zeroed::<ffi::ValaSourceLocation>();
            ffi::vala_source_reference_get_begin(self.as_raw(), &mut loc);
            SourceLocation::from_ffi(loc)
        }
    }

    /// The end position of the referenced range.
    pub fn end(&self) -> SourceLocation {
        unsafe {
            let mut loc = std::mem::zeroed::<ffi::ValaSourceLocation>();
            ffi::vala_source_reference_get_end(self.as_raw(), &mut loc);
            SourceLocation::from_ffi(loc)
        }
    }
}
