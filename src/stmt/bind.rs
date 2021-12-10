//! Binding of parameter placeholders

use super::{SqlInArg, SqlOutArg, Position};
use crate::{Result, Error, oci::{self, *}};
use std::{ptr, collections::{HashMap, HashSet}};
use libc::c_void;

pub(crate) struct Params {
    /// Parameter placeholder (name) indexes
    param_idxs: HashMap<String,usize>,
    /// OCI bind handles
    args_binds: Vec<Ptr<OCIBind>>,
    /// OUT vars returned NULL indicators
    indicators: Vec<i16>,
    /// OUT vars returned data sizes
    data_sizes: Vec<u32>,
}

impl Params {
    pub(super) fn new(stmt: *mut OCIStmt, err: *mut OCIError) -> Result<Option<Self>> {
        let num_binds = attr::get::<u32>(OCI_ATTR_BIND_COUNT, OCI_HTYPE_STMT, stmt as *const c_void, err)? as usize;
        if num_binds == 0 {
            Ok(None)
        } else {
            let mut bind_names      = vec![ptr::null_mut::<u8>(); num_binds];
            let mut bind_name_lens  = vec![0u8; num_binds];
            let mut ind_names       = vec![ptr::null_mut::<u8>(); num_binds];
            let mut ind_name_lens   = vec![0u8; num_binds];
            let mut dups            = vec![0u8; num_binds];
            let mut oci_binds       = vec![ptr::null_mut::<OCIBind>(); num_binds];
            let mut found: i32      = 0;            
            oci::stmt_get_bind_info(
                stmt, err,
                num_binds as u32, 1, &mut found,
                bind_names.as_mut_ptr(), bind_name_lens.as_mut_ptr(),
                ind_names.as_mut_ptr(),  ind_name_lens.as_mut_ptr(),
                dups.as_mut_ptr(),
                oci_binds.as_mut_ptr()
            )?;
            let mut param_idxs = HashMap::with_capacity(num_binds);
            let mut args_binds = Vec::with_capacity(num_binds);
            let mut indicators = Vec::with_capacity(num_binds);
            let mut data_sizes = Vec::with_capacity(num_binds);
            for i in 0..found as usize {
                if dups[i] == 0 {
                    let name = unsafe { std::slice::from_raw_parts(bind_names[i], bind_name_lens[i] as usize) };
                    let name = String::from_utf8_lossy(name).to_string();
                    param_idxs.insert(name, i);
                }
                args_binds.push(Ptr::new(oci_binds[i]));
                indicators.push(OCI_IND_NOTNULL);
                data_sizes.push(0u32);
            }
            Ok(Some(Self{ param_idxs, args_binds, indicators, data_sizes }))
        }
    }

    /// Returns index of the parameter placeholder.
    fn get_parameter_index(&self, name: &str) -> Result<usize> {
        // Try uppercase version of the parameter name first.
        // Explicitly convert to uppercase only if as-is search fails.
        if let Some(&ix) = self.param_idxs.get(&name[1..]) {
            Ok(ix)
        } else if let Some(&ix) = self.param_idxs.get(name[1..].to_uppercase().as_str()) {
            Ok(ix)
        } else {
            Err(Error::new(&format!("Statement does not define {} parameter placeholder", name)))
        }
    }

    /// Binds the argument to a parameter placeholder at the specified position in the SQL statement
    fn bind(&mut self, stmt: *mut OCIStmt, err: *mut OCIError, idx: usize, sql_type: u16, data: *mut c_void, buff_size: usize) -> Result<()> {
        let pos = idx + 1;
        unsafe {
            oci::bind_by_pos(
                stmt, self.args_binds[idx].as_ptr(), err,
                pos as u32,
                data, buff_size as i64, sql_type,
                self.indicators.as_mut_ptr().add(idx),  // Pointer to an indicator variable or array
                self.data_sizes.as_mut_ptr().add(idx),  // Pointer to an array of actual lengths of array elements
                ptr::null_mut::<u16>(), // Pointer to an array of column-level return codes
                0,                      // Maximum array length
                ptr::null_mut::<u32>(), // Pointer to the actual number of elements in the array
                OCI_DEFAULT
            )
        }
    }

    /// Binds provided arguments to SQL parameter placeholders. Returns indexes of parameter placeholders for the OUT args.
    pub(crate) fn bind_args(&mut self, stmt: *mut OCIStmt, err: *mut OCIError, in_args: &[&dyn SqlInArg], out_args: &mut [&mut dyn SqlOutArg]) -> Result<Option<Vec<usize>>> {
        let mut args_idxs : HashSet<_> = self.param_idxs.values().cloned().collect();

        let mut idx = 0;
        for arg in in_args {
            let param_idx = if let Some( name ) = arg.name() { self.get_parameter_index(name)? } else { idx };
            let in_arg = arg.to_sql();
            self.data_sizes[param_idx] = in_arg.sql_data_len() as u32;
            self.bind(stmt, err, param_idx, in_arg.sql_type(), in_arg.sql_data_ptr().get() as _, in_arg.sql_data_len())?;
            args_idxs.remove(&param_idx);
            idx += 1;
        }

        let out_idxs = if out_args.is_empty() {
            None
        } else {
            let mut out_param_idxs = Vec::with_capacity(out_args.len());
            for arg in out_args {
                let param_idx = if let Some( name ) = arg.name() { self.get_parameter_index(name)? } else { idx };
                let out_arg = arg.to_sql_out();
                if out_arg.sql_capacity() == 0 {
                    let msg = if let Some( name ) = arg.name() {
                        format!("Storage capacity of output variable {} is 0", name)
                    } else {
                        format!("Storage capacity of output variable {} is 0", out_param_idxs.len())
                    };
                    return Err(Error::new(&msg));
                }
                self.data_sizes[param_idx] = out_arg.sql_data_len() as u32;
                self.bind(stmt, err, param_idx, out_arg.sql_type(), out_arg.sql_mut_data_ptr().get_mut(), out_arg.sql_capacity())?;
                args_idxs.remove(&param_idx);
                out_param_idxs.push(param_idx);
                idx += 1;
            }
            Some(out_param_idxs)
        };

        // Check whether all placeholders are bound for this execution.
        // While OCIStmtExecute would see missing binds on the first run, the subsequent
        // execution of the same prepared statement might try to reuse previously bound
        // values, and those might already be gone. Hense the explicit check here.
        if !args_idxs.is_empty() {
            Err(Error::new("Not all parameters are bound"))
        } else {
            Ok(out_idxs)
        }
    }

    /// Checks whether the value returned for the output parameter is NULL.
    pub(super) fn is_null(&self, pos: impl Position) -> Result<bool> {
        pos.name()
            .and_then(|name|
                self.param_idxs
                    .get(&name[1..])
                    .or(self.param_idxs.get(name[1..].to_uppercase().as_str()))
            )
            .map(|ix| *ix)
            .or(pos.index())
            .and_then(|ix| self.indicators.get(ix))
            .map(|&null_ind| null_ind == OCI_IND_NULL)
            .ok_or_else(|| Error::new("Parameter not found."))
    }

    /// Returns the size of the returned data for the parameter at the specified 0-based index
    pub(super) fn out_data_len(&self, idx: usize) -> usize {
        self.data_sizes[idx] as usize
    }

}