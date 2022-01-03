/// Implementation of traits that allow Numbers to be used as SQL parameters

use std::mem::size_of;
use crate::{oci::*, ToSql, ToSqlOut, Result, stmt::Params};
use super::Number;

impl ToSql for OCINumber {
    fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind(pos, SQLT_VNU, self as *const OCINumber as _, size_of::<OCINumber>(), stmt, err)?;
        Ok(pos + 1)
    }
}

impl ToSql for Number<'_> {
    fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        self.num.bind_to(pos, params, stmt, err)
    }
}

impl ToSql for &Number<'_> {
    fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        self.num.bind_to(pos, params, stmt, err)
    }
}

impl ToSqlOut for OCINumber {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind_out(pos, SQLT_VNU, self as *mut OCINumber as _, size_of::<OCINumber>(), size_of::<OCINumber>(), stmt, err)?;
        Ok(pos + 1)
    }
}

impl ToSqlOut for &mut Number<'_> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        self.num.bind_to(pos, params, stmt, err)
    }
}
