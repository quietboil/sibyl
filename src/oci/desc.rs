//! OCI descriptors

use crate::{Result, Error};
use super::*;
use libc::c_void;
use std::ptr;

pub trait DescriptorType : OCIStruct {
    type OCIType;
    fn get_type() -> u32;
}

macro_rules! impl_descr_type {
    ($($oci_desc:ident => $id:ident, $ret:ident),+) => {
        $(
            impl DescriptorType for $oci_desc {
                type OCIType = $ret;
                fn get_type() -> u32 { $id }
            }
        )+
    };
}

impl_descr_type!{
    OCICLobLocator          => OCI_DTYPE_LOB,           OCILobLocator,
    OCIBLobLocator          => OCI_DTYPE_LOB,           OCILobLocator,
    OCIBFileLocator         => OCI_DTYPE_FILE,          OCILobLocator,
    OCIRowid                => OCI_DTYPE_ROWID,         OCIRowid,
    OCIParam                => OCI_DTYPE_PARAM,         OCIParam,
    OCITimestamp            => OCI_DTYPE_TIMESTAMP,     OCIDateTime,
    OCITimestampTZ          => OCI_DTYPE_TIMESTAMP_TZ,  OCIDateTime,
    OCITimestampLTZ         => OCI_DTYPE_TIMESTAMP_LTZ, OCIDateTime,
    OCIIntervalYearToMonth  => OCI_DTYPE_INTERVAL_YM,   OCIInterval,
    OCIIntervalDayToSecond  => OCI_DTYPE_INTERVAL_DS,   OCIInterval
}

pub struct Descriptor<T: DescriptorType> {
    ptr: Ptr<T>,
}

impl<T: DescriptorType> Drop for Descriptor<T> {
    fn drop(&mut self) {
        let ptr = self.ptr.get();
        if !ptr.is_null() {
            unsafe {
                OCIDescriptorFree(ptr as *mut c_void, T::get_type());
            }
        }
    }
}

impl<T: DescriptorType> Descriptor<T> {
    fn alloc(env: *mut OCIEnv) -> Result<*mut T> {
        let mut desc = ptr::null_mut::<T>();
        let res = unsafe {
            OCIDescriptorAlloc(env, &mut desc as *mut *mut T as *mut *mut c_void, T::get_type(), 0, ptr::null())
        };
        if res != OCI_SUCCESS {
            Err( Error::env(env, res) )
        } else if desc.is_null() {
            Err( Error::new("OCIDescriptorAlloc returned NULL") )
        } else {
            Ok( desc )
        }
    }

    pub(crate) fn new(env: *mut OCIEnv) -> Result<Self> {
        let desc = Self::alloc(env)?;
        Ok( Self { ptr: Ptr::new(desc) } )
    }

    pub(crate) fn from(ptr: *mut T) -> Self {
        Self { ptr: Ptr::new(ptr) }
    }

    pub(crate) fn get(&self) -> *mut T::OCIType {
        self.ptr.get() as *mut T::OCIType
    }

    pub(crate) fn as_ptr(&self) -> *mut *mut T::OCIType {
        self.ptr.as_ptr() as *mut *mut T::OCIType
    }

    pub(crate) fn swap(&self, other: &Self) {
        self.ptr.swap(&other.ptr);
    }

    pub(crate) fn get_attr<V: attr::AttrGet>(&self, attr_type: u32, err: *mut OCIError) -> Result<V> {
        attr::get::<V>(attr_type, T::get_type(), self.get() as *const c_void, err)
    }

    pub(crate) fn set_attr<V: attr::AttrSet>(&self, attr_type: u32, attr_val: V, err: *mut OCIError) -> Result<()> {
        attr::set::<V>(attr_type, attr_val, T::get_type(), self.get() as *mut c_void, err)
    }
}