//! OCI Raw functions to manipulate variable-length RAW

use crate::*;
use super::*;
use libc::c_void;
use std::{ mem, ptr };

/// C mapping of the Oracle RAW
#[repr(C)] pub struct OCIRaw { _private: [u8; 0] }

pub(crate) fn new(size: usize, env: *mut OCIEnv, err: *mut OCIError) -> Result<*mut OCIRaw> {
    let mut bin = ptr::null_mut::<OCIRaw>();
    catch!{err =>
        OCIRawResize(env, err, size as u32, &mut bin)
    }
    Ok( bin )
}

pub(crate) fn free(raw: &mut *mut OCIRaw, env: *mut OCIEnv, err: *mut OCIError) {
    unsafe {
        OCIRawResize(env, err, 0, raw);
    }
}

pub(crate) fn as_raw_ptr(raw: *const OCIRaw, env: *mut OCIEnv) -> *mut u8 {
    unsafe {
        OCIRawPtr(env, raw)
    }
}

pub(crate) fn len(raw: *const OCIRaw, env: *mut OCIEnv) -> usize {
    unsafe {
        OCIRawSize(env, raw) as usize
    }
}

pub(crate) fn as_bytes(raw: *const OCIRaw, usrenv: &dyn UsrEnv) -> &[u8] {
    let ptr = as_raw_ptr(raw, usrenv.env_ptr());
    let len = len(raw, usrenv.env_ptr());
    unsafe {
        std::slice::from_raw_parts(ptr, len)
    }
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-raw-functions.html#GUID-4856A258-8883-4470-9881-51F27FA050F6
    fn OCIRawAllocSize(
        env:        *mut OCIEnv,
        err:        *mut OCIError,
        raw:        *const OCIRaw,
        size:       *mut u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-raw-functions.html#GUID-3BB4239F-8579-4CC1-B76F-0786BDBAEF9A
    fn OCIRawAssignBytes(
        env:        *mut OCIEnv,
        err:        *mut OCIError,
        rhs:        *const u8,
        rhs_len:    u32,
        lhs:        &*mut OCIRaw
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-raw-functions.html#GUID-27DBFBE0-4511-4B34-8476-B9AC720E3F51
    fn OCIRawAssignRaw(
        env:        *mut OCIEnv,
        err:        *mut OCIError,
        rhs:        *const OCIRaw,
        lhs:        &*mut OCIRaw
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-raw-functions.html#GUID-B05C44C5-7168-438B-AC2A-BD3AD309AAEA
    fn OCIRawPtr(
        env:        *mut OCIEnv,
        raw:        *const OCIRaw
    ) -> *mut u8;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-raw-functions.html#GUID-7D757B00-DF25-4F61-A3DF-8C72F18FDC9E
    fn OCIRawResize(
        env:        *mut OCIEnv,
        err:        *mut OCIError,
        size:       u32,
        raw:        &*mut OCIRaw
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-raw-functions.html#GUID-D74E75FA-5985-4DDC-BC25-430B415B8837
    fn OCIRawSize(
        env:        *mut OCIEnv,
        raw:        *const OCIRaw
    ) -> u32;
}

/// Represents RAW and LONG RAW data types.
pub struct Raw<'e> {
    raw: *mut OCIRaw,
    env: &'e dyn Env,
}

impl Drop for Raw<'_> {
    fn drop(&mut self) {
        free(&mut self.raw, self.env.env_ptr(), self.env.err_ptr());
    }
}

impl<'e> Raw<'e> {
    /// Returns a new Raw constructed with the copy of the `data`
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let data: [u8;5] = [1,2,3,4,5];
    /// let raw = oracle::Raw::from_bytes(&data, &env)?;
    ///
    /// let size = raw.capacity()?;
    /// assert!(5 <= size);
    ///
    /// let len = raw.len();
    /// assert_eq!(5, len);
    ///
    /// let raw_data_ptr = raw.as_raw_ptr();
    /// assert!(raw_data_ptr != std::ptr::null_mut::<u8>());
    ///
    /// let raw_data: &[u8] = unsafe { std::slice::from_raw_parts(raw_data_ptr, len as usize) };
    /// assert_eq!(data, raw_data);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn from_bytes(data: &[u8], env: &'e dyn Env) -> Result<Self> {
        let mut raw = ptr::null_mut::<OCIRaw>();
        catch!{env.err_ptr() =>
            OCIRawAssignBytes(env.env_ptr(), env.err_ptr(), data.as_ptr(), data.len() as u32, &mut raw)
        }
        Ok( Self { env, raw } )
    }

    /// Returns a new Raw constructed with the copy of the date from the `other` Raw.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let data: [u8;5] = [1,2,3,4,5];
    /// let src = oracle::Raw::from_bytes(&data, &env)?;
    /// let dst = oracle::Raw::from_raw(&src)?;
    ///
    /// let raw_data_ptr = src.as_raw_ptr();
    /// assert!(raw_data_ptr != std::ptr::null_mut::<u8>());
    ///
    /// let len = src.len();
    /// assert_eq!(5, len);
    ///
    /// let src_data: &[u8] = unsafe { std::slice::from_raw_parts(raw_data_ptr, len as usize) };
    /// let raw_data_ptr = dst.as_raw_ptr();
    /// assert!(raw_data_ptr != std::ptr::null_mut::<u8>());
    ///
    /// let len = dst.len();
    /// assert_eq!(5, len);
    ///
    /// let dst_data: &[u8] = unsafe { std::slice::from_raw_parts(raw_data_ptr, len as usize) };
    /// assert_eq!(dst_data, src_data);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn from_raw(other: &Raw<'e>) -> Result<Self> {
        let env = other.env;
        let mut raw = ptr::null_mut::<OCIRaw>();
        catch!{env.err_ptr() =>
            OCIRawAssignRaw(env.env_ptr(), env.err_ptr(), other.as_ptr(), &mut raw)
        }
        Ok( Self { env, raw } )
    }

    /// Returns a new Raw with the memory allocated for the raw data.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let raw = oracle::Raw::with_capacity(19, &env)?;
    ///
    /// let size = raw.capacity()?;
    /// assert!(19 <= size);
    ///
    /// let len = raw.len();
    /// assert_eq!(0, len);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn with_capacity(size: usize, env: &'e dyn Env) -> Result<Self> {
        let raw = new(size, env.env_ptr(), env.err_ptr())?;
        Ok( Self { env, raw } )
    }

    pub(crate) fn as_ptr(&self) -> *const OCIRaw {
        self.raw
    }

    pub(crate) fn as_mut_ptr(&self) -> *mut OCIRaw {
        self.raw
    }

    /// Returns the size of the raw data in bytes.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let data: [u8;5] = [1,2,3,4,5];
    /// let raw = oracle::Raw::from_bytes(&data, &env)?;
    /// let len = raw.len();
    ///
    /// assert_eq!(5, len);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn len(&self) -> usize {
        len(self.as_ptr(), self.env.env_ptr())
    }

    /// Returns the allocated size of raw memory in bytes
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let raw = oracle::Raw::with_capacity(19, &env)?;
    /// let size = raw.capacity()?;
    ///
    /// assert!(19 <= size);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn capacity(&self) -> Result<usize> {
        let mut size: u32;
        catch!{self.env.err_ptr() =>
            size = mem::uninitialized();
            OCIRawAllocSize(self.env.env_ptr(), self.env.err_ptr(), self.as_ptr(), &mut size)
        }
        Ok( size as usize )
    }

    /// Changes the size of the memory of this raw binary in the object cache.
    /// Previous content is not preserved.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let mut bin = oracle::Raw::with_capacity(10, &env)?;
    /// let cap = bin.capacity()?;
    /// assert!(cap >= 10);
    ///
    /// bin.resize(20);
    /// let cap = bin.capacity()?;
    /// assert!(cap >= 20);
    ///
    /// bin.resize(0);
    /// let cap = bin.capacity()?;
    /// assert_eq!(0, cap);
    ///
    /// bin.resize(16);
    /// let cap = bin.capacity()?;
    /// assert!(cap >= 16);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn resize(&mut self, new_size: usize) -> Result<()> {
        catch!{self.env.err_ptr() =>
            OCIRawResize(self.env.env_ptr(), self.env.err_ptr(), new_size as u32, &mut self.raw)
        }
        Ok(())
    }

    /// Returns unsafe pointer to the RAW data
     /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let data: [u8;5] = [1,2,3,4,5];
    /// let raw = oracle::Raw::from_bytes(&data, &env)?;
    ///
    /// let raw_data_ptr = raw.as_raw_ptr();
    /// assert!(raw_data_ptr != std::ptr::null_mut::<u8>());
    ///
    /// let raw_data: &[u8] = unsafe { std::slice::from_raw_parts(raw_data_ptr, raw.len() as usize) };
    /// assert_eq!(data, raw_data);
    /// # Ok::<(),oracle::Error>(())
    /// ```
   pub fn as_raw_ptr(&self) -> *mut u8 {
       as_raw_ptr(self.as_ptr(), self.env.env_ptr())
    }
}

impl ToSql for Raw<'_> {
    fn to_sql(&self) -> (u16, *const c_void, usize) {
        ( SQLT_LVB, self.as_ptr() as *const c_void, self.len() + std::mem::size_of::<u32>() )
    }
}

impl ToSqlOut for Raw<'_> {
    fn to_sql_output(&mut self, _col_size: usize) -> (u16, *mut c_void, usize) {
        (SQLT_LVB, self.as_mut_ptr() as *mut c_void, self.capacity().ok().unwrap_or_default() + std::mem::size_of::<u32>())
    }
}

impl ToSqlOut for *mut OCIRaw {
    fn to_sql_output(&mut self, col_size: usize) -> (u16, *mut c_void, usize) {
        (SQLT_LVB, *self as *mut c_void, col_size + std::mem::size_of::<u32>())
    }
}
