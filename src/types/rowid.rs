/// The ROWID data type identifies a particular row in a database table.

use crate::{Result, RowID, env::Env, oci::{self, *, attr::AttrGetInto}, stmt::args::{ToSql, ToSqlOut}, ptr::{ScopedPtr, ScopedMutPtr}};
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
        oci::rowid_to_char(self.get(), txt.as_mut_ptr(), &mut len, env.err_ptr())?;
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
    fn sql_type(&self) -> u16 { SQLT_RDD }
    fn sql_data_ptr(&self) -> ScopedPtr<c_void> { ScopedPtr::new(self.as_ptr() as _) }
    fn sql_data_len(&self) -> usize { std::mem::size_of::<*mut OCIRowid>() }
}

impl ToSqlOut for RowID {
    fn sql_type(&self) -> u16 { SQLT_RDD }
    fn sql_mut_data_ptr(&mut self) -> ScopedMutPtr<c_void> { ScopedMutPtr::new(self.as_mut_ptr() as _) }
    fn sql_data_len(&self) -> usize { std::mem::size_of::<*mut OCIRowid>() }
}

impl AttrGetInto for RowID {
    fn as_val_ptr(&mut self) -> *mut c_void { self.get() as *mut c_void }
    fn capacity(&self) -> usize             { 0 }
    fn set_len(&mut self, _new_len: usize)  { }
}
