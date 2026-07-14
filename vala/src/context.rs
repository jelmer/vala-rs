//! Curated methods on [`CodeContext`], the entry point for driving the compiler.

use std::cell::Cell;
use std::ffi::CString;
use std::sync::{Mutex, MutexGuard};

use vala_sys as ffi;

use crate::object::RawWrapper;
use crate::{CodeContext, Report, SourceFile};

/// libvala keeps the current [`CodeContext`] on a process-global stack and does
/// not synchronise it, so concurrent compilations corrupt each other: errors get
/// attributed to the wrong file, or the compiler segfaults. Holding this is what
/// grants the right to push.
static CONTEXT_STACK: Mutex<()> = Mutex::new(());

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

    /// Run `f` with this context current, popping it afterwards even if `f`
    /// panics.
    ///
    /// Blocks until no other thread is inside `with_current`, and holds that
    /// exclusion for the whole of `f`: compilations are serialised, not
    /// parallel. Keep `f` to the work that needs the context current.
    ///
    /// # Panics
    ///
    /// Panics if called re-entrantly on this thread. Nesting a different context
    /// leaves it ambiguous which is current, and nesting the same one is
    /// redundant, so both fail loudly rather than block forever.
    pub fn with_current<R>(&self, f: impl FnOnce(&Self) -> R) -> R {
        // Before locking: this thread holds the lock already, so blocking on it
        // would deadlock instead of reaching the panic.
        assert!(
            !IS_CURRENT.get(),
            "CodeContext::with_current is not re-entrant: this thread already \
             holds libvala's context, and taking it again would deadlock"
        );
        // Recover from poisoning: Current::drop pops on unwind, so a panicking
        // compilation leaves nothing broken for the next one.
        let guard = CONTEXT_STACK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _current = Current::push(self, guard);
        f(self)
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

thread_local! {
    /// Set while this thread is inside [`CodeContext::with_current`]. A `Mutex`
    /// will not say whether the caller already holds it, so track it separately.
    static IS_CURRENT: Cell<bool> = const { Cell::new(false) };
}

/// A pushed context, owning the lock that permitted the push. Dropping it pops
/// and then unlocks, so a panicking compilation still leaves the stack balanced
/// for the next thread.
struct Current<'a> {
    _lock: MutexGuard<'a, ()>,
}

impl<'a> Current<'a> {
    fn push(ctx: &CodeContext, lock: MutexGuard<'a, ()>) -> Self {
        // Neither of these can unwind, so the guard always exists to undo them.
        IS_CURRENT.set(true);
        unsafe { ffi::vala_code_context_push(ctx.as_raw()) };
        Current { _lock: lock }
    }
}

impl Drop for Current<'_> {
    fn drop(&mut self) {
        unsafe { ffi::vala_code_context_pop() };
        IS_CURRENT.set(false);
    }
}
