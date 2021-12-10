//! Scoped send and sync pointer

/// Pointer that can be sent to async tasks and that are guaranteed to outlast the task they are sent to.
/// The "guarantee" is structural, so these pointers are restricted to very specific use cases.
pub struct ScopedPtr<T> (*const T);

impl<T> ScopedPtr<T> {
    pub(crate) fn new(ptr: *const T) -> Self {
        Self(ptr)
    }

    pub(crate) fn get(&self) -> *const T {
        self.0
    }
}

impl<T> Clone for ScopedPtr<T> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

unsafe impl<T> Send for ScopedPtr<T> {}
unsafe impl<T> Sync for ScopedPtr<T> {}

/// Mutable pointer that can be sent to async tasks and that are guaranteed to outlast the task they are sent to.
/// The "guarantee" is structural, so these pointers are restricted to very specific use cases.
pub struct ScopedMutPtr<T> (*mut T);

impl<T> ScopedMutPtr<T> {
    pub(crate) fn new(ptr: *mut T) -> Self {
        Self(ptr)
    }

    // pub(crate) fn get(&self) -> *const T {
    //     self.0 as _
    // }

    pub(crate) fn get_mut(&mut self) -> *mut T {
        self.0
    }
}

impl<T> Clone for ScopedMutPtr<T> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

unsafe impl<T> Send for ScopedMutPtr<T> {}
unsafe impl<T> Sync for ScopedMutPtr<T> {}
