/// The ROWID data type identifies a particular row in a database table.

use crate::{Result, oci::{self, *, attr::AttrGetInto}};
use libc::c_void;

mod tosql;

pub(crate) fn to_string(rowid: &OCIRowid, err: &OCIError) -> Result<String> {
    // Attempt to stringify new/uninitialized rowid leads to SIGSEGV
    if !is_initialized(rowid) {
        return Err(crate::Error::new("RowID is not initialized"));
    }
    let mut text = String::with_capacity(20);
    let txt = unsafe { text.as_mut_vec() };
    let mut len = txt.capacity();
    oci::rowid_to_char(rowid, txt.as_mut_ptr(), &mut len as *mut usize as _, err)?;
    unsafe {
        txt.set_len(len);
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
    /// Creates an unitialized `RowID`. These are used as output arguments.
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

impl AttrGetInto for RowID {
    fn as_mut_ptr(&mut self) -> *mut c_void {
        self.0.get_ptr().get() as _
    }
}