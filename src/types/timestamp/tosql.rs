/// Implementation of traits that allow Timestamps to be used as SQL parameters

use libc::c_void;
use crate::{ oci::*, ToSql, ToSqlOut};
use super::Timestamp;

macro_rules! impl_ts_to_sql {
    ($ts:ty => $sqlt:ident) => {
        impl ToSql for Timestamp<'_, $ts> {
            fn sql_type(&self) -> u16 { $sqlt }
            fn sql_data_ptr(&self) -> Ptr<c_void> { Ptr::new(self.datetime.as_ptr() as _) }
            fn sql_data_len(&self) -> usize { std::mem::size_of::<*mut OCIDateTime>() }
        }
        impl ToSql for &Timestamp<'_, $ts> {
            fn sql_type(&self) -> u16 { $sqlt }
            fn sql_data_ptr(&self) -> Ptr<c_void> { Ptr::new((*self).datetime.as_ptr() as _) }
            fn sql_data_len(&self) -> usize { std::mem::size_of::<*mut OCIDateTime>() }
        }
    };
}

impl_ts_to_sql!{ OCITimestamp    => SQLT_TIMESTAMP     }
impl_ts_to_sql!{ OCITimestampTZ  => SQLT_TIMESTAMP_TZ  }
impl_ts_to_sql!{ OCITimestampLTZ => SQLT_TIMESTAMP_LTZ }

macro_rules! impl_ts_to_sql_output {
    ($ts:ty => $sqlt:ident) => {
        impl ToSqlOut for Descriptor<$ts> {
            fn sql_type(&self) -> u16 { $sqlt }
            fn sql_mut_data_ptr(&mut self) -> Ptr<c_void> { Ptr::new(self.as_mut_ptr() as _) }
            fn sql_data_len(&self) -> usize { std::mem::size_of::<*mut OCIDateTime>() }
        }
        impl ToSqlOut for Timestamp<'_, $ts> {
            fn sql_type(&self) -> u16 { $sqlt }
            fn sql_mut_data_ptr(&mut self) -> Ptr<c_void> { Ptr::new(self.datetime.as_mut_ptr() as _) }
            fn sql_data_len(&self) -> usize { std::mem::size_of::<*mut OCIDateTime>() }
        }
    };
}

impl_ts_to_sql_output!{ OCITimestamp    => SQLT_TIMESTAMP     }
impl_ts_to_sql_output!{ OCITimestampTZ  => SQLT_TIMESTAMP_TZ  }
impl_ts_to_sql_output!{ OCITimestampLTZ => SQLT_TIMESTAMP_LTZ }
