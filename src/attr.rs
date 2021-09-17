use crate::*;
use libc::c_void;
use std::mem;

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/handle-and-descriptor-functions.html#GUID-FA199A99-4D7A-42C2-BB0A-C20047B95DF9
    fn OCIAttrGet(
        trgthndlp:  *const c_void,
        trghndltyp: u32,
        attributep: *mut c_void,
        sizep:      *mut u32,
        attrtype:   u32,
        errhp:      *mut OCIError
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/handle-and-descriptor-functions.html#GUID-3741D7BD-7652-4D7A-8813-AC2AEA8D3B03
    fn OCIAttrSet(
        trgthndlp:  *mut c_void,
        trghndltyp: u32,
        attributep: *const c_void,
        size:       u32,
        attrtype:   u32,
        errhp:      *mut OCIError
    ) -> i32;
}

pub(crate) fn get<T: AttrGet>(attr_type: u32, obj_type: u32, obj: *const c_void, err: *mut OCIError) -> Result<T> {
    let mut attr_val  = mem::MaybeUninit::<T::ValueType>::uninit();
    let mut attr_size = mem::MaybeUninit::<u32>::uninit();
    catch!{err =>
        OCIAttrGet(
            obj, obj_type,
            attr_val.as_mut_ptr() as *mut c_void, attr_size.as_mut_ptr(), attr_type,
            err
        )
    }
    Ok( AttrGet::new( unsafe { attr_val.assume_init() }, unsafe { attr_size.assume_init() } as usize) )
}

pub(crate) fn get_into<T: AttrGetInto>(attr_type: u32, into: &mut T, obj_type: u32, obj: *const c_void, err: *mut OCIError) -> Result<()> {
    let mut size = into.capacity() as u32;
    catch!{err =>
        OCIAttrGet(
            obj, obj_type,
            into.as_val_ptr(), &mut size, attr_type,
            err
        )
    }
    into.set_len(size as usize);
    Ok(())
}

pub(crate) fn set<T: AttrSet>(attr_type: u32, attr_val: T, obj_type: u32, obj: *mut c_void, err: *mut OCIError) -> Result<()> {
    catch!{err =>
        OCIAttrSet(obj, obj_type, attr_val.as_ptr(), attr_val.len() as u32, attr_type, err)
    }
    Ok(())
}

pub(crate) trait AttrGet {
    type ValueType;
    fn new(val: Self::ValueType, len: usize) -> Self;
}

pub(crate) trait AttrGetInto {
    fn as_val_ptr(&mut self) -> *mut c_void;
    fn capacity(&self) -> usize;
    fn set_len(&mut self, new_len: usize);
}

pub(crate) trait AttrSet {
    fn as_ptr(&self) -> *const c_void;
    fn len(&self) -> usize;
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
                    self as *const $t as *const c_void
                }
                fn len(&self) -> usize {
                    0
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
                    *self as *const $t as *const c_void
                }
                fn len(&self) -> usize {
                    0
                }
            }
        )+
    };
}

impl_oci_handle_attr!{ OCIServer, OCISession }

impl AttrGet for &str {
    type ValueType = *const u8;
    fn new(ptr: *const u8, len: usize) -> Self {
        unsafe { std::str::from_utf8_unchecked( std::slice::from_raw_parts(ptr, len) ) }
    }
}

impl AttrSet for &str {
    fn as_ptr(&self) -> *const c_void {
        (*self).as_ptr() as *const c_void
    }
    fn len(&self) -> usize {
        (*self).len()
    }
}

impl AttrGetInto for RowID {
    fn as_val_ptr(&mut self) -> *mut c_void { self.get() as *mut c_void }
    fn capacity(&self) -> usize             { 0 }
    fn set_len(&mut self, _new_len: usize)  { }
}

impl AttrGetInto for String {
    fn as_val_ptr(&mut self) -> *mut c_void { unsafe { self.as_mut_vec().as_mut_ptr() as *mut c_void } }
    fn capacity(&self) -> usize             { self.capacity() }
    fn set_len(&mut self, new_len: usize)   { unsafe { self.as_mut_vec().set_len(new_len) } }
}