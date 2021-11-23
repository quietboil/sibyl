//! Send-able pointers to OCI handles and descriptors

use std::ptr;
use libc::c_void;

use super::{OCIServer, OCISession, OCIStruct, OCISvcCtx, attr::{AttrGet, AttrGetInto}};

/// Send-able cell-like wrapper around a pointer to OCI handle or descriptor.
pub struct Ptr<T: OCIStruct> {
    value: *mut T
}

impl<T: OCIStruct> Ptr<T> {    
    pub(crate) fn new(ptr: *mut T) -> Self {
        Self{ value: ptr }
    }

    pub(crate) fn null() -> Self {
        Self{ value: ptr::null_mut() }
    }

    pub(crate) fn swap(&mut self, other: &mut Self) {
        if !ptr::eq(self, other) {
            unsafe {
                ptr::swap(&mut self.value, &mut other.value);
            }
        }
    }

    pub(crate) fn get(&self) -> *mut T {
        self.value
    }

    pub(crate) fn as_ptr(&self) -> *const *mut T {
        &self.value as *const *mut T
    }

    pub(crate) fn as_mut_ptr(&mut self) -> *mut *mut T {
        &mut self.value as *mut *mut T
    }
}

impl<T: OCIStruct> Clone for Ptr<T> {
    fn clone(&self) -> Self {
        Self { value: self.value }
    }
}

impl<T: OCIStruct> AttrGetInto for Ptr<T> {
    fn as_val_ptr(&mut self) -> *mut c_void { self.as_mut_ptr() as *mut c_void }
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

/*
    Server, Session and Service Context handle pointers are read-only.
    They also do not use interior mutability. Most importantly, because
    OCI environment is created by sibyl in OCI_THREADED mode, the internal
    OCI structures are protected by OCI itself from concurrent access by
    multiple threads.
*/
unsafe impl Sync for Ptr<OCIServer> {}
unsafe impl Sync for Ptr<OCISvcCtx> {}
unsafe impl Sync for Ptr<OCISession> {}
