//! Fixed or variable-length string

mod tosql;

use crate::*;
use super::*;
use std::{ mem, ptr };

/// C mapping of the OCI String
#[repr(C)] pub struct OCIString { _private: [u8; 0] }

pub(crate) fn new(size: usize, env: *mut OCIEnv, err: *mut OCIError) -> Result<*mut OCIString> {
    let mut oci_str = ptr::null_mut::<OCIString>();
    catch!{err =>
        OCIStringResize(env, err, size as u32, &mut oci_str)
    }
    Ok( oci_str )
}

pub(crate) fn resize(txt: &mut *mut OCIString, size: usize, env: *mut OCIEnv, err: *mut OCIError) -> Result<()> {
    catch!{err =>
        OCIStringResize(env, err, size as u32, txt)
    }
    Ok(())
}

pub(crate) fn free(txt: &mut *mut OCIString, env: *mut OCIEnv, err: *mut OCIError) {
    unsafe {
        OCIStringResize(env, err, 0, txt);
    }
}

pub(crate) fn capacity(txt: *const OCIString, env: *mut OCIEnv, err: *mut OCIError) -> Result<usize> {
    let mut size = mem::MaybeUninit::<u32>::uninit();
    catch!{err =>
        OCIStringAllocSize(env, err, txt, size.as_mut_ptr())
    }
    Ok( unsafe { size.assume_init() } as usize )
}

pub(crate) fn to_string(txt: *const OCIString, env: *mut OCIEnv) -> String {
    let txt = unsafe {
        let ptr = OCIStringPtr(env, txt);
        let len = OCIStringSize(env, txt) as usize;
        std::slice::from_raw_parts(ptr, len)
    };
    String::from_utf8_lossy(txt).to_string()
}

pub(crate) fn raw_ptr(txt: *const OCIString, env: *mut OCIEnv) -> *mut u8 {
    unsafe {
        OCIStringPtr(env, txt)
    }
}

pub(crate) fn len(txt: *const OCIString, env: *mut OCIEnv) -> usize {
    unsafe {
        OCIStringSize(env, txt) as usize
    }
}

pub(crate) fn as_str<'a>(txt: *const OCIString, env: *mut OCIEnv) -> &'a str {
    unsafe {
        std::str::from_utf8_unchecked(
            std::slice::from_raw_parts(
                raw_ptr(txt, env),
                len(txt, env)
            )
        )
    }
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-string-functions.html#GUID-3F336010-D8C8-4B50-89CB-ABCCA98905DA
    fn OCIStringAllocSize(
        env:        *mut OCIEnv,
        err:        *mut OCIError,
        txt:        *const OCIString,
        size:       *mut u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-string-functions.html#GUID-58BC140A-900C-4409-B3D2-C2DC8FB643FF
    fn OCIStringAssign(
        env:        *mut OCIEnv,
        err:        *mut OCIError,
        rhs:        *const OCIString,
        lhs:        &*mut OCIString
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-string-functions.html#GUID-96E8375B-9017-4E06-BF85-09C12DF286F4
    fn OCIStringAssignText(
        env:        *mut OCIEnv,
        err:        *mut OCIError,
        rhs:        *const u8,
        rhs_len:    u32,
        lhs:        &*mut OCIString
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-string-functions.html#GUID-0E1302F7-A32C-46F1-93D7-FB33CF60C24F
    fn OCIStringPtr(
        env:        *mut OCIEnv,
        txt:        *const OCIString
    ) -> *mut u8;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-string-functions.html#GUID-CA52A8A4-08BA-4F08-A4A3-79F841F6AE9E
    fn OCIStringResize(
        env:        *mut OCIEnv,
        err:        *mut OCIError,
        size:       u32,
        txt:        &*mut OCIString
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-string-functions.html#GUID-DBDAB2D9-4E78-4752-85B6-55D30CA6AF30
    fn OCIStringSize(
        env:        *mut OCIEnv,
        txt:        *const OCIString
    ) -> u32;
}

/// Represents Oracle character types - VARCHAR, LONG, etc.
pub struct Varchar<'e> {
    txt: *mut OCIString,
    env: &'e dyn Env,
}

impl Drop for Varchar<'_> {
    fn drop(&mut self) {
        free(&mut self.txt, self.env.env_ptr(), self.env.err_ptr());
    }
}

impl<'e> Varchar<'e> {
    /**
        Returns a new Varchar constructed from the specified string

        # Example
        ```
        use sibyl::{ self as oracle, Varchar };
        let env = oracle::env()?;

        let txt = Varchar::from("Hello, World!", &env)?;

        assert!(txt.capacity()? >= 13);
        assert_eq!(txt.len(), 13);
        assert_eq!(txt.as_str(), "Hello, World!");
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn from(text: &str, env: &'e dyn Env) -> Result<Self> {
        let mut txt = ptr::null_mut::<OCIString>();
        catch!{env.err_ptr() =>
            OCIStringAssignText(env.env_ptr(), env.err_ptr(), text.as_ptr(), text.len() as u32, &mut txt)
        }
        Ok( Self { env, txt } )
    }

    /**
        Returns a new Varchar constructed with the copy of the date from the `other` Varchar.

        # Example
        ```
        use sibyl::{ self as oracle, Varchar };
        let env = oracle::env()?;

        let src = Varchar::from("Hello, World!", &env)?;
        let txt = Varchar::from_varchar(&src)?;

        assert_eq!(txt.len(), 13);
        assert_eq!(txt.as_str(), "Hello, World!");
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn from_varchar(other: &'e Varchar) -> Result<Self> {
        let env = other.env;
        let mut txt = ptr::null_mut::<OCIString>();
        catch!{env.err_ptr() =>
            OCIStringAssign(env.env_ptr(), env.err_ptr(), other.as_ptr(), &mut txt)
        }
        Ok( Self { env, txt } )
    }

    pub(crate) fn from_ocistring(oci_str: *const OCIString, env: &'e dyn Env) -> Result<Self> {
        let mut txt = ptr::null_mut::<OCIString>();
        catch!{env.err_ptr() =>
            OCIStringAssign(env.env_ptr(), env.err_ptr(), oci_str, &mut txt)
        }
        Ok( Self { env, txt } )
    }

    /**
        Returns a new Varchar with the memory allocated for the txt data.

        # Example
        ```
        use sibyl::{ self as oracle, Varchar };
        let env = oracle::env()?;

        let txt = Varchar::with_capacity(19, &env)?;

        assert!(txt.capacity()? >= 19);
        assert_eq!(txt.len(), 0);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn with_capacity(size: usize, env: &'e dyn Env) -> Result<Self> {
        let txt = new(size, env.env_ptr(), env.err_ptr())?;
        Ok( Self { env, txt } )
    }

    pub(crate) fn as_ptr(&self) -> *const OCIString {
        self.txt
    }

    pub(crate) fn as_mut_ptr(&self) -> *mut OCIString {
        self.txt
    }

    /**
        Sets the content of self to `text`

        # Example
        ```
        use sibyl::{ self as oracle, Varchar };
        let env = oracle::env()?;

        let mut txt = Varchar::with_capacity(0, &env)?;
        txt.set("Hello, World!")?;

        assert_eq!(txt.len(), 13);
        assert_eq!(txt.as_str(), "Hello, World!");
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn set(&mut self, text: &str) -> Result<()> {
        catch!{self.env.err_ptr() =>
            OCIStringAssignText(self.env.env_ptr(), self.env.err_ptr(), text.as_ptr(), text.len() as u32, &mut self.txt)
        }
        Ok(())
    }

    /**
        Returns the size of the string in bytes.

        # Example
        ```
        use sibyl::{ self as oracle, Varchar };
        let env = oracle::env()?;

        let mut txt = Varchar::from("🚲🛠📬🎓", &env)?;

        assert_eq!(txt.len(), 16);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn len(&self) -> usize {
        len(self.as_ptr(), self.env.env_ptr())
    }

    /**
        Returns the allocated size of string memory in bytes

        # Example
        ```
        use sibyl::{ self as oracle, Varchar };
        let env = oracle::env()?;

        let mut txt = Varchar::from("🚲🛠📬🎓", &env)?;

        assert!(txt.capacity()? >= 16);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn capacity(&self) -> Result<usize> {
        capacity(self.as_ptr(), self.env.env_ptr(), self.env.err_ptr())
    }

    /**
        Changes the size of the memory of a string in the object cache.
        Content of the string is not preserved.

        # Example
        ```
        use sibyl::{ self as oracle, Varchar };
        let env = oracle::env()?;

        let mut txt = Varchar::with_capacity(10, &env)?;
        assert!(txt.capacity()? >= 10);

        txt.resize(20)?;
        assert!(txt.capacity()? >= 20);

        txt.resize(0)?;
        // Cannot not ask for capacity after resize to 0.
        // Yes, it works for Raw, but not for Varchars
        let res = txt.capacity();
        assert!(res.is_err());
        if let Err( sibyl::Error::Oracle(code, _message) ) = res {
            assert_eq!(code, 21500);
        } else {
            panic!("cannot match the error");
        }

        txt.resize(16)?;
        assert!(txt.capacity()? >= 16);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn resize(&mut self, new_size: usize) -> Result<()> {
        catch!{self.env.err_ptr() =>
            OCIStringResize(self.env.env_ptr(), self.env.err_ptr(), new_size as u32, &mut self.txt)
        }
        Ok(())
    }

    /**
        Extracts a string slice containing the entire content of the VARCHAR.

        # Example
        ```
        use sibyl::{ self as oracle, Varchar };
        let env = oracle::env()?;

        let txt = Varchar::from("Hello, World!", &env)?;

        assert_eq!(txt.as_str(), "Hello, World!");
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn as_str(&self) -> &str {
        as_str(self.txt, self.env.env_ptr())
    }

    /// Returns unsafe pointer to the string data
    pub fn as_raw_ptr(&self) -> *mut u8 {
        raw_ptr(self.as_ptr(), self.env.env_ptr())
    }
}

impl std::fmt::Debug for Varchar<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const MAX_LEN : usize = 50;
        let len = self.len();
        if len == 0 {
            f.write_str("VARCHAR ''")
        } else if len <= MAX_LEN {
            f.write_fmt(format_args!("VARCHAR '{}'", self.as_str()))
        } else {
            f.write_fmt(format_args!("VARCHAR '{}...'", &self.as_str()[..MAX_LEN]))
        }
    }
}

impl std::string::ToString for Varchar<'_> {
    fn to_string(&self) -> String {
        to_string(self.as_ptr(), self.env.env_ptr())
    }
}
