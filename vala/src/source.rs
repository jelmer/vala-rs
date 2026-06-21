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
