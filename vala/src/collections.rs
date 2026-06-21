//! Safe wrappers over libvala's `Vala.List` / `Vala.Iterator` (the bundled gee
//! collection types). Container getters such as
//! [`Namespace::classes`](crate::Namespace::classes) return these.
//!
//! The element type is recovered as the wrapper `T`; the list getter transfers
//! a strong reference per element, so iteration yields owned `T` handles.

use std::marker::PhantomData;

use vala_sys as ffi;

use crate::object::RawWrapper;

/// A read view over a `Vala.List<T>`.
///
/// The list is reference counted; cloning the [`List`] shares the underlying
/// collection. Elements are produced lazily as owned `T` wrappers.
pub struct List<T> {
    raw: std::ptr::NonNull<ffi::ValaList>,
    _marker: PhantomData<T>,
}

impl<T: RawWrapper> List<T> {
    /// Wrap a borrowed (transfer-none) `ValaList*`, adding one strong reference
    /// so the handle can outlive the borrow. Most libvala container getters
    /// return `unowned` lists, which must be wrapped this way.
    ///
    /// # Safety
    /// `ptr` must be null or a valid `ValaList` whose elements are instances of
    /// `T`'s class.
    pub(crate) unsafe fn from_raw_none(ptr: *mut ffi::ValaList) -> Option<Self> {
        let raw = std::ptr::NonNull::new(ptr)?;
        ffi::vala_iterable_ref(raw.as_ptr() as ffi::gpointer);
        Some(List {
            raw,
            _marker: PhantomData,
        })
    }

    fn as_collection(&self) -> *mut ffi::ValaCollection {
        self.raw.as_ptr() as *mut ffi::ValaCollection
    }

    /// Number of elements.
    pub fn len(&self) -> usize {
        unsafe { ffi::vala_collection_get_size(self.as_collection()).max(0) as usize }
    }

    /// Whether the list is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The element at `index`, or `None` if out of bounds.
    pub fn get(&self, index: usize) -> Option<T> {
        if index >= self.len() {
            return None;
        }
        unsafe {
            // The gee getter returns a transfer-full reference.
            let elem = ffi::vala_list_get(self.raw.as_ptr(), index as ffi::gint);
            T::from_raw_full(elem as *mut T::Ffi)
        }
    }

    /// Iterate over the elements as owned wrappers.
    pub fn iter(&self) -> ListIter<'_, T> {
        ListIter {
            list: self,
            index: 0,
            len: self.len(),
        }
    }

    /// Collect into a `Vec`.
    pub fn to_vec(&self) -> Vec<T> {
        self.iter().collect()
    }
}

impl<T: RawWrapper> Clone for List<T> {
    fn clone(&self) -> Self {
        unsafe {
            ffi::vala_iterable_ref(self.raw.as_ptr() as ffi::gpointer);
            List {
                raw: self.raw,
                _marker: PhantomData,
            }
        }
    }
}

impl<T> Drop for List<T> {
    fn drop(&mut self) {
        unsafe { ffi::vala_iterable_unref(self.raw.as_ptr() as ffi::gpointer) }
    }
}

impl<T: RawWrapper> std::fmt::Debug for List<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("List").field("len", &self.len()).finish()
    }
}

/// Borrowing iterator over a [`List`], yielding owned `T` wrappers.
pub struct ListIter<'a, T> {
    list: &'a List<T>,
    index: usize,
    len: usize,
}

impl<T: RawWrapper> Iterator for ListIter<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.index >= self.len {
            return None;
        }
        let item = self.list.get(self.index);
        self.index += 1;
        item
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len.saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl<T: RawWrapper> ExactSizeIterator for ListIter<'_, T> {}

impl<'a, T: RawWrapper> IntoIterator for &'a List<T> {
    type Item = T;
    type IntoIter = ListIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
