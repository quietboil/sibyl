use crate::types::OracleDataType;
/// Implementation of traits that allow Timestamps to be used as SQL parameters

use crate::{oci::*, ToSql, Result, stmt::Params};
use super::DateTime;
use std::mem::size_of;

macro_rules! impl_ts_to_sql {
    ($($ts:ty),+) => {
        $(
            impl ToSql for DateTime<'_, $ts> {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    let next_pos = self.datetime.bind_to(pos, params, stmt, err)?;
                    let (year, _, _) = self.date()?;
                    if year == 0 {
                        params.mark_as_null(pos);
                    }
                    Ok(next_pos)
                }
            }
            impl ToSql for &DateTime<'_, $ts> {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    let len = size_of::<*mut <$ts as DescriptorType>::OCIType>();
                    params.bind(pos, <$ts>::sql_type(), self.datetime.as_ptr() as _, len, len, stmt, err)?;
                    Ok(pos + 1)
                }
            }
            impl ToSql for &mut DateTime<'_, $ts> {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    (*self).bind_to(pos, params, stmt, err)
                }
            }
            impl ToSql for &[DateTime<'_, $ts>] {
                fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    let len = size_of::<*mut <$ts as DescriptorType>::OCIType>();
                    for item in self.iter() {
                        params.bind(pos, <$ts>::sql_type(), item.datetime.as_ptr() as _, len, len, stmt, err)?;
                        pos += 1;
                    }
                    Ok(pos)
                }
            }
            impl ToSql for &[&DateTime<'_, $ts>] {
                fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    let len = size_of::<*mut <$ts as DescriptorType>::OCIType>();
                    for item in self.iter() {
                        params.bind(pos, <$ts>::sql_type(), item.datetime.as_ptr() as _, len, len, stmt, err)?;
                        pos += 1;
                    }
                    Ok(pos)
                }
            }
            impl ToSql for &mut [&mut DateTime<'_, $ts>] {
                fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    for item in self.iter_mut() {
                        pos = (*item).bind_to(pos, params, stmt, err)?;
                    }
                    Ok(pos)
                }
            }
            impl SqlType for DateTime<'_, $ts> {
                fn sql_type() -> u16 {
                    <$ts>::sql_type()
                }
            }
            impl SqlType for &DateTime<'_, $ts> {
                fn sql_type() -> u16 {
                    <$ts>::sql_type()
                }
            }
            impl SqlType for &mut DateTime<'_, $ts> {
                fn sql_type() -> u16 {
                    <$ts>::sql_type()
                }
            }
        )+
    };
}

impl_ts_to_sql!{ OCITimestamp, OCITimestampTZ, OCITimestampLTZ }

impl OracleDataType for DateTime<'_, OCITimestamp> {}
impl OracleDataType for &DateTime<'_, OCITimestamp> {}
impl OracleDataType for &mut DateTime<'_, OCITimestamp> {}
impl OracleDataType for DateTime<'_, OCITimestampTZ> {}
impl OracleDataType for &DateTime<'_, OCITimestampTZ> {}
impl OracleDataType for &mut DateTime<'_, OCITimestampTZ> {}
impl OracleDataType for DateTime<'_, OCITimestampLTZ> {}
impl OracleDataType for &DateTime<'_, OCITimestampLTZ> {}
impl OracleDataType for &mut DateTime<'_, OCITimestampLTZ> {}
