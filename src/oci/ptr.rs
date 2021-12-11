//! Send-able pointers to OCI handles and descriptors

use std::ptr;
use libc::c_void;

use super::{OCIStruct, attr::{AttrGet, AttrGetInto}};

/// Send-able cell-like wrapper around a pointer to OCI handle or descriptor.
pub struct Ptr<T: OCIStruct> (*mut T);

impl<T: OCIStruct> Ptr<T> {
    pub(crate) fn new(ptr: *mut T) -> Self {
        Self(ptr)
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

    pub(crate) fn get(&self) -> *mut T {
        self.0
    }

    pub(crate) fn as_ptr(&self) -> *const *mut T {
        &self.0 as _
    }

    pub(crate) fn as_mut_ptr(&mut self) -> *mut *mut T {
        &mut self.0 as _
    }
}

impl<T: OCIStruct> Copy for Ptr<T> {}

impl<T: OCIStruct> Clone for Ptr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: OCIStruct> AttrGetInto for Ptr<T> {
    fn as_val_ptr(&mut self) -> *mut c_void { self.as_mut_ptr() as _ }
    fn capacity(&self) -> usize             { 0 }
    fn set_len(&mut self, _new_len: usize)  { }
}

impl<T: OCIStruct> AttrGet for Ptr<T> {
    type ValueType = *mut T;
    fn new(ptr: Self::ValueType, _len: usize) -> Self {
        Ptr::new(ptr)
    }
}

unsafe impl<T: OCIStruct> Send for Ptr<T> {}
unsafe impl<T: OCIStruct> Sync for Ptr<T> {}
