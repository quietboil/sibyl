/// The ROWID data type identifies a particular row in a database table.

use crate::{Result, catch, RowID, env::Env, oci::{*, attr::AttrGetInto}, stmt::args::{ToSql, ToSqlOut}};
use libc::c_void;

impl RowID  {
    /**
        Returns character representation of a ROWID.

        The returned string can then be used as an argument in SQL statements
        to query a row at the given ROWID.
    */
    pub fn to_string(&self, env: &dyn Env) -> Result<String> {
        let mut text = String::with_capacity(20);
        let txt = unsafe { text.as_mut_vec() };
        let mut len = txt.capacity() as u16;
        catch!{env.err_ptr() =>
            OCIRowidToChar(self.get(), txt.as_mut_ptr(), &mut len, env.err_ptr())
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

impl AttrGetInto for RowID {
    fn as_val_ptr(&mut self) -> *mut c_void { self.get() as *mut c_void }
    fn capacity(&self) -> usize             { 0 }
    fn set_len(&mut self, _new_len: usize)  { }
}
