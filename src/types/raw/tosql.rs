/// Implementation of traits that allow Raw values to be used as SQL parameters

use std::mem::size_of;
use crate::{oci::*, ToSql, Result, stmt::Params};
use super::Raw;

impl SqlType for Raw<'_> {
    fn sql_type() -> u16 { SQLT_LVB }
    fn sql_null_type() -> u16 { SQLT_BIN }
}

impl SqlType for &Raw<'_> {
    fn sql_type() -> u16 { SQLT_LVB }
    fn sql_null_type() -> u16 { SQLT_BIN }
}


impl ToSql for Raw<'_> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let len = self.len();
        let cap = self.capacity()? + size_of::<u32>();
        params.bind(pos, SQLT_LVB, self.raw.get() as _, len + size_of::<u32>(), cap, stmt, err)?;
        if len == 0 {
            params.mark_as_null(pos);
        }
        Ok(pos + 1)
    }
}

impl ToSql for &Raw<'_> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let len = self.len();
        let cap = self.capacity()? + size_of::<u32>();
        params.bind(pos, SQLT_LVB, self.raw.get() as _, len + size_of::<u32>(), cap, stmt, err)?;
        if len == 0 {
            params.mark_as_null(pos);
        }
        Ok(pos + 1)
    }
}

impl ToSql for &mut Raw<'_> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let len = self.len();
        let cap = self.capacity()? + size_of::<u32>();
        params.bind(pos, SQLT_LVB, self.raw.get() as _, len + size_of::<u32>(), cap, stmt, err)?;
        if len == 0 {
            params.mark_as_null(pos);
        }
        Ok(pos + 1)
    }
}

impl ToSql for &[Raw<'_>] {
    fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        for item in self.iter() {
            let len = item.len();
            let cap = item.capacity()? + size_of::<u32>();
            params.bind(pos, SQLT_LVB, item.raw.get() as _, len + size_of::<u32>(), cap, stmt, err)?;
            if len == 0 {
                params.mark_as_null(pos);
            }
            pos += 1;
        }
        Ok(pos)
    }
}

impl ToSql for &[&Raw<'_>] {
    fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        for item in self.iter() {
            let len = item.len();
            let cap = item.capacity()? + size_of::<u32>();
            params.bind(pos, SQLT_LVB, item.raw.get() as _, len + size_of::<u32>(), cap, stmt, err)?;
            if len == 0 {
                params.mark_as_null(pos);
            }
            pos += 1;
        }
        Ok(pos)
    }
}

impl ToSql for &mut [&mut Raw<'_>] {
    fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        for item in self.iter_mut() {
            let len = item.len();
            let cap = item.capacity()? + size_of::<u32>();
            params.bind(pos, SQLT_LVB, item.raw.get() as _, len + size_of::<u32>(), cap, stmt, err)?;
            if len == 0 {
                params.mark_as_null(pos);
            }
            pos += 1;
        }
        Ok(pos)
    }
}
