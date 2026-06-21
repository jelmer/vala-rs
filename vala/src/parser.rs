//! Curated methods on [`Parser`].

use vala_sys as ffi;

use crate::object::RawWrapper;
use crate::{CodeContext, Parser, SourceFile};

impl Parser {
    /// Create a new Vala parser.
    pub fn new() -> Self {
        unsafe {
            Self::from_raw_full(ffi::vala_parser_new()).expect("vala_parser_new returned null")
        }
    }

    /// Parse all source files registered with `context` into its code tree.
    /// Diagnostics land in the context's [`Report`](crate::Report).
    pub fn parse(&self, context: &CodeContext) {
        unsafe { ffi::vala_parser_parse(self.as_raw(), context.as_raw()) }
    }

    /// Parse a single source file.
    pub fn parse_file(&self, file: &SourceFile) {
        unsafe { ffi::vala_parser_parse_file(self.as_raw(), file.as_raw()) }
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}
