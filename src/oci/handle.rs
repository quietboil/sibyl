//! OCI handles.

use crate::{Result, Error, oci};
use super::*;
use std::ops::{Deref, DerefMut};

pub(crate) struct Handle<T: HandleType> (Ptr<T>);

impl<T: HandleType> Deref for Handle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: HandleType> DerefMut for Handle<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: HandleType> AsRef<T> for Handle<T> {
    fn as_ref(&self) -> &T {
        self.0.deref()
    }
}

unsafe impl Sync for Handle<OCIEnv> {}
unsafe impl Sync for Handle<OCIError> {}
unsafe impl Sync for Handle<OCISPool> {}
unsafe impl Sync for Handle<OCICPool> {}
unsafe impl Sync for Handle<OCIServer> {}
unsafe impl Sync for Handle<OCISvcCtx> {}
unsafe impl Sync for Handle<OCISession> {}

impl<T: HandleType> Drop for Handle<T> {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                OCIHandleFree(self.0.get() as _, T::get_type());
            }
        }
    }
}

impl<T: HandleType> Handle<T> {
    fn alloc(env: &impl AsRef<OCIEnv>) -> Result<Ptr<T>> {
        let mut handle_ptr = Ptr::<T>::null();
        oci::handle_alloc(env.as_ref(), handle_ptr.as_mut_ptr(), T::get_type())?;
        if handle_ptr.is_null() {
            Err( Error::msg(format!("OCI returned NULL for handle {}", T::get_type())) )
        } else {
            Ok( handle_ptr )
        }
    }

    pub(crate) fn new(env: &impl AsRef<OCIEnv>) -> Result<Self> {
        let handle_ptr = Self::alloc(env)?;
        Ok( Self(handle_ptr) )
    }

    // Some handles (like OCIEnv) are allocated by their respective OCI*Create* APIs.
    // But we need to dispose of them (as handles) when it is time to drop them.
    pub(crate) fn from(handle_ptr: Ptr<T>) -> Self {
        Self(handle_ptr)
    }

    pub(crate) fn take(other: &mut Self) -> Self {
        let mut handle_ptr = Ptr::<T>::null();
        handle_ptr.swap(&mut other.0);
        Self(handle_ptr)
    }

    pub(crate) fn get_ptr(&self) -> Ptr<T> {
        self.0
    }

    pub(crate) fn as_ptr(&self) -> *const *mut T {
        self.0.as_ptr()
    }

    pub(crate) fn as_mut_ptr(&mut self) -> *mut *mut T {
        self.0.as_mut_ptr()
    }

    pub(crate) fn swap(&mut self, other: &mut Self) {
        self.0.swap(&mut other.0);
    }

    pub(crate) fn get_attr<V: attr::AttrGet>(&self, attr_type: u32, err: &OCIError) -> Result<V> {
        attr::get::<T, V>(attr_type, T::get_type(), &self.0, err)
    }

    pub(crate) fn get_attr_into<V: attr::AttrGetInto>(&self, attr_type: u32, into: &mut V, err: &OCIError) -> Result<()> {
        attr::get_into::<T, V>(attr_type, into, T::get_type(), &self.0, err)
    }

    pub(crate) fn set_attr<V: attr::AttrSet>(&self, attr_type: u32, attr_val: V, err: &OCIError) -> Result<()> {
        attr::set::<T, V>(attr_type, attr_val, T::get_type(), &self.0, err)
    }
}
