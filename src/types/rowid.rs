/// The ROWID data type identifies a particular row in a database table.

use crate::{Result, oci::{self, *, attr::AttrGetInto}, ToSql, ToSqlOut, stmt::Params};
use libc::c_void;

pub(crate) fn to_string(rowid: &OCIRowid, err: &OCIError) -> Result<String> {
    let mut text = String::with_capacity(20);
    let txt = unsafe { text.as_mut_vec() };
    let mut len = txt.capacity() as u16;
    oci::rowid_to_char(rowid, txt.as_mut_ptr(), &mut len, err)?;
    unsafe {
        txt.set_len(len as usize);
    }
    Ok( text )
}

pub(crate) fn is_initialized(rowid: &OCIRowid) -> bool {
    // This implementation is based on reverse enginnering of the OCIRowid on x64 Windows
    // TODO: check accuracy in multiple environments
    let ptr: *const u8 = (rowid as *const OCIRowid).cast();
    // OCIRowid length (32) was returned by OCIAttrGet(..., OCI_ATTR_ROWID, ..., OCI_HTYPE_STMT, ...)
    let mem = std::ptr::slice_from_raw_parts(ptr, 32);
    let mem = unsafe { &*mem };
    mem[16..26].iter().any(|&b| b != 0)
}

/// Represents ROWID
pub struct RowID (Descriptor<OCIRowid>);

impl RowID  {
    pub fn new(env: &impl AsRef<OCIEnv>) -> Result<Self> {
        let desc = Descriptor::new(env)?;
        Ok( Self(desc) )
    }

    pub(crate) fn from(rowid: Descriptor<OCIRowid>) -> Self {
        Self(rowid)
    }

    /**
        Returns character representation of a ROWID.

        The returned string can then be used as an argument in SQL statements
        to query a row at the given ROWID.
    */
    pub fn to_string(&self, err: &impl AsRef<OCIError>) -> Result<String> {
        to_string(&self.0, err.as_ref())
    }
}

impl ToSql for RowID {
    fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let len = std::mem::size_of::<*mut OCIRowid>();
        params.bind(pos, SQLT_RDD, self.0.as_ptr() as _, len, stmt, err)?;
        Ok(pos + 1)
    }
}

impl ToSql for &RowID {
    fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let len = std::mem::size_of::<*mut OCIRowid>();
        params.bind(pos, SQLT_RDD, self.0.as_ptr() as _, len, stmt, err)?;
        Ok(pos + 1)
    }
}

impl ToSqlOut for &mut RowID {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let len = std::mem::size_of::<*mut OCIRowid>();
        params.bind_out(pos, SQLT_RDD, self.0.as_mut_ptr() as _, len, len, stmt, err)?;
        Ok(pos + 1)
    }
}

impl AttrGetInto for RowID {
    fn as_mut_ptr(&mut self) -> *mut c_void {
        self.0.get_ptr().get() as _
    }
}