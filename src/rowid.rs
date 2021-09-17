/// The ROWID data type identifies a particular row in a database table.

use crate::*;
use libc::c_void;

impl ToSql for &RowID {
    fn to_sql(&self) -> (u16, *const c_void, usize) {
        ( SQLT_RDD, self.as_ptr() as *const c_void, std::mem::size_of::<*mut OCIRowid>() )
    }
}

impl ToSqlOut for RowID {
    fn to_sql_output(&mut self) -> (u16, *mut c_void, usize, usize) {
        (SQLT_RDD, self.as_ptr() as *mut c_void, std::mem::size_of::<*mut OCIRowid>(), std::mem::size_of::<*mut OCIRowid>())
    }
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/miscellaneous-functions.html#GUID-064F2680-453A-40D1-9C36-518F1E2B31DF
    fn OCIRowidToChar(
        desc:   *mut OCIRowid,
        text:   *mut u8,
        size:   *mut u16,
        err:    *mut OCIError,
    ) -> i32;
}

impl RowID  {
    pub fn to_string(&self, err: *mut OCIError) -> Result<String> {
        let mut text = String::with_capacity(20);
        let txt = unsafe { text.as_mut_vec() };
        let mut len = txt.capacity() as u16;
        catch!{err =>
            OCIRowidToChar(self.get(), txt.as_mut_ptr(), &mut len, err)
        }
        unsafe {
            txt.set_len(len as usize);
        }
        Ok( text )
    }

    pub fn is_initialized(&self) -> bool {
        // This implementation is based on reverse enginnering of the OCIRowid on X64 Windows
        // TODO: check accuracy in multiple environments
        let ptr = self.get() as *const libc::c_void as *const u8;
        // OCIRowid length - 32 - was returned by OCIAttrGet(..., OCI_ATTR_ROWID, ..., OCI_HTYPE_STMT, ...)
        let mem = std::ptr::slice_from_raw_parts(ptr, 32);
        let mem = unsafe { &*mem };
        mem[16..26].iter().any(|&b| b != 0)
    }
}
