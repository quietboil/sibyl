use crate::*;
use crate::attr;
use libc::{ c_void, size_t };
use std::{ ptr, cell::Cell };

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/handle-and-descriptor-functions.html#GUID-C5BF55F7-A110-4CB5-9663-5056590F12B5
    fn OCIHandleAlloc(
        parenth:    *mut OCIEnv,
        hndlpp:     *mut *mut  c_void,
        hndl_type:  u32,
        xtramem_sz: size_t,
        usrmempp:   *const c_void
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/handle-and-descriptor-functions.html#GUID-E87E9F91-D3DC-4F35-BE7C-F1EFBFEEBA0A
    fn OCIHandleFree(
        hndlp:      *mut c_void,
        hnd_type:   u32
    ) -> i32;
}

pub struct Handle<T: HandleType> {
    ptr: Cell<*mut T>
}

pub trait HandleType {
    fn get_type(&self) -> u32;
}

macro_rules! impl_handle_type {
    ($($oci_handle:ty => $id:ident),+) => {
        $(
            impl HandleType for $oci_handle {
                fn get_type(&self) -> u32 {
                    $id
                }
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
    OCIStmt     => OCI_HTYPE_STMT,
    OCIBind     => OCI_HTYPE_BIND,
    OCIDefine   => OCI_HTYPE_DEFINE,
    OCIDescribe => OCI_HTYPE_DESCRIBE
}

impl<T: HandleType> Drop for Handle<T> {
    fn drop(&mut self) {
        let ptr = self.ptr.get();
        if !ptr.is_null() {
            unsafe {
                OCIHandleFree(ptr as *mut c_void, (*ptr).get_type());
            }
        }
    }
}

impl<T: HandleType> Handle<T> {
    fn alloc(env: *mut OCIEnv) -> Result<*mut T> {
        let mut handle = ptr::null_mut::<T>();
        let handle_type = unsafe {
            (*handle).get_type()
        };
        let res = unsafe {
            OCIHandleAlloc(env, &mut handle as *mut *mut T as *mut *mut c_void, handle_type, 0, ptr::null())
        };
        if res != OCI_SUCCESS {
            Err( Error::env(env, res) )
        } else if handle == ptr::null_mut() {
            Err( Error::new(&format!("OCI returned NULL for handle {}", handle_type)) )
        } else {
            Ok( handle )
        }
    }

    pub(crate) fn new(env: *mut OCIEnv) -> Result<Self> {
        let ptr = Self::alloc(env)?;
        Ok( Self { ptr: Cell::new(ptr) } )
    }

    pub(crate) fn from(ptr: *mut T) -> Self {
        Self { ptr: Cell::new(ptr) }
    }

    pub(crate) fn get(&self) -> *mut T {
        self.ptr.get()
    }

    pub(crate) fn as_ptr(&self) -> *mut *mut T {
        self.ptr.as_ptr()
    }

    pub(crate) fn get_type(&self) -> u32 {
        let ptr = self.ptr.get();
        unsafe {
            (*ptr).get_type()
        }
    }

    pub(crate) fn take(&self, env: *mut OCIEnv) -> Result<Self> {
        let new_handle = Self::alloc(env)?;
        let old_handle = self.ptr.replace(new_handle);
        Ok( Self::from(old_handle) )
    }

    // pub(crate) fn replace(&self, ptr: *mut T) {
    //     self.ptr.replace(ptr);
    // }

    pub(crate) fn get_attr<V: attr::AttrGet>(&self, attr_type: u32, err: *mut OCIError) -> Result<V> {
        let ptr = self.ptr.get();
        attr::get::<V>(attr_type, self.get_type(), ptr as *const c_void, err)
    }

    pub(crate) fn get_attr_into<V: attr::AttrGetInto>(&self, attr_type: u32, into: &mut V, err: *mut OCIError) -> Result<()> {
        let ptr = self.ptr.get();
        attr::get_into::<V>(attr_type, into, self.get_type(), ptr as *const c_void, err)
    }

    pub(crate) fn set_attr<V: attr::AttrSet>(&self, attr_type: u32, attr_val: V, err: *mut OCIError) -> Result<()> {
        let ptr = self.ptr.get();
        attr::set::<V>(attr_type, attr_val, self.get_type(), ptr as *mut c_void, err)
    }
}

impl Handle<OCIStmt> {}
