/// Implementation of traits that allow Timestamps to be used as SQL parameters

use libc::c_void;
use crate::{ oci::*, tosql::ToSql, tosqlout::ToSqlOut, desc::Descriptor };
use super::Timestamp;

macro_rules! impl_ts_to_sql {
    ($ts:ty => $sqlt:ident) => {
        impl ToSql for Timestamp<'_, $ts> {
            fn to_sql(&self) -> (u16, *const c_void, usize) {
                ( $sqlt, self.datetime.as_ptr() as *const c_void, std::mem::size_of::<*mut OCIDateTime>() )
            }
        }
        impl ToSql for &Timestamp<'_, $ts> {
            fn to_sql(&self) -> (u16, *const c_void, usize) {
                ( $sqlt, (*self).datetime.as_ptr() as *const c_void, std::mem::size_of::<*mut OCIDateTime>() )
            }
        }
    };
}

impl_ts_to_sql!{ OCITimestamp    => SQLT_TIMESTAMP     }
impl_ts_to_sql!{ OCITimestampTZ  => SQLT_TIMESTAMP_TZ  }
impl_ts_to_sql!{ OCITimestampLTZ => SQLT_TIMESTAMP_LTZ }

macro_rules! impl_ts_to_sql_output {
    ($ts:ty => $sqlt:ident) => {
        impl ToSqlOut for Descriptor<$ts> {
            fn to_sql_output(&mut self) -> (u16, *mut c_void, usize, usize) {
                ($sqlt, self.as_ptr() as *mut c_void, std::mem::size_of::<*mut OCIDateTime>(), std::mem::size_of::<*mut OCIDateTime>())
            }
        }
        impl ToSqlOut for Timestamp<'_, $ts> {
            fn to_sql_output(&mut self) -> (u16, *mut c_void, usize, usize) {
                self.datetime.to_sql_output()
            }
        }
    };
}

impl_ts_to_sql_output!{ OCITimestamp    => SQLT_TIMESTAMP     }
impl_ts_to_sql_output!{ OCITimestampTZ  => SQLT_TIMESTAMP_TZ  }
impl_ts_to_sql_output!{ OCITimestampLTZ => SQLT_TIMESTAMP_LTZ }
