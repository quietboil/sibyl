/// The ROWID data type identifies a particular row in a database table.

use crate::*;
use libc::c_void;

impl ToSql for RowID {
    fn to_sql(&self) -> (u16, *const c_void, usize) {
        ( SQLT_RDD, self.as_ptr() as *const c_void, std::mem::size_of::<*mut OCIRowid>() )
    }
}

impl ToSqlOut for RowID {
    fn to_sql_output(&mut self, _col_size: usize) -> (u16, *mut c_void, usize) {
        (SQLT_RDD, self.as_ptr() as *mut c_void, std::mem::size_of::<*mut OCIRowid>())
    }
}

// extern "C" {
//     // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/miscellaneous-functions.html#GUID-064F2680-453A-40D1-9C36-518F1E2B31DF
//     fn OCIRowidToChar(
//         desc:   *mut OCIRowid,
//         text:   *mut u8,
//         size:   *mut u16,
//         err:    *mut OCIError,
//     ) -> i32;
// }

// pub fn to_string(rowid: &RowID, env: &env::Env) -> Result<String> {
//     let mut text = String::with_capacity(4096);
//     let txt = unsafe { text.as_mut_vec() };
//     let mut len = txt.capacity() as u16;
//     catch!{env.err_ptr() =>
//         OCIRowidToChar(rowid.as_ptr(), txt.as_mut_ptr(), &mut len, env.err_ptr())
//     }
//     unsafe {
//         txt.set_len(len as usize);
//     }
//     Ok( text )
// }
