/// Implementation of traits that allow Varchars to be used as SQL parameters

use std::mem::size_of;
use crate::types::OracleDataType;
use crate::{oci::*, ToSql, Result, stmt::Params};
use super::Varchar;

impl SqlType for Varchar<'_> {
    fn sql_type() -> u16 { SQLT_LVC }
    fn sql_null_type() -> u16 { SQLT_CHR }
}

impl SqlType for &Varchar<'_> {
    fn sql_type() -> u16 { SQLT_LVC }
    fn sql_null_type() -> u16 { SQLT_CHR }
}

impl SqlType for &mut Varchar<'_> {
    fn sql_type() -> u16 { SQLT_LVC }
    fn sql_null_type() -> u16 { SQLT_CHR }
}


impl ToSql for Varchar<'_> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let len = self.len();
        let cap = self.capacity()? + size_of::<u32>();
        params.bind(pos, SQLT_LVC, self.txt.get() as _, len + size_of::<u32>(), cap, stmt, err)?;
        if len == 0 {
            params.mark_as_null(pos);
        }
        Ok(pos + 1)
    }
}

impl ToSql for &Varchar<'_> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let len = self.len();
        let cap = self.capacity()? + size_of::<u32>();
        params.bind(pos, SQLT_LVC, self.txt.get() as _, len + size_of::<u32>(), cap, stmt, err)?;
        if len == 0 {
            params.mark_as_null(pos);
        }
        Ok(pos + 1)
    }
}

impl ToSql for &mut Varchar<'_> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let len = self.len();
        let cap = self.capacity()? + size_of::<u32>();
        params.bind(pos, SQLT_LVC, self.txt.get() as _, len + size_of::<u32>(), cap, stmt, err)?;
        if len == 0 {
            params.mark_as_null(pos);
        }
        Ok(pos + 1)
    }
}

impl ToSql for &[Varchar<'_>] {
    fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        for item in self.iter() {
            let len = item.len();
            let cap = item.capacity()? + size_of::<u32>();
            params.bind(pos, SQLT_LVC, item.txt.get() as _, len + size_of::<u32>(), cap, stmt, err)?;
            if len == 0 {
                params.mark_as_null(pos);
            }
            pos += 1;
        }
        Ok(pos)
    }
}

impl ToSql for &[&Varchar<'_>] {
    fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        for item in self.iter() {
            let len = item.len();
            let cap = item.capacity()? + size_of::<u32>();
            params.bind(pos, SQLT_LVC, item.txt.get() as _, len + size_of::<u32>(), cap, stmt, err)?;
            if len == 0 {
                params.mark_as_null(pos);
            }
            pos += 1;
        }
        Ok(pos)
    }
}

impl ToSql for &mut [&mut Varchar<'_>] {
    fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        for item in self.iter_mut() {
            let len = item.len();
            let cap = item.capacity()? + size_of::<u32>();
            params.bind(pos, SQLT_LVC, item.txt.get() as _, len + size_of::<u32>(), cap, stmt, err)?;
            if len == 0 {
                params.mark_as_null(pos);
            }
            pos += 1;
        }
        Ok(pos)
    }
}

impl OracleDataType for Varchar<'_> {}
impl OracleDataType for &Varchar<'_> {}
impl OracleDataType for &mut Varchar<'_> {}
