/// Implementation of traits that allow Dates to be used as SQL parameters

use libc::c_void;
use crate::{ oci::*, stmt::args::{ ToSql, ToSqlOut }, ptr::{ScopedPtr, ScopedMutPtr} };
use super::Date;

impl ToSql for Date<'_> {
    fn sql_type(&self) -> u16 { SQLT_ODT }
    fn sql_data_ptr(&self) -> ScopedPtr<c_void> { ScopedPtr::new(self.as_ptr() as _) }
    fn sql_data_len(&self) -> usize { std::mem::size_of::<OCIDate>() }
}

impl ToSql for &Date<'_> {
    fn sql_type(&self) -> u16 { SQLT_ODT }
    fn sql_data_ptr(&self) -> ScopedPtr<c_void> { ScopedPtr::new((*self).as_ptr() as _) }
    fn sql_data_len(&self) -> usize { std::mem::size_of::<OCIDate>() }
}

impl ToSqlOut for Date<'_> {
    fn sql_type(&self) -> u16 { SQLT_ODT }
    fn sql_mut_data_ptr(&mut self) -> ScopedMutPtr<c_void> { ScopedMutPtr::new(self.as_mut_ptr() as _) }
    fn sql_data_len(&self) -> usize { std::mem::size_of::<OCIDate>() }
}
