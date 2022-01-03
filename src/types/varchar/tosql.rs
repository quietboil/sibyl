/// Implementation of traits that allow Varchars to be used as SQL parameters

use std::mem::size_of;
use crate::{oci::*, ToSql, ToSqlOut, Result, stmt::Params};
use super::Varchar;

impl ToSql for Varchar<'_> {
    fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let len = self.len() + size_of::<u32>();
        params.bind(pos, SQLT_LVC, self.txt.get() as _, len, stmt, err)?;
        Ok(pos + 1)
    }
}

impl ToSql for &Varchar<'_> {
    fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let len = self.len() + size_of::<u32>();
        params.bind(pos, SQLT_LVC, self.txt.get() as _, len, stmt, err)?;
        Ok(pos + 1)
    }
}

impl ToSqlOut for &mut Varchar<'_> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let len = self.len() + size_of::<u32>();
        let cap = self.capacity()? + size_of::<u32>();
        params.bind_out(pos, SQLT_LVC, self.txt.get() as _, len, cap, stmt, err)?;
        Ok(pos + 1)
    }
}
