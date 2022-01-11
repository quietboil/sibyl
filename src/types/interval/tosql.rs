/// Implementation of traits that allow Intervals to be used as SQL parameters

use crate::{oci::*, ToSql, ToSqlOut, Result, stmt::Params};
use super::Interval;

macro_rules! impl_int_to_sql {
    ($($ts:ty),+) => {
        $(
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
            impl ToSqlOut for &mut Interval<'_, $ts> {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    self.interval.bind_to(pos, params, stmt, err)
                }
            }
        )+
    };
}

impl_int_to_sql!{ OCIIntervalYearToMonth, OCIIntervalDayToSecond }
