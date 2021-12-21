use crate::{Result, oci};
use super::*;
use libc::c_void;
use std::mem;

pub(crate) trait AttrGet {
    type ValueType;
    fn new(val: Self::ValueType, len: usize) -> Self;
}

pub(crate) trait AttrSet {
    fn as_ptr(&self) -> *const c_void;
    fn len(&self) -> usize { 0 }
}

pub(crate) trait AttrGetInto {
    fn as_mut_ptr(&mut self) -> *mut c_void;
    fn capacity(&self) -> usize { 0 }
    fn set_len(&mut self, _new_len: usize) {}
}

pub(crate) fn get<O, A>(attr_type: u32, obj_type: u32, obj: &O, err: &OCIError) -> Result<A> 
where O: OCIStruct
    , A: AttrGet
{
    let mut attr_val  = mem::MaybeUninit::<A::ValueType>::uninit();
    let mut attr_size = 0u32;
    oci::attr_get(obj, obj_type, attr_val.as_mut_ptr() as _, &mut attr_size, attr_type, err)?;
    Ok( AttrGet::new( unsafe { attr_val.assume_init() }, attr_size as usize) )
}

pub(crate) fn get_into<O, A>(attr_type: u32, into: &mut A, obj_type: u32, obj: &O, err: &OCIError) -> Result<()> 
where O: OCIStruct
    , A: AttrGetInto
{
    let mut size = into.capacity() as u32;
    oci::attr_get(obj, obj_type, into.as_mut_ptr(), &mut size, attr_type, err)?;
    into.set_len(size as usize);
    Ok(())
}

pub(crate) fn set<O, A>(attr_type: u32, attr_val: A, obj_type: u32, obj: &O, err: &OCIError) -> Result<()> 
where O: OCIStruct
    , A: AttrSet
{
    oci::attr_set(obj, obj_type, attr_val.as_ptr(), attr_val.len() as u32, attr_type, err)
}

macro_rules! impl_int_attr {
    ($($t:ty),+) => {
        $(
            impl AttrGet for $t {
                type ValueType = $t;
                fn new(val: $t, _len: usize) -> Self {
                    val
                }
            }
            impl AttrSet for $t {
                fn as_ptr(&self) -> *const c_void {
                    self as *const $t as _
                }
            }
        )+
    };
}

impl_int_attr!{ u8, i8, u16, i16, u32, u64 }

macro_rules! impl_oci_handle_attr {
    ($($t:ty),+) => {
        $(
            impl AttrSet for *mut $t {
                fn as_ptr(&self) -> *const c_void {
                    *self as *const $t as _
                }
            }
        )+
    };
}

impl_oci_handle_attr!{ OCIServer, OCISession, OCIAuthInfo }

impl AttrGet for &str {
    type ValueType = *const u8;
    fn new(ptr: *const u8, len: usize) -> Self {
        unsafe {
            std::str::from_utf8_unchecked(
                std::slice::from_raw_parts(ptr, len)
            )
        }
    }
}

impl AttrSet for &str {
    fn as_ptr(&self) -> *const c_void {
        (*self).as_ptr() as _
    }
    fn len(&self) -> usize {
        (*self).len()
    }
}

impl AttrGetInto for String {
    fn as_mut_ptr(&mut self) -> *mut c_void { unsafe { self.as_mut_vec().as_mut_ptr() as _ } }
    fn capacity(&self) -> usize             { self.capacity() }
    fn set_len(&mut self, new_len: usize)   { unsafe { self.as_mut_vec().set_len(new_len) } }
}
