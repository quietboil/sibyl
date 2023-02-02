use std::mem::size_of;
use crate::types::OracleDataType;
use crate::{oci::*, ToSql, Result, stmt::Params, RowID};
use super::is_initialized;

impl ToSql for RowID {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let len = size_of::<*mut OCIRowid>();
        params.bind(pos, SQLT_RDD, self.0.as_ptr() as _, len, len, stmt, err)?;
        if !is_initialized(&self.0) {
            params.mark_as_null(pos);
        }
        Ok(pos + 1)
    }
}

impl ToSql for &RowID {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let len = size_of::<*mut OCIRowid>();
        params.bind_in(pos, SQLT_RDD, self.0.as_ptr() as _, len, stmt, err)?;
        Ok(pos + 1)
    }
}

impl ToSql for &mut RowID {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let len = size_of::<*mut OCIRowid>();
        params.bind(pos, SQLT_RDD, self.0.as_mut_ptr() as _, len, len, stmt, err)?;
        if !is_initialized(&self.0) {
            params.mark_as_null(pos);
        }
        Ok(pos + 1)
    }
}

impl ToSql for &[RowID] {
    fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let len = size_of::<*mut OCIRowid>();
        for item in self.iter() {
            params.bind_in(pos, SQLT_RDD, item.0.as_ptr() as _, len, stmt, err)?;
            pos += 1;
        }
        Ok(pos)
    }

    fn update_from_bind(&mut self, pos: usize, _params: &Params) -> Result<usize> {
        Ok(pos + self.len())
    }
}

impl ToSql for &[&RowID] {
    fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let len = size_of::<*mut OCIRowid>();
        for &item in self.iter() {
            params.bind_in(pos, SQLT_RDD, item.0.as_ptr() as _, len, stmt, err)?;
            pos += 1;
        }
        Ok(pos)
    }

    fn update_from_bind(&mut self, pos: usize, _params: &Params) -> Result<usize> {
        Ok(pos + self.len())
    }
}

impl ToSql for &mut [&mut RowID] {
    fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let len = size_of::<*mut OCIRowid>();
        for item in self.iter_mut() {
            params.bind(pos, SQLT_RDD, item.0.as_mut_ptr() as _, len, len, stmt, err)?;
            if !is_initialized(&item.0) {
                params.mark_as_null(pos);
            }
            pos += 1;
        }
        Ok(pos)
    }

    fn update_from_bind(&mut self, pos: usize, _params: &Params) -> Result<usize> {
        Ok(pos + self.len())
    }
}

impl_sql_type!{ RowID, &RowID, &mut RowID => SQLT_RDD }

impl OracleDataType for RowID {}
impl OracleDataType for &RowID {}
impl OracleDataType for &mut RowID {}