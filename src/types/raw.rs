//! OCI Raw functions to manipulate variable-length RAW

mod tosql;

use crate::*;
use super::*;
use libc::c_void;
use std::{ mem, ptr };

/// C mapping of the Oracle RAW
#[repr(C)] pub struct OCIRaw { _private: [u8; 0] }

pub(crate) fn new(size: u32, env: *mut OCIEnv, err: *mut OCIError) -> Result<*mut OCIRaw> {
    let mut bin = ptr::null_mut::<OCIRaw>();
    catch!{err =>
        OCIRawResize(env, err, size, &mut bin)
    }
    Ok( bin )
}

// pub(crate) fn resize(bin: &mut *mut OCIRaw, size: usize, env: *mut OCIEnv, err: *mut OCIError) -> Result<()> {
//     catch!{err =>
//         OCIRawResize(env, err, size as u32, bin)
//     }
//     Ok(())
// }

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

pub(crate) fn as_bytes<'a>(raw: *const OCIRaw, env: *mut OCIEnv) -> &'a [u8] {
    let ptr = as_raw_ptr(raw, env);
    let len = len(raw, env);
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
        raw:        &mut *mut OCIRaw
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-raw-functions.html#GUID-D74E75FA-5985-4DDC-BC25-430B415B8837
    fn OCIRawSize(
        env:        *mut OCIEnv,
        raw:        *const OCIRaw
    ) -> u32;
}

/**
    Represents RAW and LONG RAW data types.

    The RAW datatype is used for binary data or byte strings that are not to be interpreted by Oracle.
    The maximum length of a RAW column is 2000 bytes.
    The LONG RAW datatype is similar to the RAW datatype, except that it stores raw data with a length up to two gigabytes.
*/
pub struct Raw<'a> {
    raw: *mut OCIRaw,
    env: &'a dyn Env,
}

impl Drop for Raw<'_> {
    fn drop(&mut self) {
        free(&mut self.raw, self.env.env_ptr(), self.env.err_ptr());
    }
}

impl<'a> Raw<'a> {
    /**
        Returns a new Raw constructed with the copy of the `data`

        # Example
        ```
        use sibyl::{ self as oracle, Raw };
        let env = oracle::env()?;

        let raw = Raw::from_bytes(&[1u8,2,3,4,5], &env)?;

        assert!(raw.capacity()? >= 5);
        assert_eq!(raw.len(), 5);
        assert_eq!(raw.as_bytes(), &[1u8,2,3,4,5]);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn from_bytes(data: &[u8], env: &'a dyn Env) -> Result<Self> {
        let mut raw = ptr::null_mut::<OCIRaw>();
        catch!{env.err_ptr() =>
            OCIRawAssignBytes(env.env_ptr(), env.err_ptr(), data.as_ptr(), data.len() as u32, &mut raw)
        }
        Ok( Self { env, raw } )
    }

    /**
        Returns a new Raw constructed with the copy of the date from the `other` Raw.

        # Example
        ```
        use sibyl::{ self as oracle, Raw };
        let env = oracle::env()?;

        let src = Raw::from_bytes(&[1u8,2,3,4,5], &env)?;
        let dst = Raw::from_raw(&src)?;

        assert_eq!(dst.len(), 5);
        assert_eq!(dst.as_bytes(), &[1u8,2,3,4,5]);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn from_raw(other: &Raw<'a>) -> Result<Self> {
        let env = other.env;
        let mut raw = ptr::null_mut::<OCIRaw>();
        catch!{env.err_ptr() =>
            OCIRawAssignRaw(env.env_ptr(), env.err_ptr(), other.as_ptr(), &mut raw)
        }
        Ok( Self { env, raw } )
    }

    /**
        Returns a new Raw with the memory allocated for the raw data.

        # Example
        ```
        use sibyl::{ self as oracle, Raw };
        let env = oracle::env()?;

        let raw = Raw::with_capacity(19, &env)?;

        assert!(raw.capacity()? >= 19);
        assert_eq!(raw.len(), 0);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn with_capacity(size: usize, env: &'a dyn Env) -> Result<Self> {
        let raw = new(size as u32, env.env_ptr(), env.err_ptr())?;
        Ok( Self { env, raw } )
    }

    /**
        Returns the allocated size of raw memory in bytes

        # Example
        ```
        use sibyl::{ self as oracle, Raw };
        let env = oracle::env()?;

        let raw = Raw::with_capacity(19, &env)?;

        assert!(raw.capacity()? >= 19);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn capacity(&self) -> Result<usize> {
        let mut size = mem::MaybeUninit::<u32>::uninit();
        catch!{self.env.err_ptr() =>
            OCIRawAllocSize(self.env.env_ptr(), self.env.err_ptr(), self.as_ptr(), size.as_mut_ptr())
        }
        Ok( unsafe { size.assume_init() } as usize )
    }

    /**
        Changes the size of the memory of this raw binary in the object cache.
        Previous content is not preserved.

        # Example
        ```
        use sibyl::{ self as oracle, Raw };
        let env = oracle::env()?;

        let mut bin = Raw::with_capacity(10, &env)?;
        assert!(bin.capacity()? >= 10);

        bin.resize(20);
        assert!(bin.capacity()? >= 20);

        bin.resize(0);
        assert_eq!(bin.capacity()?, 0);

        bin.resize(16);
        assert!(bin.capacity()? >= 16);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn resize(&mut self, new_size: usize) -> Result<()> {
        catch!{self.env.err_ptr() =>
            OCIRawResize(self.env.env_ptr(), self.env.err_ptr(), new_size as u32, &mut self.raw)
        }
        Ok(())
    }

    pub(crate) fn as_ptr(&self) -> *const OCIRaw {
        self.raw
    }

    pub(crate) fn as_mut_ptr(&self) -> *mut OCIRaw {
        self.raw
    }

    /**
        Returns the size of the raw data in bytes.

        # Example
        ```
        use sibyl::{ self as oracle, Raw };
        let env = oracle::env()?;

        let raw = Raw::from_bytes(&[1u8,2,3,4,5], &env)?;

        assert_eq!(raw.len(), 5);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn len(&self) -> usize {
        len(self.as_ptr(), self.env.env_ptr())
    }

    /**
        Returns a byte slice of this Raw’s contents.

        # Example
        ```
        use sibyl::{ self as oracle, Raw };
        let env = oracle::env()?;

        let raw = Raw::from_bytes(&[1u8,2,3,4,5], &env)?;

        assert_eq!(raw.as_bytes(), &[1u8,2,3,4,5]);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn as_bytes(&self) -> &[u8] {
        as_bytes(self.as_ptr(), self.env.env_ptr())
    }
}

impl std::fmt::Debug for Raw<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const MAX_LEN : usize = 50;
        let data = self.as_bytes();
        if data.len() > MAX_LEN {
            f.write_fmt(format_args!("RAW {:?}...", &data[..MAX_LEN]))
        } else {
            f.write_fmt(format_args!("RAW {:?}...", data))
        }
    }
}
