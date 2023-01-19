/// Implementation of traits that allow Numbers to be used as SQL parameters

use std::mem::size_of;
use crate::{oci::*, ToSql, Result, stmt::Params};
use super::Number;

impl ToSql for OCINumber {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind(pos, SQLT_VNU, self as *const OCINumber as _, size_of::<OCINumber>(), size_of::<OCINumber>(), stmt, err)?;
        Ok(pos + 1)
    }
}

impl ToSql for Number<'_> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        self.num.bind_to(pos, params, stmt, err)
    }
}

impl ToSql for &Number<'_> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind_in(pos, SQLT_VNU, &self.num as *const OCINumber as _, size_of::<OCINumber>(), stmt, err)?;
        Ok(pos + 1)
    }
}

impl ToSql for &mut Number<'_> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind(pos, SQLT_VNU, &mut self.num as *mut OCINumber as _, size_of::<OCINumber>(), size_of::<OCINumber>(), stmt, err)?;
        Ok(pos + 1)
    }
}

impl ToSql for &[Number<'_>] {
    fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        for item in self.iter() {
            params.bind_in(pos, SQLT_VNU, &item.num as *const OCINumber as _, size_of::<OCINumber>(), stmt, err)?;
            pos += 1;
        }
        Ok(pos)
    }
}

impl ToSql for &[&Number<'_>] {
    fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        for item in self.iter() {
            params.bind_in(pos, SQLT_VNU, &item.num as *const OCINumber as _, size_of::<OCINumber>(), stmt, err)?;
            pos += 1;
        }
        Ok(pos)
    }
}

impl ToSql for &mut [&mut Number<'_>] {
    fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        for item in self.iter_mut() {
            params.bind(pos, SQLT_VNU, &mut item.num as *mut OCINumber as _, size_of::<OCINumber>(), size_of::<OCINumber>(), stmt, err)?;
            pos += 1;
        }
        Ok(pos)
    }
}

impl_sql_type!{ OCINumber, Number<'_> => SQLT_VNU }