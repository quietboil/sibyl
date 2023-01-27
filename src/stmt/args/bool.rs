use super::{Params, ToSql};
use crate::{oci::*, Result};
use std::mem::size_of;

impl ToSql for bool {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let boolean = *self as i32;
        // This is safe as `bin_in_mut` copies the data into the internal buffer and binds it there instead.
        params.bind_in_mut(pos, SQLT_BOL, &boolean as *const i32 as _, size_of::<i32>(), stmt, err)?;
        Ok(pos + 1)
    }
}

impl ToSql for &bool {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let boolean = **self as i32;
        params.bind_in_mut(pos, SQLT_BOL, &boolean as *const i32 as _, size_of::<i32>(), stmt, err)?;
        Ok(pos + 1)
    }
}
