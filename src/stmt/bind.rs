//! Binding of parameter placeholders

use super::Position;
use crate::{Result, Error, oci::{self, *}, ToSql, ToSqlOut};
use std::{ptr, collections::HashMap};
use libc::c_void;


/// OUT (and INOUT) argument NULL indicators and return data sizes
struct OutInfo {
    /// size of returned data
    data_size: u32,
    /// NULL indicator
    indicator: i16,
    /// Index of this OUT bind
    bind_index: u16,
}

impl OutInfo {
    fn new(index: usize, ind: i16, data_len: usize) -> Self {
        Self {
            data_size: data_len as _,
            indicator: ind,
            bind_index: index as _,
        }
    }
}

pub struct Params {
    /// Parameter placeholder (name) indexes
    idxs: HashMap<&'static str,usize>,
    /// Parameter names
    names: HashMap<usize,&'static str>,
    /// OCI bind handles
    binds: Vec<Ptr<OCIBind>>,
    /// Bit "vector" of binds that have been established before
    current_binds: u64,
    /// NULL indicators and returned data sizes for OUT variables (if any)
    out_info: Vec<OutInfo>,
}

impl Params {
    pub(super) fn new(stmt: &OCIStmt, err: &OCIError) -> Result<Option<Self>> {
        let num_binds : u32 = attr::get(OCI_ATTR_BIND_COUNT, OCI_HTYPE_STMT, stmt, err)?;
        if num_binds == 0 {
            Ok(None)
        } else {
            let num_binds = num_binds as usize;
            let mut idxs  = HashMap::with_capacity(num_binds);
            let mut names = HashMap::with_capacity(num_binds);
            let mut binds = Vec::with_capacity(num_binds);

            let mut bind_names      = vec![     ptr::null_mut::<u8>(); num_binds];
            let mut bind_name_lens  = vec![                       0u8; num_binds];
            let mut ind_names       = vec![     ptr::null_mut::<u8>(); num_binds];
            let mut ind_name_lens   = vec![                       0u8; num_binds];
            let mut dups            = vec![                       0u8; num_binds];
            let mut oci_binds       = vec![ptr::null_mut::<OCIBind>(); num_binds];
            let mut found: i32      = 0;

            oci::stmt_get_bind_info(
                stmt, err,
                num_binds as u32, 1, &mut found,
                bind_names.as_mut_ptr(), bind_name_lens.as_mut_ptr(),
                ind_names.as_mut_ptr(),  ind_name_lens.as_mut_ptr(),
                dups.as_mut_ptr(),       oci_binds.as_mut_ptr()
            )?;

            for i in 0..found as usize {
                if dups[i] == 0 {
                    let name = unsafe { std::slice::from_raw_parts(bind_names[i], bind_name_lens[i] as usize) };
                    let name = unsafe { std::str::from_utf8_unchecked(name) };
                    // The `idxs` and `names` hash maps won't outlive `Params` and the latter won't outlive `Statement`.
                    // While `str` for names that we created above will only live as long as the containing `Statement`,
                    // within `Params` they can be seen as static as they will be alive longer.
                    idxs.insert(name, i);
                    names.insert(i, name);
                }
                binds.push(Ptr::new(oci_binds[i]));
            }

            Ok(Some(Self{ idxs, names, binds, current_binds: 0, out_info: Vec::new() }))
        }
    }

    /// Returns index of the parameter placeholder.
    pub(crate) fn index_of(&self, name: &str) -> Result<usize> {
        // Assume `name` is already uppercase and use it as-is first.
        // Explicitly convert to uppercase only if as-is search fails.
        if let Some(&ix) = self.idxs.get(&name[1..]) {
            Ok(ix)
        } else if let Some(&ix) = self.idxs.get(name[1..].to_uppercase().as_str()) {
            Ok(ix)
        } else {
            Err(Error::new(&format!("Statement does not define parameter placeholder {}", name)))
        }
    }

    /// Binds an IN argument to a parameter placeholder at the specified position in the SQL statement.
    pub(crate) fn bind(&mut self, idx: usize, sql_type: u16, data_ptr: *mut c_void, data_len: usize, stmt: &OCIStmt, err: &OCIError) -> Result<()> {
        self.current_binds |= 1 << idx;
        oci::bind_by_pos(
            stmt, self.binds[idx].as_mut_ptr(), err,
            (idx + 1) as u32, data_ptr, data_len as i64, sql_type,
            ptr::null_mut(), ptr::null_mut(),
            OCI_DEFAULT
        )
    }

    /// Binds an OUT argument to a parameter placeholder at the specified position in the SQL statement.
    pub(crate) fn bind_out(&mut self, idx: usize, sql_type: u16, data_ptr: *mut c_void, data_len: usize, buff_size: usize, stmt: &OCIStmt, err: &OCIError) -> Result<()> {
        if buff_size == 0 {
            let msg = if let Some(name) = self.names.get(&idx) {
                format!("Storage capacity of output variable {} is 0", name)
            } else {
                format!("Storage capacity of output variable {} is 0", idx)
            };
            return Err(Error::msg(msg));
        }
        self.current_binds |= 1 << idx;
        let out_idx = self.out_info.len();
        self.out_info.push(OutInfo::new(idx, OCI_IND_NOTNULL, data_len));
        oci::bind_by_pos(
            stmt, self.binds[idx].as_mut_ptr(), err,
            (idx + 1) as u32, data_ptr, buff_size as i64, sql_type,
            &mut self.out_info[out_idx].indicator,  // Pointer to an indicator variable or array
            &mut self.out_info[out_idx].data_size,  // Pointer to an array of actual lengths of array elements
            OCI_DEFAULT
        )
    }

    /// Binds provided arguments to SQL parameter placeholders. Returns indexes of parameter placeholders for the OUT args.
    pub(crate) fn bind_args(&mut self, stmt: &OCIStmt, err: &OCIError, in_args: &impl ToSql, out_args: &mut impl ToSqlOut) -> Result<()> {
        let prior_binds = self.current_binds;
        self.current_binds = 0;
        self.out_info.clear();
        let pos = in_args.bind_to(0, self, stmt, err)?;
        out_args.bind_to(pos, self, stmt, err)?;
        if (prior_binds ^ self.current_binds) & prior_binds != 0 {
            Err(Error::new("not all existing binds have been updated"))
        } else {
            Ok(())
        }
    }

    pub(crate) fn set_out_data_len(&self, out_args: &mut impl ToSqlOut) {
        out_args.set_len_from_bind(0, self);
    }

    /// Checks whether the value returned for the output parameter is NULL.
    pub(super) fn is_null(&self, pos: impl Position) -> Result<bool> {
        pos.name()
            .and_then(|name|
                self.idxs
                    .get(&name[1..])
                    .or(self.idxs.get(name[1..].to_uppercase().as_str()))
            )
            .map(|ix| *ix)
            .or(pos.index())
            .and_then(|ix| self.out_info.iter().find(|&info| info.bind_index == ix as u16))
            .map(|out_info| out_info.indicator == OCI_IND_NULL)
            .ok_or_else(|| Error::new("Parameter not found."))
    }

    /// Returns the size of the returned data for the OUT parameter at the specified OUT index
    pub(super) fn out_data_len(&self, idx: usize) -> usize {
        self.out_info
            .get(idx)
            .map(|out_info| out_info.data_size as usize)
            .unwrap_or_default()
    }
}
