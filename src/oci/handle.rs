//! OCI handles.

use crate::{Result, Error, oci};
use super::*;
use libc::c_void;
use std::ptr;

pub trait HandleType : OCIStruct {
    fn get_type() -> u32;
}

macro_rules! impl_handle_type {
    ($($oci_handle:ty => $id:ident),+) => {
        $(
            impl HandleType for $oci_handle {
                fn get_type() -> u32 { $id }
            }
        )+
    };
}

impl_handle_type!{
    OCIEnv      => OCI_HTYPE_ENV,
    OCIError    => OCI_HTYPE_ERROR,
    OCIServer   => OCI_HTYPE_SERVER,
    OCISvcCtx   => OCI_HTYPE_SVCCTX,
    OCISession  => OCI_HTYPE_SESSION,
    OCIAuthInfo => OCI_HTYPE_AUTHINFO,
    OCIStmt     => OCI_HTYPE_STMT,
    OCIBind     => OCI_HTYPE_BIND,
    OCIDefine   => OCI_HTYPE_DEFINE,
    OCIDescribe => OCI_HTYPE_DESCRIBE
}

pub struct Handle<T: HandleType> {
    ptr: Ptr<T>
}

/*
    All but OCIStmt handles are read-only (as far as Rust is concerned).
    They also do not use interior mutability. Most importantly, because
    OCI environment is created by sibyl in OCI_THREADED mode, the internal
    OCI structures are protected by OCI itself from concurrent access by
    multiple threads.

    `Handle<OCIStmt>` on the other hand needs to be mutable to allow FromSql
    to swap cursor handles and thus it would have to be put behind a RwLock.
    As long as we are locking OCIStmt we might as well lock the entire
    Statement and thus keep Bind, Define and Describe as !Sync.
*/
unsafe impl Sync for Handle<OCIEnv> {}
unsafe impl Sync for Handle<OCIError> {}
unsafe impl Sync for Handle<OCIServer> {}
unsafe impl Sync for Handle<OCISvcCtx> {}
unsafe impl Sync for Handle<OCISession> {}

impl<T: HandleType> Drop for Handle<T> {
    fn drop(&mut self) {
        let ptr = self.ptr.get();
        if !ptr.is_null() {
            unsafe {
                OCIHandleFree(ptr as *mut c_void, T::get_type());
            }
        }
    }
}

impl<T: HandleType> Handle<T> {
    fn alloc(env: *mut OCIEnv) -> Result<*mut T> {
        let mut handle = ptr::null_mut::<T>();
        oci::handle_alloc(env, &mut handle as *mut *mut T as *mut *mut c_void, T::get_type(), 0, ptr::null())?;
        if handle.is_null() {
            Err( Error::new(&format!("OCI returned NULL for handle {}", T::get_type())) )
        } else {
            Ok( handle )
        }
    }

    pub(crate) fn new(env: *mut OCIEnv) -> Result<Self> {
        let ptr = Self::alloc(env)?;
        Ok( Self { ptr: Ptr::new(ptr) } )
    }

    pub(crate) fn from(ptr: *mut T) -> Self {
        Self { ptr: Ptr::new(ptr) }
    }

    pub(crate) fn from_handle(handle: &mut Self) -> Self {
        let mut ptr = Ptr::null();
        ptr.swap(&mut handle.ptr);
        Self { ptr }
    }

    pub(crate) fn get(&self) -> *mut T {
        self.ptr.get()
    }

    pub(crate) fn as_ptr(&self) -> *const *mut T {
        self.ptr.as_ptr()
    }

    pub(crate) fn as_mut_ptr(&mut self) -> *mut *mut T {
        self.ptr.as_mut_ptr()
    }

    pub(crate) fn swap(&mut self, other: &mut Self) {
        self.ptr.swap(&mut other.ptr);
    }

    pub(crate) fn get_attr<V: attr::AttrGet>(&self, attr_type: u32, err: *mut OCIError) -> Result<V> {
        let ptr = self.ptr.get();
        attr::get::<V>(attr_type, T::get_type(), ptr as *const c_void, err)
    }

    pub(crate) fn get_attr_into<V: attr::AttrGetInto>(&self, attr_type: u32, into: &mut V, err: *mut OCIError) -> Result<()> {
        let ptr = self.ptr.get();
        attr::get_into::<V>(attr_type, into, T::get_type(), ptr as *const c_void, err)
    }

    pub(crate) fn set_attr<V: attr::AttrSet>(&self, attr_type: u32, attr_val: V, err: *mut OCIError) -> Result<()> {
        let ptr = self.ptr.get();
        attr::set::<V>(attr_type, attr_val, T::get_type(), ptr as *mut c_void, err)
    }
}
