/// Implementation of traits that allow Timestamps to be used as SQL parameters

use crate::{oci::*, ToSql, ToSqlOut, Result, stmt::Params};
use super::DateTime;

macro_rules! impl_ts_to_sql {
    ($($ts:ty),+) => {
        $(
            impl ToSql for DateTime<'_, $ts> {
                fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    self.datetime.bind_to(pos, params, stmt, err)
                }
            }
            impl ToSql for &DateTime<'_, $ts> {
                fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    self.datetime.bind_to(pos, params, stmt, err)
                }
            }
            impl ToSqlOut for &mut DateTime<'_, $ts> {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    self.datetime.bind_to(pos, params, stmt, err)
                }
            }
        )+
    };
}

impl_ts_to_sql!{ OCITimestamp, OCITimestampTZ, OCITimestampLTZ }
