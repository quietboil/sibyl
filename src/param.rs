use crate::*;
use crate::desc::{ Descriptor, DescriptorType };
use libc::c_void;
use std::ptr;

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/handle-and-descriptor-functions.html#GUID-35D2FF91-139B-4A5C-97C8-8BC29866CCA4
    fn OCIParamGet(
        hndlp:      *const c_void,
        htype:      u32,
        errhp:      *mut OCIError,
        descr:      *mut *mut c_void,
        pos:        u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/handle-and-descriptor-functions.html#GUID-280CF9E5-3537-4785-9AFA-4E63DE29A266
    // fn OCIParamSet(
    //     hndlp:      *const c_void,
    //     htype:      u32,
    //     errhp:      *mut OCIError,
    //     descr:      *const c_void,
    //     dtype:      u32,
    //     pos:        u32
    // ) -> i32;
}

pub(crate) fn get<T>(pos: u32, obj_type: u32, obj: *const c_void, err: *mut OCIError) -> Result<Descriptor<T>>
    where T: DescriptorType
{
    let mut descr = ptr::null_mut::<T>();
    catch!{err =>
        OCIParamGet(obj, obj_type, err, &mut descr as *mut *mut T as *mut *mut c_void, pos)
    }
    Ok( Descriptor::from(descr) )
}
