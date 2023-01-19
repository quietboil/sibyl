//! Variable-length RAW data

mod tosql;

use crate::{ Result, oci::{self, *} };

use super::Ctx;

pub(crate) fn new(size: u32, env: &OCIEnv, err: &OCIError) -> Result<Ptr<OCIRaw>> {
    let mut bin = Ptr::<OCIRaw>::null();
    oci::raw_resize(env, err, size, bin.as_mut_ptr())?;
    Ok( bin )
}

pub(crate) fn free(raw: &mut Ptr<OCIRaw>, env: &OCIEnv, err: &OCIError) {
    unsafe {
        OCIRawResize(env, err, 0, raw.as_mut_ptr());
    }
}

pub(crate) fn as_ptr(raw: &OCIRaw, env: &OCIEnv) -> *const u8 {
    unsafe {
        OCIRawPtr(env, raw)
    }
}

pub(crate) fn len(raw: &OCIRaw, env: &OCIEnv) -> usize {
    unsafe {
        OCIRawSize(env, raw) as usize
    }
}

/**
    Represents RAW and LONG RAW data types.

    The RAW datatype is used for binary data or byte strings that are not to be interpreted by Oracle.
    The maximum length of a RAW column is 2000 bytes.
    The LONG RAW datatype is similar to the RAW datatype, except that it stores raw data with a length up to two gigabytes.
*/
pub struct Raw<'a> {
    raw: Ptr<OCIRaw>,
    ctx: &'a dyn Ctx,
}

impl Drop for Raw<'_> {
    fn drop(&mut self) {
        free(&mut self.raw, self.ctx.as_ref(), self.ctx.as_ref());
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
    pub fn from_bytes(data: &[u8], ctx: &'a dyn Ctx) -> Result<Self> {
        let mut raw = Ptr::<OCIRaw>::null();
        oci::raw_assign_bytes(ctx.as_ref(), ctx.as_ref(), data.as_ptr(), data.len() as u32, raw.as_mut_ptr())?;
        Ok( Self { raw, ctx } )
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
        let mut raw = Ptr::<OCIRaw>::null();
        oci::raw_assign_raw(other.ctx.as_ref(), other.ctx.as_ref(), &other.raw, raw.as_mut_ptr())?;
        Ok( Self { raw, ..*other } )
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
    pub fn with_capacity(size: usize, ctx: &'a dyn Ctx) -> Result<Self> {
        let raw = new(size as u32, ctx.as_ref(), ctx.as_ref())?;
        Ok( Self { raw, ctx } )
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
        let mut size = 0u32;
        oci::raw_alloc_size(self.ctx.as_ref(), self.ctx.as_ref(), &self.raw, &mut size)?;
        Ok( size as usize )
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
        oci::raw_resize(self.ctx.as_ref(), self.ctx.as_ref(), new_size as u32, self.raw.as_mut_ptr())
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
        len(&self.raw, self.ctx.as_ref())
    }

    /**
        Sets the content of self to `data`

        # Example
        ```
        use sibyl::{ self as oracle, Raw };
        let env = oracle::env()?;

        let mut bin = Raw::with_capacity(10, &env)?;
        bin.set(&[0x41, 0x72, 0x65, 0x61, 0x20, 0x35, 0x31])?;

        assert_eq!(bin.len(), 7);
        assert_eq!(bin.as_bytes(), &[0x41, 0x72, 0x65, 0x61, 0x20, 0x35, 0x31]);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn set(&mut self, data: &[u8]) -> Result<()> {
        oci::raw_assign_bytes(self.ctx.as_ref(), self.ctx.as_ref(), data.as_ptr(), data.len() as u32, self.raw.as_mut_ptr())
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
        let ptr = as_ptr(&self.raw, self.ctx.as_ref());
        let len = len(&self.raw, self.ctx.as_ref());
        unsafe {
            std::slice::from_raw_parts(ptr, len)
        }
    }
}

impl std::fmt::Debug for Raw<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const MAX_LEN : usize = 50;
        let data = self.as_bytes();
        if data.len() > MAX_LEN {
            f.write_fmt(format_args!("RAW {:?}...", &data[..MAX_LEN]))
        } else {
            f.write_fmt(format_args!("RAW {:?}", data))
        }
    }
}
