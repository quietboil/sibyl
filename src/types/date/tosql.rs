/// Implementation of traits that allow Dates to be used as SQL parameters

use libc::c_void;
use crate::{ oci::*, ToSql, ToSqlOut};
use super::Date;

impl ToSql for Date<'_> {
    fn sql_type(&self) -> u16 { SQLT_ODT }
    fn sql_data_ptr(&self) -> Ptr<c_void> { Ptr::new(&self.date as *const OCIDate as _) }
    fn sql_data_len(&self) -> usize { std::mem::size_of::<OCIDate>() }
}

impl ToSql for &Date<'_> {
    fn sql_type(&self) -> u16 { SQLT_ODT }
    fn sql_data_ptr(&self) -> Ptr<c_void> { Ptr::new(&self.date as *const OCIDate as _) }
    fn sql_data_len(&self) -> usize { std::mem::size_of::<OCIDate>() }
}

impl ToSqlOut for Date<'_> {
    fn sql_type(&self) -> u16 { SQLT_ODT }
    fn sql_mut_data_ptr(&mut self) -> Ptr<c_void> { Ptr::new(&mut self.date as *const OCIDate as _) }
    fn sql_data_len(&self) -> usize { std::mem::size_of::<OCIDate>() }
}
