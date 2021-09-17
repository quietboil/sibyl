/// Implementation of traits that allow Numbers to be used as SQL parameters

use super::*;

impl ToSql for Number<'_> {
    fn to_sql(&self) -> (u16, *const c_void, usize) {
        ( SQLT_VNU, self.as_ptr() as *const c_void, std::mem::size_of::<OCINumber>() )
    }
}

impl ToSql for &Number<'_> {
    fn to_sql(&self) -> (u16, *const c_void, usize) {
        ( SQLT_VNU, (*self).as_ptr() as *const c_void, std::mem::size_of::<OCINumber>() )
    }
}

impl ToSqlOut for Number<'_> {
    fn to_sql_output(&mut self) -> (u16, *mut c_void, usize, usize) {
        (SQLT_VNU, self.as_mut_ptr() as *mut c_void, std::mem::size_of::<OCINumber>(), std::mem::size_of::<OCINumber>())
    }
}
