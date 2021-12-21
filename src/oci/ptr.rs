//! Send-able pointers to OCI handles and descriptors

use std::{ptr, ops::{Deref, DerefMut}};

use libc::c_void;

use super::attr::{AttrGetInto, AttrGet, AttrSet};

/// Send-able cell-like wrapper around a pointer to OCI handle or descriptor.
pub struct Ptr<T> (*mut T);

impl<T> Ptr<T> {
    pub(crate) fn new(ptr: *const T) -> Self {
        Self(ptr as _)
    }

    pub(crate) fn null() -> Self {
        Self(ptr::null_mut())
    }

    pub(crate) fn swap(&mut self, other: &mut Self) {
        if !ptr::eq(self, other) && !ptr::eq(self.0, other.0) {
            unsafe {
                ptr::swap(&mut self.0, &mut other.0);
            }
        }
    }

    pub(crate) fn is_null(&self) -> bool {
        self.0.is_null()
    }

    pub(crate) fn get(&self) -> *const T {
        self.0
    }

    pub(crate) fn get_mut(&self) -> *mut T {
        self.0
    }

    pub(crate) fn as_ptr(&self) -> *const *mut T {
        &self.0 as _
    }

    pub(crate) fn as_mut_ptr(&mut self) -> *mut *mut T {
        &mut self.0 as _
    }
}

impl<T> From<&T> for Ptr<T> {
    fn from(oci_ref: &T) -> Self {
        Self(oci_ref as *const T as _)
    }
}

impl<T> Copy for Ptr<T> {}

impl<T> Clone for Ptr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Deref for Ptr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            &*self.0
        }
    }
}

impl<T> DerefMut for Ptr<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            &mut *self.0
        }
    }
}

impl<T> AsRef<T> for Ptr<T> {
    fn as_ref(&self) -> &T {
        unsafe {
            &*self.0
        }
    }
}

impl<T> AsMut<T> for Ptr<T> {
    fn as_mut(&mut self) -> &mut T {
        unsafe {
            &mut *self.0
        }
    }
}

impl<T> std::fmt::Pointer for Ptr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Pointer::fmt(&self.0, f)
    }
}

impl<T> AttrGet for Ptr<T> {
    type ValueType = *mut T;
    fn new(ptr: Self::ValueType, _len: usize) -> Self {
        Ptr::new(ptr)
    }
}

impl<T> AttrGetInto for Ptr<T> {
    fn as_mut_ptr(&mut self) -> *mut c_void {
        self.as_mut_ptr() as _
    }
}

impl<T> AttrSet for Ptr<T> {
    fn as_ptr(&self) -> *const c_void {
        self.get() as _
    }
}

unsafe impl<T> Send for Ptr<T> {}
unsafe impl<T> Sync for Ptr<T> {}
