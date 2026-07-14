//! Curated methods on [`SourceFile`].

use std::ffi::CString;

use vala_sys as ffi;

use crate::context::SourceFileType;
use crate::object::{opt_string, take_string, RawWrapper};
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

    /// The file's source text.
    ///
    /// For a file created without explicit content this is whatever libvala read
    /// from disk, so it is only populated once the file has been read (the
    /// parser does this). Returns `None` if libvala holds no content.
    pub fn content(&self) -> Option<String> {
        unsafe { opt_string(ffi::vala_source_file_get_content(self.as_raw())) }
    }

    /// A single line of the file's source text, without its newline. `lineno` is
    /// 1-based, matching [`SourceLocation::line`]. Returns `None` when the line
    /// is out of range.
    pub fn source_line(&self, lineno: i32) -> Option<String> {
        unsafe { take_string(ffi::vala_source_file_get_source_line(self.as_raw(), lineno)) }
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
    pub(crate) unsafe fn from_ffi(raw: ffi::ValaSourceLocation) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CodeContext;

    fn with_file<T>(src: &str, f: impl FnOnce(&SourceFile) -> T) -> T {
        let ctx = CodeContext::new();
        ctx.with_current(|ctx| {
            let file = SourceFile::new(ctx, SourceFileType::Source, "t.vala", Some(src));
            ctx.add_source_file(&file);
            f(&file)
        })
    }

    #[test]
    fn content_round_trips() {
        let src = "class Foo {\n}\n";
        assert_eq!(with_file(src, |f| f.content()), Some(src.to_string()));
    }

    #[test]
    fn source_line_is_one_based_and_drops_the_newline() {
        let src = "class Foo {\n    int x;\n}\n";
        let lines = with_file(src, |f| {
            (f.source_line(1), f.source_line(2), f.source_line(3))
        });
        assert_eq!(lines.0, Some("class Foo {".to_string()));
        assert_eq!(lines.1, Some("    int x;".to_string()));
        assert_eq!(lines.2, Some("}".to_string()));
    }

    #[test]
    fn source_line_out_of_range_is_none() {
        let src = "class Foo {\n}\n";
        assert_eq!(with_file(src, |f| f.source_line(99)), None);
    }
}
