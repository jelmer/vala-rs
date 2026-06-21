//! Runtime support for the generated wrapper types: reference counting, raw
//! pointer bridging, and the `IsA` / `Implements` upcast markers.
//!
//! libvala's compiler classes are GLib *fundamental* reference-counted types
//! (a `GTypeInstance` plus a `ref_count`), not `GObject`s. Each root type owns a
//! `vala_<root>_ref` / `vala_<root>_unref` pair that operates on any instance in
//! its hierarchy. The generated wrappers therefore store, per type, the correct
//! root ref/unref pair and the type's own `get_type` for checked downcasts.

/// Marker: `Self` can be treated as a `T` (i.e. `T` is `Self` or an ancestor).
///
/// # Safety
/// Implementors guarantee that a pointer to `Self`'s instance is also a valid
/// pointer to a `T` instance, matching the libvala class hierarchy.
pub unsafe trait IsA<T>: 'static {}

/// Marker: `Self` implements the libvala interface `I`.
///
/// # Safety
/// Implementors guarantee the underlying instance implements interface `I`.
pub unsafe trait Implements<I> {}

/// Common behaviour shared by every generated wrapper.
///
/// # Safety
/// Implementors must back the wrapper with a live, ref-counted libvala instance
/// and return its raw pointer from [`as_raw`](RawWrapper::as_raw).
pub unsafe trait RawWrapper: Sized {
    /// The opaque sys instance type, e.g. `ffi::ValaCodeContext`.
    type Ffi;

    /// Borrow the underlying instance pointer. Always non-null for a live value.
    fn as_raw(&self) -> *mut Self::Ffi;

    /// Wrap a raw pointer, taking ownership of one strong reference (does not
    /// add a reference). Returns `None` for a null pointer.
    ///
    /// # Safety
    /// `ptr` must be either null or a valid instance of this type's class for
    /// which the caller transfers one owned reference.
    unsafe fn from_raw_full(ptr: *mut Self::Ffi) -> Option<Self>;

    /// Wrap a raw pointer, adding one strong reference (borrowed transfer).
    ///
    /// # Safety
    /// `ptr` must be either null or a valid instance of this type's class.
    unsafe fn from_raw_none(ptr: *mut Self::Ffi) -> Option<Self>;

    /// The GType of this wrapper's class.
    fn static_type() -> glib::types::Type;
}

/// Cast helpers available on every wrapper through a blanket impl.
pub trait Cast: Sized {
    /// Upcast to an ancestor type by value. Infallible at compile time.
    fn upcast<T>(self) -> T
    where
        Self: IsA<T>,
        T: RawWrapper,
        Self: RawWrapper,
    {
        // SAFETY: `Self: IsA<T>` guarantees pointer compatibility; we transfer
        // our owned reference into the target wrapper.
        unsafe {
            let ptr = self.as_raw() as *mut T::Ffi;
            std::mem::forget(self);
            T::from_raw_full(ptr).expect("non-null instance during upcast")
        }
    }

    /// Borrow as an ancestor type.
    fn upcast_ref<T>(&self) -> &T
    where
        Self: AsRef<T>,
    {
        self.as_ref()
    }

    /// Attempt a checked downcast to a more derived type `T`, consuming `self`.
    /// Returns the original value back on failure.
    fn downcast<T>(self) -> Result<T, Self>
    where
        Self: RawWrapper,
        T: RawWrapper + IsA<Self>,
    {
        if self.is::<T>() {
            // SAFETY: runtime GType check passed; transfer the owned reference.
            unsafe {
                let ptr = self.as_raw() as *mut T::Ffi;
                std::mem::forget(self);
                Ok(T::from_raw_full(ptr).expect("non-null instance during downcast"))
            }
        } else {
            Err(self)
        }
    }

    /// Runtime test of whether this instance is of type `T` (or a subtype).
    fn is<T>(&self) -> bool
    where
        Self: RawWrapper,
        T: RawWrapper,
    {
        unsafe {
            let instance = self.as_raw() as *mut gobject_sys::GTypeInstance;
            if instance.is_null() {
                return false;
            }
            let want = glib::translate::IntoGlib::into_glib(T::static_type());
            gobject_sys::g_type_check_instance_is_a(instance, want) != glib_sys::GFALSE
        }
    }
}

impl<W: RawWrapper> Cast for W {}

/// Internal: turn a borrowed C string into an owned `String`, or `None` if null.
///
/// # Safety
/// `ptr` must be null or point to a valid NUL-terminated C string borrowed for
/// the duration of the call.
pub(crate) unsafe fn opt_string(ptr: *const std::os::raw::c_char) -> Option<String> {
    if ptr.is_null() {
        None
    } else {
        Some(std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned())
    }
}

/// Internal: turn an owned (transfer-full) C string into an owned `String`,
/// freeing the original with `g_free`.
///
/// # Safety
/// `ptr` must be null or an owned NUL-terminated string allocated by GLib.
#[allow(dead_code)]
pub(crate) unsafe fn take_string(ptr: *mut std::os::raw::c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let s = std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned();
    glib_sys::g_free(ptr as glib_sys::gpointer);
    Some(s)
}

/// Re-exported for the generated code and macro.
#[doc(hidden)]
pub use ::std::ptr::NonNull as __NonNull;

/// Define a ref-counted wrapper over a libvala fundamental type.
///
/// Generates the struct, `RawWrapper`, `Clone`, `Drop`, and `Debug`.
#[macro_export]
#[doc(hidden)]
macro_rules! ffi_wrapper {
    (
        $(#[$meta:meta])*
        pub struct $name:ident(ptr: *mut $ffi:path) {
            root_ref: $root_ref:ident,
            root_unref: $root_unref:ident,
            get_type: $get_type:ident,
        }
    ) => {
        $(#[$meta])*
        #[repr(transparent)]
        pub struct $name($crate::object::__NonNull<$ffi>);

        unsafe impl $crate::object::RawWrapper for $name {
            type Ffi = $ffi;

            #[inline]
            fn as_raw(&self) -> *mut $ffi {
                self.0.as_ptr()
            }

            unsafe fn from_raw_full(ptr: *mut $ffi) -> Option<Self> {
                $crate::object::__NonNull::new(ptr).map(Self)
            }

            unsafe fn from_raw_none(ptr: *mut $ffi) -> Option<Self> {
                let nn = $crate::object::__NonNull::new(ptr)?;
                ffi::$root_ref(ptr as ffi::gpointer);
                Some(Self(nn))
            }

            fn static_type() -> glib::types::Type {
                unsafe { glib::translate::from_glib(ffi::$get_type()) }
            }
        }

        impl Clone for $name {
            fn clone(&self) -> Self {
                unsafe {
                    ffi::$root_ref(self.0.as_ptr() as ffi::gpointer);
                    Self(self.0)
                }
            }
        }

        impl Drop for $name {
            fn drop(&mut self) {
                unsafe { ffi::$root_unref(self.0.as_ptr() as ffi::gpointer) }
            }
        }

        impl ::std::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.debug_tuple(stringify!($name))
                    .field(&self.0.as_ptr())
                    .finish()
            }
        }
    };
}
