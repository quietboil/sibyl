/// Implementation of traits that allow Raw values to be used as SQL parameters

use std::mem::size_of;
use crate::{oci::*, ToSql, ToSqlOut, Result, stmt::Params};
use super::Raw;

impl ToSql for Raw<'_> {
    fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let len = self.len() + size_of::<u32>();
        params.bind(pos, SQLT_LVB, self.raw.as_ptr() as _, len, stmt, err)?;
        Ok(pos + 1)
    }
}

impl ToSql for &Raw<'_> {
    fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let len = self.len() + size_of::<u32>();
        params.bind(pos, SQLT_LVB, self.raw.as_ptr() as _, len, stmt, err)?;
        Ok(pos + 1)
    }
}

impl ToSqlOut for &mut Raw<'_> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let len = self.len() + size_of::<u32>();
        let cap = self.capacity()? + size_of::<u32>();
        params.bind_out(pos, SQLT_LVB, self.raw.as_mut_ptr() as _, len, cap, stmt, err)?;
        Ok(pos + 1)
    }
}
