//! Send-able pointers to OCI handles and descriptors

use std::{cell::UnsafeCell, ptr};
use super::OCIStruct;

/// Send-able cell-like wrapper around a pointer to OCI handle or descriptor.
pub struct Ptr<T: OCIStruct> {
    value: UnsafeCell<*mut T>
}

impl<T: OCIStruct> Ptr<T> {    
    pub(crate) fn new(ptr: *mut T) -> Self {
        Self{ value: UnsafeCell::new(ptr) }
    }

    pub(crate) fn null() -> Self {
        Self{ value: UnsafeCell::new(ptr::null_mut()) }
    }

    pub(crate) fn swap(&self, other: &Self) {
        if !ptr::eq(self, other) {
            unsafe {
                ptr::swap(self.value.get(), other.value.get());
            }
        }
    }

    pub(crate) fn get(&self) -> *mut T {
        unsafe { *self.value.get() }
    }

    pub(crate) fn as_ptr(&self) -> *mut *mut T {
        self.value.get()
    }
}

unsafe impl<T: OCIStruct> Send for Ptr<T> {}
