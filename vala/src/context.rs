//! Curated methods on [`CodeContext`], the entry point for driving the compiler.

use std::ffi::CString;

use vala_sys as ffi;

use crate::object::RawWrapper;
use crate::{CodeContext, Report, SourceFile};

/// The kind of a source file added to a [`CodeContext`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFileType {
    /// Unspecified.
    None,
    /// A `.vala`/`.gs` source file to compile.
    Source,
    /// A `.vapi` package file providing declarations only.
    Package,
    /// A fast (declaration-only) vapi.
    Fast,
}

impl SourceFileType {
    fn to_ffi(self) -> ffi::ValaSourceFileType {
        match self {
            SourceFileType::None => ffi::VALA_SOURCE_FILE_TYPE_NONE,
            SourceFileType::Source => ffi::VALA_SOURCE_FILE_TYPE_SOURCE,
            SourceFileType::Package => ffi::VALA_SOURCE_FILE_TYPE_PACKAGE,
            SourceFileType::Fast => ffi::VALA_SOURCE_FILE_TYPE_FAST,
        }
    }
}

/// The target runtime profile, which determines the implicit standard packages
/// (and hence which base types like `int`/`string` resolve).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// The default GObject profile (`glib-2.0`, `gobject-2.0`).
    GObject,
    /// The minimal POSIX/libc profile.
    Posix,
}

impl Profile {
    fn to_ffi(self) -> ffi::ValaProfile {
        match self {
            Profile::GObject => ffi::VALA_PROFILE_GOBJECT,
            Profile::Posix => ffi::VALA_PROFILE_POSIX,
        }
    }
}

impl CodeContext {
    /// Create a fresh, empty compilation context.
    pub fn new() -> Self {
        unsafe {
            Self::from_raw_full(ffi::vala_code_context_new())
                .expect("vala_code_context_new returned null")
        }
    }

    /// Push this context onto libvala's global context stack. Many libvala
    /// operations read the current context; wrap work in [`push`]/[`pop`] or use
    /// [`with_current`](CodeContext::with_current).
    ///
    /// [`push`]: CodeContext::push
    /// [`pop`]: CodeContext::pop
    pub fn push(&self) {
        unsafe { ffi::vala_code_context_push(self.as_raw()) }
    }

    /// Pop the top context off libvala's global stack.
    pub fn pop() {
        unsafe { ffi::vala_code_context_pop() }
    }

    /// Run `f` with this context pushed as the current global context, popping it
    /// afterwards even on panic.
    pub fn with_current<R>(&self, f: impl FnOnce(&Self) -> R) -> R {
        self.push();
        let guard = PopGuard;
        let r = f(self);
        drop(guard);
        r
    }

    /// Add a source file by name. `is_source` marks it as compilable Vala (as
    /// opposed to a package). Returns whether libvala accepted the file.
    pub fn add_source_filename(&self, filename: &str, is_source: bool) -> bool {
        let c = CString::new(filename).expect("filename contains NUL");
        unsafe {
            ffi::vala_code_context_add_source_filename(
                self.as_raw(),
                c.as_ptr(),
                is_source as ffi::gboolean,
                glib_sys::GFALSE,
            ) != glib_sys::GFALSE
        }
    }

    /// Add a source file object created with [`SourceFile::new`].
    pub fn add_source_file(&self, file: &SourceFile) {
        unsafe { ffi::vala_code_context_add_source_file(self.as_raw(), file.as_raw()) }
    }

    /// Add an external package dependency (resolved via the vapi search path).
    /// Returns whether the package's vapi was found.
    pub fn add_external_package(&self, pkg: &str) -> bool {
        let c = CString::new(pkg).expect("package name contains NUL");
        unsafe {
            ffi::vala_code_context_add_external_package(self.as_raw(), c.as_ptr())
                != glib_sys::GFALSE
        }
    }

    /// Set the target profile and optionally include its standard packages.
    /// This must be called (typically before adding sources) for semantic
    /// analysis to resolve built-in types such as `int` and `string`.
    pub fn set_target_profile(&self, profile: Profile, include_stdpkg: bool) {
        unsafe {
            ffi::vala_code_context_set_target_profile(
                self.as_raw(),
                profile.to_ffi(),
                include_stdpkg as ffi::gboolean,
            )
        }
    }

    /// Add a preprocessor define.
    pub fn add_define(&self, define: &str) {
        let c = CString::new(define).expect("define contains NUL");
        unsafe { ffi::vala_code_context_add_define(self.as_raw(), c.as_ptr()) }
    }

    /// Run semantic analysis over the added sources, accumulating diagnostics in
    /// the [`Report`]. Must be called with this context current (see
    /// [`with_current`](CodeContext::with_current)).
    pub fn check(&self) {
        unsafe { ffi::vala_code_context_check(self.as_raw()) }
    }

    /// The diagnostic report collecting errors and warnings.
    pub fn report(&self) -> Report {
        unsafe {
            Report::from_raw_none(ffi::vala_code_context_get_report(self.as_raw()))
                .expect("context report was null")
        }
    }

    /// The source files registered with this context (including any package
    /// vapis pulled in as dependencies).
    pub fn source_files(&self) -> crate::List<SourceFile> {
        unsafe {
            crate::collections::List::from_raw_none(ffi::vala_code_context_get_source_files(
                self.as_raw(),
            ))
            .expect("source files list was null")
        }
    }

    /// The root namespace of the parsed code tree.
    pub fn root(&self) -> crate::Namespace {
        unsafe {
            crate::Namespace::from_raw_none(ffi::vala_code_context_get_root(self.as_raw()))
                .expect("context root namespace was null")
        }
    }

    pub(crate) fn source_file_type(ty: SourceFileType) -> ffi::ValaSourceFileType {
        ty.to_ffi()
    }
}

impl Default for CodeContext {
    fn default() -> Self {
        Self::new()
    }
}

struct PopGuard;

impl Drop for PopGuard {
    fn drop(&mut self) {
        CodeContext::pop();
    }
}
