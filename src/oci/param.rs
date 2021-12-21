//! OCI Parameter descriptor functions

use crate::{Result, oci};
use super::{*, desc::Descriptor};

pub(crate) fn get(pos: u32, obj_type: u32, obj: &OCIStmt, err: &OCIError) -> Result<Descriptor<OCIParam>> {
    let mut descr = Ptr::<OCIParam>::null();
    oci::param_get(obj, obj_type, err, descr.as_mut_ptr(), pos)?;
    Ok( Descriptor::from(descr) )
}
