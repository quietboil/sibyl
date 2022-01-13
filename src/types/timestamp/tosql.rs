/// Implementation of traits that allow Timestamps to be used as SQL parameters

use crate::{oci::*, ToSql, Result, stmt::Params};
use super::DateTime;

macro_rules! impl_ts_to_sql {
    ($($ts:ty),+) => {
        $(
            impl ToSql for DateTime<'_, $ts> {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    self.datetime.bind_to(pos, params, stmt, err)
                }
            }
            impl ToSql for &DateTime<'_, $ts> {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    let len = std::mem::size_of::<*mut <$ts as DescriptorType>::OCIType>();
                    params.bind(pos, <$ts>::sql_type(), self.datetime.as_ptr() as _, len, stmt, err)?;
                    Ok(pos + 1)
                }
            }
            impl ToSql for &mut DateTime<'_, $ts> {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    let len = std::mem::size_of::<*mut <$ts as DescriptorType>::OCIType>();
                    params.bind_out(pos, <$ts>::sql_type(), self.datetime.as_mut_ptr() as _, len, len, stmt, err)?;
                    Ok(pos + 1)
                }
            }
        )+
    };
}

impl_ts_to_sql!{ OCITimestamp, OCITimestampTZ, OCITimestampLTZ }
