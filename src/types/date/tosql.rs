/// Implementation of traits that allow Dates to be used as SQL parameters

use libc::c_void;
use crate::{ oci::*, tosql::ToSql, tosqlout::ToSqlOut };
use super::{ Date, OCIDate };

impl ToSql for Date<'_> {
    fn to_sql(&self) -> (u16, *const c_void, usize) {
        ( SQLT_ODT, self.as_ptr() as *const c_void, std::mem::size_of::<OCIDate>() )
    }
}

impl ToSql for &Date<'_> {
    fn to_sql(&self) -> (u16, *const c_void, usize) {
        ( SQLT_ODT, (*self).as_ptr() as *const c_void, std::mem::size_of::<OCIDate>() )
    }
}

impl ToSqlOut for Date<'_> {
    fn to_sql_output(&mut self) -> (u16, *mut c_void, usize, usize) {
        (SQLT_ODT, self.as_mut_ptr() as *mut c_void, std::mem::size_of::<OCIDate>(), std::mem::size_of::<OCIDate>())
    }
}
