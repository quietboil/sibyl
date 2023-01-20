//! OCI descriptors

use crate::{Result, Error, oci};
use super::*;
use std::ops::{Deref, DerefMut};

pub(crate) struct Descriptor<T> (Ptr<T::OCIType>)
where T: DescriptorType
    , T::OCIType: OCIStruct
;

impl<T> Deref for Descriptor<T>
where T: DescriptorType
    , T::OCIType: OCIStruct
{
    type Target = T::OCIType;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl<T> DerefMut for Descriptor<T>
where T: DescriptorType
    , T::OCIType: OCIStruct
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut()
    }
}

impl<T> AsRef<T::OCIType> for Descriptor<T>
where T: DescriptorType
    , T::OCIType: OCIStruct
{
    fn as_ref(&self) -> &T::OCIType {
        self.0.as_ref()
    }
}

impl<T> AsMut<T::OCIType> for Descriptor<T>
where T: DescriptorType
    , T::OCIType: OCIStruct
{
    fn as_mut(&mut self) -> &mut T::OCIType {
        self.0.as_mut()
    }
}

impl<T> Drop for Descriptor<T>
where T: DescriptorType
    , T::OCIType: OCIStruct
{
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                OCIDescriptorFree(self.0.get() as _, T::get_type());
            }
        }
    }
}

impl<T> Descriptor<T>
    where T: DescriptorType
        , T::OCIType : OCIStruct
{
    fn alloc(env: &impl AsRef<OCIEnv>) -> Result<Ptr<T::OCIType>> {
        let mut desc_ptr = Ptr::<T::OCIType>::null();
        oci::descriptor_alloc(env.as_ref(), desc_ptr.as_mut_ptr(), T::get_type())?;
        if desc_ptr.is_null() {
            Err( Error::new("OCIDescriptorAlloc returned NULL") )
        } else {
            Ok( desc_ptr )
        }
    }

    pub(crate) fn new(env: &impl AsRef<OCIEnv>) -> Result<Self> {
        let desc_ptr = Self::alloc(env)?;
        Ok( Self(desc_ptr) )
    }

    pub(crate) fn from(desc_ptr: Ptr<T::OCIType>) -> Self {
        Self(desc_ptr)
    }

    pub(crate) fn take(other: &mut Self) -> Self {
        let mut desc_ptr = Ptr::<T::OCIType>::null();
        desc_ptr.swap(&mut other.0);
        Self(desc_ptr)
    }

    pub(crate) fn replace(&mut self, mut ptr: Ptr<T::OCIType>) {
        self.0.swap(&mut ptr);
    }

    pub(crate) fn get_ptr(&self) -> Ptr<T::OCIType> {
        self.0
    }

    pub(crate) fn as_ptr(&self) -> *const *mut T::OCIType {
        self.0.as_ptr() as _
    }

    pub(crate) fn as_mut_ptr(&mut self) -> *mut *mut T::OCIType {
        self.0.as_mut_ptr() as _
    }

    pub(crate) fn swap(&mut self, other: &mut Self) {
        self.0.swap(&mut other.0);
    }

    pub(crate) fn get_attr<V: attr::AttrGet>(&self, attr_type: u32, err: &OCIError) -> Result<V> {
        attr::get::<T::OCIType, V>(attr_type, T::get_type(), &self.0, err)
    }

    pub(crate) fn get_attr_into<V: attr::AttrGetInto>(&self, attr_type: u32, into: &mut V, err: &OCIError) -> Result<()> {
        attr::get_into::<T::OCIType, V>(attr_type, into, T::get_type(), &self.0, err)
    }

    pub(crate) fn set_attr<V: attr::AttrSet>(&self, attr_type: u32, attr_val: V, err: &OCIError) -> Result<()> {
        attr::set::<T::OCIType, V>(attr_type, attr_val, T::get_type(), &self.0, err)
    }

    // pub(crate) fn dump(&self, pfx: &str)
    // {
    //     let ptr = self.0.get() as *const libc::c_void as *const u8;
    //     let mem = std::ptr::slice_from_raw_parts(ptr, 32);
    //     let mem = unsafe { &*mem };
    //     println!("{pfx}: {mem:?}");
    // }
}
