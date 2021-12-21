/// Implementation of traits that allow Intervals to be used as SQL parameters

use libc::c_void;
use crate::{ oci::*, ToSql, ToSqlOut};
use super::Interval;

macro_rules! impl_int_to_sql {
    ($ts:ty => $sqlt:ident) => {
        impl ToSql for Interval<'_, $ts> {
            fn sql_type(&self) -> u16 { $sqlt }
            fn sql_data_ptr(&self) -> Ptr<c_void> { Ptr::new(self.interval.as_ptr() as _) }
            fn sql_data_len(&self) -> usize { std::mem::size_of::<*mut OCIInterval>() }
        }
        impl ToSql for &Interval<'_, $ts> {
            fn sql_type(&self) -> u16 { $sqlt }
            fn sql_data_ptr(&self) -> Ptr<c_void> { Ptr::new((*self).interval.as_ptr() as _) }
            fn sql_data_len(&self) -> usize { std::mem::size_of::<*mut OCIInterval>() }
        }
    };
}

impl_int_to_sql!{ OCIIntervalYearToMonth => SQLT_INTERVAL_YM }
impl_int_to_sql!{ OCIIntervalDayToSecond => SQLT_INTERVAL_DS }

macro_rules! impl_int_to_sql_output {
    ($ts:ty => $sqlt:ident) => {
        impl ToSqlOut for Descriptor<$ts> {
            fn sql_type(&self) -> u16 { $sqlt }
            fn sql_mut_data_ptr(&mut self) -> Ptr<c_void> { Ptr::new(self.as_mut_ptr() as _) }
            fn sql_data_len(&self) -> usize { std::mem::size_of::<*mut OCIInterval>() }
        }
        impl ToSqlOut for Interval<'_, $ts> {
            fn sql_type(&self) -> u16 { $sqlt }
            fn sql_mut_data_ptr(&mut self) -> Ptr<c_void> { Ptr::new(self.interval.as_mut_ptr() as _) }
            fn sql_data_len(&self) -> usize { std::mem::size_of::<*mut OCIInterval>() }
        }
    };
}

impl_int_to_sql_output!{ OCIIntervalYearToMonth => SQLT_INTERVAL_YM }
impl_int_to_sql_output!{ OCIIntervalDayToSecond => SQLT_INTERVAL_DS }
