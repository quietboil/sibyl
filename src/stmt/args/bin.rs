use super::{Params, ToSql};
use crate::{oci::*, Result};

impl ToSql for &[u8] {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind_in(pos, SQLT_LBI, self.as_ptr() as _, self.len(), stmt, err)?;
        Ok(pos + 1)
    }
}

impl ToSql for &&[u8] {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind_in(pos, SQLT_LBI, self.as_ptr() as _, self.len(), stmt, err)?;
        Ok(pos + 1)
    }
}

impl ToSql for &mut [u8] {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind(pos, SQLT_LBI, self.as_mut_ptr() as _, self.len(), self.len(), stmt, err)?;
        Ok(pos + 1)
    }
}

impl ToSql for & mut & mut [u8] {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind(pos, SQLT_LBI, self.as_mut_ptr() as _, self.len(), self.len(), stmt, err)?;
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
            }
        )+
    };
}

impl_slice_option!{ &[u8], &&[u8] => SQLT_LBI }

macro_rules! impl_mut_bin_slice_option {
    ($($t:ty),+ => $sqlt:ident, $max_len:expr) => {
        $(
            impl ToSql for Option<$t> {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    if let Some(val) = self {
                        params.bind(pos, $sqlt, val.as_mut_ptr() as _, val.len(), val.len(), stmt, err)?;
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
                        params.bind(pos, $sqlt, val.as_mut_ptr() as _, val.len(), val.len(), stmt, err)?;
                    } else {
                        params.bind_null(pos, $sqlt, stmt, err)?;
                    }
                    Ok(pos + 1)
                }
                fn update_from_bind(&mut self, pos: usize, params: &Params) {
                    if params.is_null(pos).unwrap_or(true) {
                        self.take();
                    }
                }
            }
        )+
    };
}

impl_mut_bin_slice_option!{ &mut [u8], &mut &mut [u8] => SQLT_LBI, 2000 }

