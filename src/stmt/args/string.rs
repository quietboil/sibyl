use super::{Params, ToSql};
use crate::{oci::*, Result};

impl ToSql for String {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind(pos, SQLT_CHR, self.as_ptr() as _, self.len(), self.capacity(), stmt, err)?;
        Ok(pos + 1)
    }

    fn update_from_bind(&mut self, pos: usize, params: &Params) {
        let new_len = params.get_data_len(pos);
        unsafe {
            self.as_mut_vec().set_len(new_len);
        }
    }
}

impl ToSql for &String {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind_in(pos, SQLT_CHR, self.as_ptr() as _, self.len(), stmt, err)?;
        Ok(pos + 1)
    }
}

impl ToSql for &mut String {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind(pos, SQLT_CHR, self.as_mut_ptr() as _, self.len(), self.capacity(), stmt, err)?;
        Ok(pos + 1)
    }

    fn update_from_bind(&mut self, pos: usize, params: &Params) {
        let new_len = params.get_data_len(pos);
        unsafe {
            self.as_mut_vec().set_len(new_len);
        }
    }
}

impl_sql_type!{ String, &String, &mut String => SQLT_CHR }

impl ToSql for Option<String> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        if let Some(val) = self {
            params.bind(pos, SQLT_CHR, val.as_ptr() as _, val.len(), val.capacity(), stmt, err)?;
        } else {
            params.bind_null(pos, SQLT_CHR, stmt, err)?;
        }
        Ok(pos + 1)
    }
}

impl ToSql for Option<&String> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        if let Some(val) = self {
            params.bind_in(pos, SQLT_CHR, val.as_ptr() as _, val.len(), stmt, err)?;
        } else {
            params.bind_null(pos, SQLT_CHR, stmt, err)?;
        }
        Ok(pos + 1)
    }
}

impl ToSql for Option<&mut String> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        if let Some(val) = self {
            params.bind(pos, SQLT_CHR, val.as_ptr() as _, val.len(), val.capacity(), stmt, err)?;
        } else {
            params.bind_null(pos, SQLT_CHR, stmt, err)?;
        }
        Ok(pos + 1)
    }

    fn update_from_bind(&mut self, pos: usize, params: &Params) {
        if let Some(val) = self {
            let new_len = params.get_data_len(pos);
            unsafe {
                val.as_mut_vec().set_len(new_len);
            }
        }
    }
}

impl ToSql for &Option<String> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        if let Some(val) = self {
            params.bind_in(pos, SQLT_CHR, val.as_ptr() as _, val.len(), stmt, err)?;
        } else {
            params.bind_null(pos, SQLT_CHR, stmt, err)?;
        }
        Ok(pos + 1)
    }
}

impl ToSql for &Option<&String> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        if let Some(val) = self {
            params.bind_in(pos, SQLT_CHR, val.as_ptr() as _, val.len(), stmt, err)?;
        } else {
            params.bind_null(pos, SQLT_CHR, stmt, err)?;
        }
        Ok(pos + 1)
    }
}

impl ToSql for &Option<&mut String> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        if let Some(val) = self {
            params.bind_in(pos, SQLT_CHR, val.as_ptr() as _, val.len(), stmt, err)?;
        } else {
            params.bind_null(pos, SQLT_CHR, stmt, err)?;
        }
        Ok(pos + 1)
    }
}

impl ToSql for &mut Option<String> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        if let Some(val) = self {
            params.bind(pos, SQLT_CHR, val.as_ptr() as _, val.len(), val.capacity(), stmt, err)?;
        } else {
            params.bind_null(pos, SQLT_CHR, stmt, err)?;
        }
        Ok(pos + 1)
    }

    fn update_from_bind(&mut self, pos: usize, params: &Params) {
        if params.is_null(pos).unwrap_or(true) {
            self.take();
        } else if let Some(val) = self {
            let new_len = params.get_data_len(pos);
            unsafe {
                val.as_mut_vec().set_len(new_len);
            }
        } else if let Some(val) = params.get_data_as_bytes(pos) {
            let new_str = String::from_utf8_lossy(val).into_owned();
            self.replace(new_str);
        }
    }
}

impl ToSql for &mut Option<&String> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        if let Some(val) = self {
            params.bind_in(pos, SQLT_CHR, val.as_ptr() as _, val.len(), stmt, err)?;
        } else {
            params.bind_null(pos, SQLT_CHR, stmt, err)?;
        }
        Ok(pos + 1)
    }
}

impl ToSql for &mut Option<&mut String> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        if let Some(val) = self {
            params.bind(pos, SQLT_CHR, val.as_ptr() as _, val.len(), val.capacity(), stmt, err)?;
        } else {
            // There is nothing we can do if they passed None as we cannot insert mut ref back into Option
            params.bind_null(pos, SQLT_CHR, stmt, err)?;
        }
        Ok(pos + 1)
    }

    fn update_from_bind(&mut self, pos: usize, params: &Params) {
        if params.is_null(pos).unwrap_or(true) {
            self.take();
        } else if let Some(val) = self {
            let new_len = params.get_data_len(pos);
            unsafe {
                val.as_mut_vec().set_len(new_len);
            }
        }
    }
}

impl ToSql for &[String] {
    fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        for txt in self.iter() {
            params.bind_in(pos, SQLT_CHR, txt.as_ptr() as _, txt.len(), stmt, err)?;
            pos += 1;
        }
        Ok(pos)
    }
}

impl ToSql for &[&String] {
    fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        for &txt in self.iter() {
            params.bind_in(pos, SQLT_CHR, txt.as_ptr() as _, txt.len(), stmt, err)?;
            pos += 1;
        }
        Ok(pos)
    }
}

impl ToSql for &mut [String] {
    fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        for txt in self.iter_mut() {
            params.bind(pos, SQLT_CHR, txt.as_mut_ptr() as _, txt.len(), txt.capacity(), stmt, err)?;
            pos += 1;
        }
        Ok(pos)
    }

    fn update_from_bind(&mut self, pos: usize, params: &Params) {
        for txt in self.iter_mut() {
            let new_len = params.get_data_len(pos);
            unsafe {
                (*txt).as_mut_vec().set_len(new_len)
            }
        }
    }
}

impl ToSql for &mut [&mut String] {
    fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        for txt in self.iter_mut() {
            params.bind(pos, SQLT_CHR, txt.as_mut_ptr() as _, txt.len(), txt.capacity(), stmt, err)?;
            pos += 1;
        }
        Ok(pos)
    }

    fn update_from_bind(&mut self, pos: usize, params: &Params) {
        for txt in self.iter_mut() {
            let new_len = params.get_data_len(pos);
            unsafe {
                (*txt).as_mut_vec().set_len(new_len)
            }
        }
    }
}

