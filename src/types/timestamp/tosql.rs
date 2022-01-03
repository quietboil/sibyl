/// Implementation of traits that allow Timestamps to be used as SQL parameters

use std::mem::size_of;
use crate::{oci::*, ToSql, ToSqlOut, Result, stmt::Params};
use super::Timestamp;

macro_rules! impl_ts_to_sql {
    ($ts:ty => $sqlt:ident) => {
        impl ToSql for Descriptor<$ts> {
            fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                params.bind(pos, $sqlt, self.as_ptr() as _, size_of::<*mut OCIDateTime>(), stmt, err)?;
                Ok(pos + 1)
            }
        }
        impl ToSql for Timestamp<'_, $ts> {
            fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                self.datetime.bind_to(pos, params, stmt, err)
            }
        }
        impl ToSql for &Timestamp<'_, $ts> {
            fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                self.datetime.bind_to(pos, params, stmt, err)
            }
        }
    };
}

impl_ts_to_sql!{ OCITimestamp    => SQLT_TIMESTAMP     }
impl_ts_to_sql!{ OCITimestampTZ  => SQLT_TIMESTAMP_TZ  }
impl_ts_to_sql!{ OCITimestampLTZ => SQLT_TIMESTAMP_LTZ }

macro_rules! impl_ts_to_sql_output {
    ($ts:ty => $sqlt:ident) => {
        impl ToSqlOut for &mut Descriptor<$ts> {
            fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                let len = size_of::<*mut OCIDateTime>();
                params.bind_out(pos, $sqlt, self.as_mut_ptr() as _, len, len, stmt, err)?;
                Ok(pos + 1)
            }
        }
        impl ToSqlOut for &mut Timestamp<'_, $ts> {
            fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                self.datetime.bind_to(pos, params, stmt, err)
            }
        }
    };
}

impl_ts_to_sql_output!{ OCITimestamp    => SQLT_TIMESTAMP     }
impl_ts_to_sql_output!{ OCITimestampTZ  => SQLT_TIMESTAMP_TZ  }
impl_ts_to_sql_output!{ OCITimestampLTZ => SQLT_TIMESTAMP_LTZ }
