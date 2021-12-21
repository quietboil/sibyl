/// Implementation of traits that allow Numbers to be used as SQL parameters

use libc::c_void;
use crate::{ oci::*, ToSql, ToSqlOut};
use super::Number;

impl ToSql for Number<'_> {
    fn sql_type(&self) -> u16 { SQLT_VNU }
    fn sql_data_ptr(&self) -> Ptr<c_void> { Ptr::new(&self.num as *const OCINumber as _) }
    fn sql_data_len(&self) -> usize { std::mem::size_of::<OCINumber>() }
}

impl ToSql for &Number<'_> {
    fn sql_type(&self) -> u16 { SQLT_VNU }
    fn sql_data_ptr(&self) -> Ptr<c_void> { Ptr::new(&self.num as *const OCINumber as _) }
    fn sql_data_len(&self) -> usize { std::mem::size_of::<OCINumber>() }
}

impl ToSqlOut for Number<'_> {
    fn sql_type(&self) -> u16 { SQLT_VNU }
    fn sql_mut_data_ptr(&mut self) -> Ptr<c_void> { Ptr::new(&mut self.num as *mut OCINumber as _) }
    fn sql_data_len(&self) -> usize { std::mem::size_of::<OCINumber>() }
}
