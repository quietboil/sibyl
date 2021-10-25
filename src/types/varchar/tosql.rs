/// Implementation of traits that allow Varchars to be used as SQL parameters

use libc::c_void;
use crate::{ oci::*, tosql::ToSql, tosqlout::ToSqlOut };
use super::Varchar;

impl ToSql for Varchar<'_> {
    fn to_sql(&self) -> (u16, *const c_void, usize) {
        ( SQLT_LVC, self.as_ptr() as *const c_void, self.len() + std::mem::size_of::<u32>() )
    }
}

impl ToSqlOut for Varchar<'_> {
    fn to_sql_output(&mut self) -> (u16, *mut c_void, usize, usize) {
        (
            SQLT_LVC,
            self.as_mut_ptr() as *mut c_void,
            self.capacity().ok().unwrap_or_default() + std::mem::size_of::<u32>(),
            self.len() + std::mem::size_of::<u32>()
        )
    }
}