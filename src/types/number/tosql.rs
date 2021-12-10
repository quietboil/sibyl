/// Implementation of traits that allow Numbers to be used as SQL parameters

use libc::c_void;
use crate::{ oci::*, stmt::args::{ ToSql, ToSqlOut }, ptr::{ScopedPtr, ScopedMutPtr} };
use super::Number;

impl ToSql for Number<'_> {
    fn sql_type(&self) -> u16 { SQLT_VNU }
    fn sql_data_ptr(&self) -> ScopedPtr<c_void> { ScopedPtr::new(self.as_ptr() as _) }
    fn sql_data_len(&self) -> usize { std::mem::size_of::<OCINumber>() }
}

impl ToSql for &Number<'_> {
    fn sql_type(&self) -> u16 { SQLT_VNU }
    fn sql_data_ptr(&self) -> ScopedPtr<c_void> { ScopedPtr::new((*self).as_ptr() as _) }
    fn sql_data_len(&self) -> usize { std::mem::size_of::<OCINumber>() }
}

impl ToSqlOut for Number<'_> {
    fn sql_type(&self) -> u16 { SQLT_VNU }
    fn sql_mut_data_ptr(&mut self) -> ScopedMutPtr<c_void> { ScopedMutPtr::new(self.as_mut_ptr() as _) }
    fn sql_data_len(&self) -> usize { std::mem::size_of::<OCINumber>() }
}
