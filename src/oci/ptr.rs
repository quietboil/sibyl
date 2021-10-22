//! Send-able pointers to OCI handles and descriptors

use std::{cell::UnsafeCell, ptr};
use super::*;

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

    pub(crate) fn set(&self, mut ptr: *mut T) {
        unsafe {
            ptr::swap(self.value.get(), &mut ptr);
        }
        // drop(ptr)
    }

    pub(crate) fn swap(&self, other: &Self) {
        if !ptr::eq(self, other) {
            unsafe {
                ptr::swap(self.value.get(), other.value.get());
            }
        }
    }

    pub(crate) fn take(&self) -> *mut T {
        let mut value = ptr::null_mut();
        unsafe {
            ptr::swap(self.value.get(), &mut value);
        }
        value
    }

    pub(crate) fn get(&self) -> *mut T {
        unsafe { *self.value.get() }
    }

    pub(crate) fn as_ptr(&self) -> *mut *mut T {
        self.value.get()
    }
}

unsafe impl<T: OCIStruct> Send for Ptr<T> {}
