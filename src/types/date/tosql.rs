/// Implementation of traits that allow Dates to be used as SQL parameters

use std::mem::size_of;
use crate::{oci::*, ToSql, ToSqlOut, Result, stmt::Params};
use super::Date;

impl ToSql for OCIDate {
    fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind(pos, SQLT_ODT, self as *const OCIDate as _, size_of::<OCIDate>(), stmt, err)?;
        Ok(pos + 1)
    }
}

impl ToSql for Date<'_> {
    fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        self.date.bind_to(pos, params, stmt, err)
    }
}

impl ToSql for &Date<'_> {
    fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        self.date.bind_to(pos, params, stmt, err)
    }
}

impl ToSqlOut for OCIDate {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind_out(pos, SQLT_ODT, self as *mut OCIDate as _, size_of::<OCIDate>(), size_of::<OCIDate>(), stmt, err)?;
        Ok(pos + 1)
    }
}

impl ToSqlOut for &mut Date<'_> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        self.date.bind_to(pos, params, stmt, err)
    }
}
