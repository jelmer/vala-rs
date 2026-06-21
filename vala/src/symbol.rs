//! Curated methods on [`Symbol`], the base of every named declaration.

use vala_sys as ffi;

use crate::object::{opt_string, RawWrapper};
use crate::Symbol;

impl Symbol {
    /// The symbol's declared name, or `None` for anonymous symbols (such as the
    /// global root namespace).
    pub fn name(&self) -> Option<String> {
        unsafe { opt_string(ffi::vala_symbol_get_name(self.as_raw())) }
    }
}
