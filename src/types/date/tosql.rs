/// Implementation of traits that allow Dates to be used as SQL parameters

use std::mem::size_of;
use crate::types::OracleDataType;
use crate::{oci::*, ToSql, Result, stmt::Params};
use super::Date;

impl ToSql for OCIDate {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind(pos, SQLT_ODT, self as *const OCIDate as _, size_of::<OCIDate>(), size_of::<OCIDate>(), stmt, err)?;
        if self.year == 0 {
            params.mark_as_null(pos);
        }
        Ok(pos + 1)
    }
}

impl ToSql for Date<'_> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        self.date.bind_to(pos, params, stmt, err)
    }
}

impl ToSql for &Date<'_> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind_in(pos, SQLT_ODT, &self.date as *const OCIDate as _, size_of::<OCIDate>(), stmt, err)?;
        Ok(pos + 1)
    }
}

impl ToSql for &mut Date<'_> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        self.date.bind_to(pos, params, stmt, err)
    }
}

impl ToSql for &[Date<'_>] {
    fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        for item in self.iter() {
            params.bind_in(pos, SQLT_ODT, &item.date as *const OCIDate as _, size_of::<OCIDate>(), stmt, err)?;
            pos += 1;
        }
        Ok(pos)
    }

    fn update_from_bind(&mut self, pos: usize, _params: &Params) -> Result<usize> {
        Ok(pos + self.len())
    }
}

impl ToSql for &[&Date<'_>] {
    fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        for &item in self.iter() {
            params.bind_in(pos, SQLT_ODT, &item.date as *const OCIDate as _, size_of::<OCIDate>(), stmt, err)?;
            pos += 1;
        }
        Ok(pos)
    }

    fn update_from_bind(&mut self, pos: usize, _params: &Params) -> Result<usize> {
        Ok(pos + self.len())
    }
}

impl ToSql for &mut [&mut Date<'_>] {
    fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        for item in self.iter_mut() {
            pos = item.date.bind_to(pos, params, stmt, err)?;            
        }
        Ok(pos)
    }

    fn update_from_bind(&mut self, pos: usize, _params: &Params) -> Result<usize> {
        Ok(pos + self.len())
    }
}

impl_sql_type!{ Date<'_>, &Date<'_>, &mut Date<'_> => SQLT_ODT }

impl OracleDataType for Date<'_> {}
impl OracleDataType for &Date<'_> {}
impl OracleDataType for &mut Date<'_> {}
