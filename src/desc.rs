use crate::*;
use libc::{ c_void, size_t };
use std::{ ptr, cell::Cell };

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/handle-and-descriptor-functions.html#GUID-E9EF2766-E078-49A7-B1D1-738E4BA4814F
    fn OCIDescriptorAlloc(
        parenth:    *mut OCIEnv,
        descpp:     *mut *mut  c_void,
        desc_type:  u32,
        xtramem_sz: size_t,
        usrmempp:   *const c_void
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/handle-and-descriptor-functions.html#GUID-A32BF051-3DC1-491C-AAFD-A46034DD1629
    fn OCIDescriptorFree(
        descp:      *mut c_void,
        desc_type:  u32
    ) -> i32;
}

pub trait DescriptorType {
    type OCIType;
    fn get_type(&self) -> u32;
}

pub struct Descriptor<T: DescriptorType> {
    ptr: Cell<*mut T>,
}

macro_rules! impl_descr_type {
    ($($oci_desc:ident => $id:ident, $ret:ident),+) => {
        $(
            impl DescriptorType for $oci_desc {
                type OCIType = $ret;
                fn get_type(&self) -> u32 {
                    $id
                }
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

impl<T: DescriptorType> Drop for Descriptor<T> {
    fn drop(&mut self) {
        let ptr = self.ptr.get();
        if !ptr.is_null() {
            unsafe {
                OCIDescriptorFree(ptr as *mut c_void, self.get_type());
            }
        }
    }
}

impl<T: DescriptorType> Descriptor<T> {
    fn alloc(env: *mut OCIEnv) -> Result<*mut T> {
        let mut desc = ptr::null_mut::<T>();
        let desc_type = unsafe {
            (*desc).get_type()
        };
        let res = unsafe {
            OCIDescriptorAlloc(env, &mut desc as *mut *mut T as *mut *mut c_void, desc_type, 0, ptr::null())
        };
        if res != OCI_SUCCESS {
            Err( Error::env(env, res) )
        } else if desc.is_null() {
            Err( Error::new(&format!("OCI returned NULL for descriptor {}", desc_type)) )
        } else {
            Ok( desc )
        }
    }

    pub(crate) fn new(env: *mut OCIEnv) -> Result<Self> {
        let desc = Self::alloc(env)?;
        Ok( Self { ptr: Cell::new(desc) } )
    }

    pub(crate) fn from(ptr: *mut T) -> Self {
        Self { ptr: Cell::new(ptr) }
    }

    pub(crate) fn get(&self) -> *mut T::OCIType {
        self.ptr.get() as *mut T::OCIType
    }

    pub(crate) fn as_ptr(&self) -> *mut *mut T::OCIType {
        self.ptr.as_ptr() as *mut *mut T::OCIType
    }

    pub(crate)fn replace(&self, ptr: *mut T) {
        self.ptr.replace(ptr);
    }

    pub(crate) fn take(&self, env: *mut OCIEnv) -> Result<Self> {
        let new_oci_desc = Self::alloc(env)?;
        let old_oci_desc = self.ptr.replace(new_oci_desc);
        Ok( Self::from(old_oci_desc) )
    }

    pub(crate) fn get_type(&self) -> u32 {
        let ptr = self.ptr.get();
        unsafe {
            (*ptr).get_type()
        }
    }

    pub(crate) fn get_attr<V: attr::AttrGet>(&self, attr_type: u32, err: *mut OCIError) -> Result<V> {
        attr::get::<V>(attr_type, self.get_type(), self.get() as *const c_void, err)
    }

    pub(crate) fn set_attr<V: attr::AttrSet>(&self, attr_type: u32, attr_val: V, err: *mut OCIError) -> Result<()> {
        attr::set::<V>(attr_type, attr_val, self.get_type(), self.get() as *mut c_void, err)
    }
}
