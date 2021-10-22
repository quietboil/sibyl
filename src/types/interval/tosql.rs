/// Implementation of traits that allow Intervals to be used as SQL parameters

use libc::c_void;
use crate::{ oci::*, tosql::ToSql, tosqlout::ToSqlOut, desc::Descriptor };
use super::Interval;

macro_rules! impl_int_to_sql {
    ($ts:ty => $sqlt:ident) => {
        impl ToSql for Interval<'_, $ts> {
            fn to_sql(&self) -> (u16, *const c_void, usize) {
                ( $sqlt, self.interval.as_ptr() as *const c_void, std::mem::size_of::<*mut OCIInterval>() )
            }
        }
        impl ToSql for &Interval<'_, $ts> {
            fn to_sql(&self) -> (u16, *const c_void, usize) {
                ( $sqlt, (*self).interval.as_ptr() as *const c_void, std::mem::size_of::<*mut OCIInterval>() )
            }
        }
    };
}

impl_int_to_sql!{ OCIIntervalYearToMonth => SQLT_INTERVAL_YM }
impl_int_to_sql!{ OCIIntervalDayToSecond => SQLT_INTERVAL_DS }

macro_rules! impl_int_to_sql_output {
    ($ts:ty => $sqlt:ident) => {
        impl ToSqlOut for Descriptor<$ts> {
            fn to_sql_output(&mut self) -> (u16, *mut c_void, usize, usize) {
                ($sqlt, self.as_ptr() as *mut c_void, std::mem::size_of::<*mut OCIInterval>(), std::mem::size_of::<*mut OCIInterval>())
            }
        }
        impl ToSqlOut for Interval<'_, $ts> {
            fn to_sql_output(&mut self) -> (u16, *mut c_void, usize, usize) {
                self.interval.to_sql_output()
            }
        }
    };
}

impl_int_to_sql_output!{ OCIIntervalYearToMonth => SQLT_INTERVAL_YM }
impl_int_to_sql_output!{ OCIIntervalDayToSecond => SQLT_INTERVAL_DS }
