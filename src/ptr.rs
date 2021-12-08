//! Scoped send and sync pointer

/// Pointer that can be sent to async tasks and that are guaranteed to outlast the task they are sent to.
/// The "guarantee" is structural, so these pointers are restricted to very sepecific use cases.
pub(crate) struct ScopedPtr<T> {
    value: *const T
}

impl<T> ScopedPtr<T> {
    pub(crate) fn new(ptr: *const T) -> Self {
        Self{ value: ptr }
    }

    pub(crate) fn get(&self) -> *const T {
        self.value
    }
}

impl<T> Clone for ScopedPtr<T> {
    fn clone(&self) -> Self {
        Self { value: self.value }
    }
}

unsafe impl<T> Send for ScopedPtr<T> {}
unsafe impl<T> Sync for ScopedPtr<T> {}
