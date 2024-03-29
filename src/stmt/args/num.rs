use super::{Params, ToSql};
use crate::{oci::*, Result};
use std::mem::size_of;

macro_rules! impl_num_to_sql {
    ($($t:ty),+ => $sqlt:ident) => {
        $(
            impl ToSql for $t {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    params.bind(pos, $sqlt, self as *const $t as _, size_of::<$t>(), size_of::<$t>(), stmt, err)?;
                    Ok(pos + 1)
                }
            }
            impl ToSql for &$t {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    params.bind_in(pos, $sqlt, *self as *const $t as _, size_of::<$t>(), stmt, err)?;
                    Ok(pos + 1)
                }
            }
            impl ToSql for &mut $t {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    params.bind(pos, $sqlt, *self as *mut $t as _, size_of::<$t>(), size_of::<$t>(), stmt, err)?;
                    Ok(pos + 1)
                }
            }

            impl ToSql for Option<$t> {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    if let Some(val) = self {
                        params.bind(pos, $sqlt, val as *const $t as _, size_of::<$t>(), size_of::<$t>(), stmt, err)?;
                    } else {
                        params.bind_null(pos, $sqlt, stmt, err)?;
                    }
                    Ok(pos + 1)
                }
            }
            impl ToSql for Option<&$t> {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    if let Some(val) = self {
                        params.bind_in(pos, $sqlt, *val as *const $t as _, size_of::<$t>(), stmt, err)?;
                    } else {
                        params.bind_null(pos, $sqlt, stmt, err)?;
                    }
                    Ok(pos + 1)
                }
            }
            impl ToSql for Option<&mut $t> {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    if let Some(val) = self {
                        params.bind(pos, $sqlt, *val as *mut $t as _, size_of::<$t>(), size_of::<$t>(), stmt, err)?;
                    } else {
                        // There is nothing we can do if they passed None as we cannot insert mut ref back into Option
                        params.bind_null(pos, $sqlt, stmt, err)?;
                    }
                    Ok(pos + 1)
                }
            }
            impl ToSql for &Option<$t> {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    if let Some(val) = self {
                        params.bind_in(pos, $sqlt, val as *const $t as _, size_of::<$t>(), stmt, err)?;
                    } else {
                        params.bind_null(pos, $sqlt, stmt, err)?;
                    }
                    Ok(pos + 1)
                }
            }
            impl ToSql for &Option<&$t> {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    if let Some(val) = self {
                        params.bind_in(pos, $sqlt, *val as *const $t as _, size_of::<$t>(), stmt, err)?;
                    } else {
                        params.bind_null(pos, $sqlt, stmt, err)?;
                    }
                    Ok(pos + 1)
                }
            }
            impl ToSql for &Option<&mut $t> {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    if let Some(val) = self {
                        params.bind_in(pos, $sqlt, *val as *const $t as _, size_of::<$t>(), stmt, err)?;
                    } else {
                        params.bind_null(pos, $sqlt, stmt, err)?;
                    }
                    Ok(pos + 1)
                }
            }
            impl ToSql for &mut Option<$t> {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    if let Some(val) = self {
                        params.bind(pos, $sqlt, val as *mut $t as _, size_of::<$t>(), size_of::<$t>(), stmt, err)?;
                    } else {
                        params.bind_null_mut(pos, $sqlt, size_of::<$t>(), stmt, err)?;
                    }
                    Ok(pos + 1)
                }
                fn update_from_bind(&mut self, pos: usize, params: &Params) -> Result<usize> {
                    if params.is_null(pos).unwrap_or(true) {
                        self.take();
                    } else if self.is_some() {
                        // has been updated by OCI
                    } else if let Some(val) = params.get_data_as_ref(pos) {
                        self.replace(*val);
                    }
                    Ok(pos + 1)
                }
            }
            impl ToSql for &mut Option<&$t> {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    if let Some(val) = self {
                        params.bind_in(pos, $sqlt, *val as *const $t as _, size_of::<$t>(), stmt, err)?;
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
            impl ToSql for &mut Option<&mut $t> {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    if let Some(val) = self {
                        params.bind(pos, $sqlt, *val as *mut $t as _, size_of::<$t>(), size_of::<$t>(), stmt, err)?;
                    } else {
                        // There is nothing we can do if they passed None as we cannot insert mut ref back into Option
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

impl_num_to_sql!{ i8, i16, i32, i64, isize => SQLT_INT }
impl_num_to_sql!{ u8, u16, u32, u64, usize => SQLT_UIN }
impl_num_to_sql!{ f32 => SQLT_BFLOAT }
impl_num_to_sql!{ f64 => SQLT_BDOUBLE }

macro_rules! impl_num_slice_to_sql {
    ($($t:ty),+ => $sqlt:ident) => {
        $(
            impl ToSql for &[$t] {
                fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    for num in self.iter() {
                        params.bind_in(pos, $sqlt, num as *const $t as _, size_of::<$t>(), stmt, err)?;
                        pos += 1;
                    }
                    Ok(pos)
                }
                fn update_from_bind(&mut self, pos: usize, _params: &Params) -> Result<usize> {
                    Ok(pos + self.len())
                }
            }
            impl ToSql for &[&$t] {
                fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    for &num in self.iter() {
                        params.bind_in(pos, $sqlt, num as *const $t as _, size_of::<$t>(), stmt, err)?;
                        pos += 1;
                    }
                    Ok(pos)
                }
                fn update_from_bind(&mut self, pos: usize, _params: &Params) -> Result<usize> {
                    Ok(pos + self.len())
                }
            }
            impl ToSql for &mut [&mut $t] {
                fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    for num in self.iter_mut() {
                        params.bind(pos, $sqlt, *num as *mut $t as _, size_of::<$t>(), size_of::<$t>(), stmt, err)?;
                        pos += 1;
                    }
                    Ok(pos)
                }
                fn update_from_bind(&mut self, pos: usize, _params: &Params) -> Result<usize> {
                    Ok(pos + self.len())
                }
            }
        )+
    };
}

impl_num_slice_to_sql!{ i8, i16, i32, i64, isize => SQLT_INT }
impl_num_slice_to_sql!{ u16, u32, u64, usize => SQLT_UIN }
impl_num_slice_to_sql!{ f32 => SQLT_BFLOAT }
impl_num_slice_to_sql!{ f64 => SQLT_BDOUBLE }

impl_sql_type!{ i8, i16, i32, i64, isize => SQLT_INT }
impl_sql_type!{ &i8, &i16, &i32, &i64, &isize => SQLT_INT }
impl_sql_type!{ &mut i8, &mut i16, &mut i32, &mut i64, &mut isize => SQLT_INT }
impl_sql_type!{ u16, u32, u64, usize => SQLT_UIN }
impl_sql_type!{ &u16, &u32, &u64, &usize => SQLT_UIN }
impl_sql_type!{ &mut u16, &mut u32, &mut u64, &mut usize => SQLT_UIN }
impl_sql_type!{ f32, &f32, &mut f32 => SQLT_BFLOAT }
impl_sql_type!{ f64, &f64, &mut f64 => SQLT_BDOUBLE }
