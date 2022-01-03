/// Implementation of traits that allow Intervals to be used as SQL parameters

use std::mem::size_of;
use crate::{oci::*, ToSql, ToSqlOut, Result, stmt::Params};
use super::Interval;

macro_rules! impl_int_to_sql {
    ($ts:ty => $sqlt:ident) => {
        impl ToSql for Descriptor<$ts> {
            fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                params.bind(pos, $sqlt, self.as_ptr() as _, size_of::<*mut OCIInterval>(), stmt, err)?;
                Ok(pos + 1)
            }
        }
        impl ToSql for Interval<'_, $ts> {
            fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                self.interval.bind_to(pos, params, stmt, err)
            }
        }
        impl ToSql for &Interval<'_, $ts> {
            fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                self.interval.bind_to(pos, params, stmt, err)
            }
        }
    };
}

impl_int_to_sql!{ OCIIntervalYearToMonth => SQLT_INTERVAL_YM }
impl_int_to_sql!{ OCIIntervalDayToSecond => SQLT_INTERVAL_DS }

macro_rules! impl_int_to_sql_output {
    ($ts:ty => $sqlt:ident) => {
        impl ToSqlOut for Descriptor<$ts> {
            fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                let len = size_of::<*mut OCIInterval>();
                params.bind_out(pos, $sqlt, self.as_mut_ptr() as _, len, len, stmt, err)?;
                Ok(pos + 1)
            }
        }
        impl ToSqlOut for &mut Interval<'_, $ts> {
            fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                self.interval.bind_to(pos, params, stmt, err)
            }
        }
    };
}

impl_int_to_sql_output!{ OCIIntervalYearToMonth => SQLT_INTERVAL_YM }
impl_int_to_sql_output!{ OCIIntervalDayToSecond => SQLT_INTERVAL_DS }

// impl ToSqlOut for Descriptor<OCIIntervalYearToMonth> {
//     fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
//         let len = size_of::<*mut OCIInterval>();
//         params.bind_out(pos, SQLT_INTERVAL_YM, self.as_mut_ptr() as _, len, len, stmt, err)?;
//         Ok(pos + 1)
//     }
// }

// impl ToSqlOut for &mut Interval<'_, OCIIntervalYearToMonth> {
//     fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
//         self.interval.bind_to(pos, params, stmt, err)
//     }
// }
