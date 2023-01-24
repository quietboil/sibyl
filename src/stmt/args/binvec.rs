use super::{Params, ToSql};
use crate::{oci::*, Result};

impl ToSql for Vec<u8> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind_in(pos, SQLT_LBI, self.as_ptr() as _, self.len(), stmt, err)?;
        Ok(pos + 1)
    }
}

impl ToSql for &Vec<u8> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind_in(pos, SQLT_LBI, self.as_ptr() as _, self.len(), stmt, err)?;
        Ok(pos + 1)
    }
}

impl ToSql for &mut Vec<u8> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind(pos, SQLT_LBI, self.as_mut_ptr() as _, self.len(), self.capacity(), stmt, err)?;
        Ok(pos + 1)
    }

    fn update_from_bind(&mut self, pos: usize, params: &Params) {
        let new_len = params.out_data_len(pos);
        unsafe {
            self.set_len(new_len)
        }
    }
}

impl ToSql for Option<Vec<u8>> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        if let Some(val) = self {
            params.bind(pos, SQLT_LBI, val.as_ptr() as _, val.len(), val.capacity(), stmt, err)?;
        } else {
            params.bind_null(pos, SQLT_LBI, stmt, err)?;
        }
        Ok(pos + 1)
    }
}

impl ToSql for Option<&Vec<u8>> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        if let Some(val) = self {
            params.bind_in(pos, SQLT_LBI, val.as_ptr() as _, val.len(), stmt, err)?;
        } else {
            params.bind_null(pos, SQLT_LBI, stmt, err)?;
        }
        Ok(pos + 1)
    }
}

impl ToSql for Option<&mut Vec<u8>> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        if let Some(val) = self {
            params.bind(pos, SQLT_LBI, val.as_ptr() as _, val.len(), val.capacity(), stmt, err)?;
        } else {
            params.bind_null(pos, SQLT_LBI, stmt, err)?;
        }
        Ok(pos + 1)
    }

    fn update_from_bind(&mut self, pos: usize, params: &Params) {
        if let Some(val) = self {
            let new_len = params.out_data_len(pos);
            unsafe {
                val.set_len(new_len);
            }
        }
    }
}

impl ToSql for &Option<Vec<u8>> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        if let Some(val) = self {
            params.bind_in(pos, SQLT_LBI, val.as_ptr() as _, val.len(), stmt, err)?;
        } else {
            params.bind_null(pos, SQLT_LBI, stmt, err)?;
        }
        Ok(pos + 1)
    }
}

impl ToSql for &Option<&Vec<u8>> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        if let Some(val) = self {
            params.bind_in(pos, SQLT_LBI, val.as_ptr() as _, val.len(), stmt, err)?;
        } else {
            params.bind_null(pos, SQLT_LBI, stmt, err)?;
        }
        Ok(pos + 1)
    }
}

impl ToSql for &Option<&mut Vec<u8>> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        if let Some(val) = self {
            params.bind_in(pos, SQLT_LBI, val.as_ptr() as _, val.len(), stmt, err)?;
        } else {
            params.bind_null(pos, SQLT_LBI, stmt, err)?;
        }
        Ok(pos + 1)
    }
}

impl ToSql for &mut Option<Vec<u8>> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        if let Some(val) = self {
            params.bind(pos, SQLT_LBI, val.as_ptr() as _, val.len(), val.capacity(), stmt, err)?;
        } else {
            params.bind_null(pos, SQLT_LBI, stmt, err)?;
        }
        Ok(pos + 1)
    }

    fn update_from_bind(&mut self, pos: usize, params: &Params) {
        if params.is_null(pos).unwrap_or(true) {
            self.take();
        } else if let Some(val) = self {
            let new_len = params.out_data_len(pos);
            unsafe {
                val.set_len(new_len);
            }
        } else if let Some(val) = params.get_data_as_bytes(pos) {
            self.replace(val.to_vec());
        }
    }
}

impl ToSql for &mut Option<&Vec<u8>> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        if let Some(val) = self {
            params.bind_in(pos, SQLT_LBI, val.as_ptr() as _, val.len(), stmt, err)?;
        } else {
            params.bind_null(pos, SQLT_LBI, stmt, err)?;
        }
        Ok(pos + 1)
    }
}

impl ToSql for &mut Option<&mut Vec<u8>> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        if let Some(val) = self {
            params.bind(pos, SQLT_LBI, val.as_ptr() as _, val.len(), val.capacity(), stmt, err)?;
        } else {
            // There is nothing we can do if they passed None as we cannot insert mut ref back into Option
            params.bind_null(pos, SQLT_LBI, stmt, err)?;
        }
        Ok(pos + 1)
    }

    fn update_from_bind(&mut self, pos: usize, params: &Params) {
        if params.is_null(pos).unwrap_or(true) {
            self.take();
        } else if let Some(val) = self {
            let new_len = params.out_data_len(pos);
            unsafe {
                val.set_len(new_len);
            }
        }
    }
}
