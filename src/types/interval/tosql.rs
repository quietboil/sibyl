/// Implementation of traits that allow Intervals to be used as SQL parameters

use crate::{oci::*, ToSql, Result, stmt::Params};
use super::Interval;

macro_rules! impl_int_to_sql {
    ($($ts:ty),+) => {
        $(
            impl ToSql for Interval<'_, $ts> {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    self.interval.bind_to(pos, params, stmt, err)
                }
            }
            impl ToSql for &Interval<'_, $ts> {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    let len = std::mem::size_of::<*mut <$ts as DescriptorType>::OCIType>();
                    params.bind(pos, <$ts>::sql_type(), self.interval.as_ptr() as _, len, stmt, err)?;
                    Ok(pos + 1)
                }
            }
            impl ToSql for &mut Interval<'_, $ts> {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    let len = std::mem::size_of::<*mut <$ts as DescriptorType>::OCIType>();
                    params.bind_out(pos, <$ts>::sql_type(), self.interval.as_mut_ptr() as _, len, len, stmt, err)?;
                    Ok(pos + 1)
                }
            }
        )+
    };
}

impl_int_to_sql!{ OCIIntervalYearToMonth, OCIIntervalDayToSecond }
