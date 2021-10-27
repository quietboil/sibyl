//! OCI Parameter descriptor functions

use crate::{Result, catch};
use super::*;
use std::ptr;
use libc::c_void;

pub(crate) fn get<T>(pos: u32, obj_type: u32, obj: *const c_void, err: *mut OCIError) -> Result<Descriptor<T>>
    where T: DescriptorType
{
    let mut descr = ptr::null_mut::<T>();
    catch!{err =>
        OCIParamGet(obj, obj_type, err, &mut descr as *mut *mut T as *mut *mut c_void, pos)
    }
    Ok( Descriptor::from(descr) )
}
