use super::{Params, ToSql};
use crate::{oci::*, Result};

impl ToSql for &str {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind_in(pos, SQLT_CHR, self.as_ptr() as _, self.len(), stmt, err)?;
        Ok(pos + 1)
    }
}

impl ToSql for &&str {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind_in(pos, SQLT_CHR, self.as_ptr() as _, self.len(), stmt, err)?;
        Ok(pos + 1)
    }
}

macro_rules! impl_slice_option {
    ($($t:ty),+ => $sqlt:ident) => {
        $(
            impl ToSql for Option<$t> {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    if let Some(val) = self {
                        params.bind_in(pos, $sqlt, val.as_ptr() as _, val.len(), stmt, err)?;
                    } else {
                        params.bind_null(pos, $sqlt, stmt, err)?;
                    }
                    Ok(pos + 1)
                }
            }
            impl ToSql for &Option<$t> {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    if let Some(val) = self {
                        params.bind_in(pos, $sqlt, val.as_ptr() as _, val.len(), stmt, err)?;
                    } else {
                        params.bind_null(pos, $sqlt, stmt, err)?;
                    }
                    Ok(pos + 1)
                }
            }
            impl ToSql for &mut Option<$t> {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    if let Some(val) = self {
                        params.bind_in(pos, $sqlt, val.as_ptr() as _, val.len(), stmt, err)?;
                    } else {
                        params.bind_null(pos, $sqlt, stmt, err)?;
                    }
                    Ok(pos + 1)
                }
                fn update_from_bind(&mut self, pos: usize, params: &Params) -> Result<usize> {
                    if params.is_null(pos).unwrap_or(true) {
                        self.take();
                    }
                    Ok(pos + 1)
                }
            }
        )+
    };
}

impl_slice_option!{ &str, &&str => SQLT_CHR }

impl ToSql for &[&str] {
    fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        for &txt in self.iter() {
            params.bind_in(pos, SQLT_CHR, txt.as_ptr() as _, txt.len(), stmt, err)?;
            pos += 1;
        }
        Ok(pos)
    }

    fn update_from_bind(&mut self, pos: usize, _params: &Params) -> Result<usize> {
        Ok(pos + self.len())
    }
}

impl ToSql for &[&&str] {
    fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        for &&txt in self.iter() {
            params.bind_in(pos, SQLT_CHR, txt.as_ptr() as _, txt.len(), stmt, err)?;
            pos += 1;
        }
        Ok(pos)
    }

    fn update_from_bind(&mut self, pos: usize, _params: &Params) -> Result<usize> {
        Ok(pos + self.len())
    }
}
