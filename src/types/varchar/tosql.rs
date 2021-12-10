/// Implementation of traits that allow Varchars to be used as SQL parameters

use libc::c_void;
use crate::{ oci::*, stmt::args::{ ToSql, ToSqlOut }, ptr::{ScopedPtr, ScopedMutPtr} };
use super::Varchar;

impl ToSql for Varchar<'_> {
    fn sql_type(&self) -> u16 { SQLT_LVC }
    fn sql_data_ptr(&self) -> ScopedPtr<c_void> { ScopedPtr::new(self.as_ptr() as _) }
    fn sql_data_len(&self) -> usize { self.len() + std::mem::size_of::<u32>() }
}

impl ToSqlOut for Varchar<'_> {
    fn sql_type(&self) -> u16 { SQLT_LVC }
    fn sql_mut_data_ptr(&mut self) -> ScopedMutPtr<c_void> { ScopedMutPtr::new(self.as_mut_ptr() as _) }
    fn sql_data_len(&self) -> usize { self.len() + std::mem::size_of::<u32>() }
    fn sql_capacity(&self) -> usize { self.capacity().ok().unwrap_or_default() + std::mem::size_of::<u32>() }
}
