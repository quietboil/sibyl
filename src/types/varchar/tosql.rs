/// Implementation of traits that allow Varchars to be used as SQL parameters

use libc::c_void;
use crate::{ oci::*, ToSql, ToSqlOut};
use super::Varchar;

impl ToSql for Varchar<'_> {
    fn sql_type(&self) -> u16 { SQLT_LVC }
    fn sql_data_ptr(&self) -> Ptr<c_void> { Ptr::new(self.txt.get() as _) }
    fn sql_data_len(&self) -> usize { self.len() + std::mem::size_of::<u32>() }
}

impl ToSqlOut for Varchar<'_> {
    fn sql_type(&self) -> u16 { SQLT_LVC }
    fn sql_mut_data_ptr(&mut self) -> Ptr<c_void> { Ptr::new(self.txt.get() as _) }
    fn sql_data_len(&self) -> usize { self.len() + std::mem::size_of::<u32>() }
    fn sql_capacity(&self) -> usize { self.capacity().ok().unwrap_or_default() + std::mem::size_of::<u32>() }
}
