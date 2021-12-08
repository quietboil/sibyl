//! OCI descriptors

use crate::{Result, Error, oci};
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

pub struct Descriptor<T>
    where T: DescriptorType
        , T::OCIType : OCIStruct
{
    ptr: Ptr<T>,
}

impl<T> Drop for Descriptor<T>
    where T: DescriptorType
        , T::OCIType : OCIStruct
{
    fn drop(&mut self) {
        let ptr = self.ptr.get();
        if !ptr.is_null() {
            unsafe {
                OCIDescriptorFree(ptr as *mut c_void, T::get_type());
            }
        }
    }
}

impl<T> Descriptor<T>
    where T: DescriptorType
        , T::OCIType : OCIStruct
{
    fn alloc(env: *mut OCIEnv) -> Result<*mut T> {
        let mut desc = ptr::null_mut::<T>();
        oci::descriptor_alloc(env, &mut desc as *mut *mut T as *mut *mut c_void, T::get_type(), 0, ptr::null())?;
        if desc.is_null() {
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

    pub(crate) fn take_over(other: &mut Self) -> Self {
        let mut ptr = Ptr::null();
        ptr.swap(&mut other.ptr);
        Self { ptr }
    }

    pub(crate) fn replace(&mut self, ptr: Ptr<T::OCIType>) {
        let mut ptr = Ptr::new(ptr.get() as *mut T);
        self.ptr.swap(&mut ptr);
    }

    pub(crate) fn get_ptr(&self) -> Ptr<T::OCIType> {
        Ptr::new(self.get())
    }

    pub(crate) fn get(&self) -> *mut T::OCIType {
        self.ptr.get() as *mut T::OCIType
    }

    pub(crate) fn as_ptr(&self) -> *const *mut T::OCIType {
        self.ptr.as_ptr() as *const *mut T::OCIType
    }

    pub(crate) fn as_mut_ptr(&mut self) -> *mut *mut T::OCIType {
        self.ptr.as_ptr() as *mut *mut T::OCIType
    }

    pub(crate) fn swap(&mut self, other: &mut Self) {
        self.ptr.swap(&mut other.ptr);
    }

    pub(crate) fn get_attr<V: attr::AttrGet>(&self, attr_type: u32, err: *mut OCIError) -> Result<V> {
        attr::get::<V>(attr_type, T::get_type(), self.get() as *const c_void, err)
    }

    pub(crate) fn set_attr<V: attr::AttrSet>(&self, attr_type: u32, attr_val: V, err: *mut OCIError) -> Result<()> {
        attr::set::<V>(attr_type, attr_val, T::get_type(), self.get() as *mut c_void, err)
    }
}
