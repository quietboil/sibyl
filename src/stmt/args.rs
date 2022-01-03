//! SQL statement arguments

use super::bind::Params;
use crate::{oci::*, Result};
use std::mem::size_of;

/// A trait for types that can be used as SQL IN arguments
pub trait ToSql : Send + Sync {
    /**
    Binds the value to the SQL IN placeholder

    # Parameters

    - `pos` - zero-based index of the parameter placeholder to which the value will be bound
    - `params` - Statement parameters as defined in the SQL
    - `stmt` - statement to which the argument value will be bound
    - `err` - OCI error structure

    Note that the specified position might be ignored if the argument also provides the specific
    placeholder name to which the value should be bound.

    # Returns

    The index of the placeholder for the next argument.
    */
    fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize>;
}

/// A trait for types that can be used as SQL OUT or INOUT arguments
pub trait ToSqlOut : Send + Sync {
    /**
    Binds the value to the SQL OUT or INOUT placeholder

    # Parameters

    - `pos`  - zero-based index of the parameter placeholder to which the value will be bound
    - `params` - Statement parameters as defined in the SQL
    - `stmt` - OCI statement to which the argument value will be bound
    - `err`  - OCI error structure

    Note that the specified position might be ignored if the argument also provides the specific
    placeholder name to which the value should be bound.

    # Returns

    The index of the placeholder for the next argument.
    */
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize>;

    /**
    Sets the length of the received data.

    This is only applicable to dynamically sized types like `Vec` and `String`
    */
    fn set_len_from_bind(&mut self, _pos: usize, _params: &Params) {}
}

impl ToSql for () {
    fn bind_to(&self, pos: usize, _params: &mut Params, _stmt: &OCIStmt, _err: &OCIError) -> Result<usize> {
        Ok(pos)
    }
}

impl ToSqlOut for () {
    fn bind_to(&mut self, pos: usize, _params: &mut Params, _stmt: &OCIStmt, _err: &OCIError) -> Result<usize> {
        Ok(pos)
    }
}

macro_rules! impl_num_to_sql {
    ($($t:ty),+ => $sqlt:ident) => {
        $(
            impl ToSql for $t {
                fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    let ptr = self as *const $t;
                    let len = size_of::<$t>();
                    params.bind(pos, $sqlt, ptr as _, len, stmt, err)?;
                    Ok(pos + 1)
                }
            }
            impl ToSql for &$t {
                fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    let ptr = *self as *const $t;
                    let len = size_of::<$t>();
                    params.bind(pos, $sqlt, ptr as _, len, stmt, err)?;
                    Ok(pos + 1)
                }
            }
            impl ToSqlOut for &mut $t {
                fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                    let ptr = *self as *mut $t;
                    let len = size_of::<$t>();
                    params.bind_out(pos, $sqlt, ptr as _, len, len, stmt, err)?;
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

impl ToSql for &str {
    fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind(pos, SQLT_CHR, (*self).as_ptr() as _, (*self).len(), stmt, err)?;
        Ok(pos + 1)
    }
}

impl ToSql for &&str {
    fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind(pos, SQLT_CHR, (**self).as_ptr() as _, (**self).len(), stmt, err)?;
        Ok(pos + 1)
    }
}

impl ToSql for &[u8] {
    fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind(pos, SQLT_LBI, (*self).as_ptr() as _, (*self).len(), stmt, err)?;
        Ok(pos + 1)
    }
}

impl ToSql for &&[u8] {
    fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind(pos, SQLT_LBI, (**self).as_ptr() as _, (**self).len(), stmt, err)?;
        Ok(pos + 1)
    }
}

impl ToSqlOut for &mut String {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind_out(pos, SQLT_CHR, unsafe { self.as_mut_vec().as_mut_ptr() } as _, self.len(), self.capacity(), stmt, err)?;
        Ok(pos + 1)
    }

    fn set_len_from_bind(&mut self, pos: usize, params: &Params) {
        let new_len = params.out_data_len(pos);
        unsafe {
            self.as_mut_vec().set_len(new_len)
        }
    }
}

impl ToSqlOut for &mut Vec<u8> {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind_out(pos, SQLT_LBI, (*self).as_mut_ptr() as _, (*self).len(), (*self).capacity(), stmt, err)?;
        Ok(pos + 1)
    }

    fn set_len_from_bind(&mut self, pos: usize, params: &Params) {
        let new_len = params.out_data_len(pos);
        unsafe {
            self.set_len(new_len)
        }
    }
}

impl ToSqlOut for &mut [u8] {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind_out(pos, SQLT_LBI, (*self).as_mut_ptr() as _, 0, (*self).len(), stmt, err)?;
        Ok(pos + 1)
    }
}

impl ToSqlOut for &mut &mut [u8] {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind_out(pos, SQLT_LBI, (**self).as_mut_ptr() as _, 0, (**self).len(), stmt, err)?;
        Ok(pos + 1)
    }
}

impl<T> ToSql for (&str, T) where T: ToSql {
    fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let idx = params.index_of(self.0)?;
        self.1.bind_to(idx, params, stmt, err)?;
        Ok(pos)
    }
}

impl<T1,T2> ToSql for ((&str, T1), (&str, T2)) where T1: ToSql, T2: ToSql {
    fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let idx = params.index_of(self.0.0)?;
        self.0.1.bind_to(idx, params, stmt, err)?;
        let idx = params.index_of(self.1.0)?;
        self.1.1.bind_to(idx, params, stmt, err)?;
        Ok(pos)
    }
}

impl<T> ToSqlOut for (&str, T) where T: ToSqlOut {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let idx = params.index_of(self.0)?;
        self.1.bind_to(idx, params, stmt, err)?;
        Ok(pos)
    }

    fn set_len_from_bind(&mut self, pos: usize, params: &Params) {
        self.1.set_len_from_bind(pos, params);
    }
}

impl<T1,T2> ToSqlOut for ((&str, T1), (&str, T2)) where T1: ToSqlOut, T2: ToSqlOut {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let idx = params.index_of(self.0.0)?;
        self.0.1.bind_to(idx, params, stmt, err)?;
        let idx = params.index_of(self.1.0)?;
        self.1.1.bind_to(idx, params, stmt, err)?;
        Ok(pos)
    }

    fn set_len_from_bind(&mut self, pos: usize, params: &Params) {
        self.0.1.set_len_from_bind(pos, params);
        self.1.1.set_len_from_bind(pos + 1, params);
    }
}

macro_rules! impl_tuple_args {
    ($($name:ident)+) => {
        impl<$($name),+> ToSql for ($($name),+) where $($name: ToSql),+ {
            #[allow(non_snake_case)]
            fn bind_to(&self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                let ($(ref $name),+) = *self;
                $(
                    let pos = $name.bind_to(pos, params, stmt, err)?;
                )+
                Ok(pos)
            }
        }
        impl<$($name),+> ToSqlOut for ($($name),+) where $($name: ToSqlOut),+ {
            #[allow(non_snake_case)]
            fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                let ($(ref mut $name),+) = *self;
                $(
                    let pos = $name.bind_to(pos, params, stmt, err)?;
                )+
                Ok(pos)
            }
            #[allow(non_snake_case,unused_assignments)]
            fn set_len_from_bind(&mut self, mut pos: usize, params: &Params) {
                let ($(ref mut $name),+) = *self;
                $(
                    $name.set_len_from_bind(pos, params);
                    pos += 1;
                )+
            }
        }
    };
}

impl_tuple_args! { A B C }
impl_tuple_args! { A B C D }
impl_tuple_args! { A B C D E }
impl_tuple_args! { A B C D E F }
impl_tuple_args! { A B C D E F G }
impl_tuple_args! { A B C D E F G H }
impl_tuple_args! { A B C D E F G H I }
impl_tuple_args! { A B C D E F G H I J }
impl_tuple_args! { A B C D E F G H I J K }
impl_tuple_args! { A B C D E F G H I J K L }
impl_tuple_args! { A B C D E F G H I J K L M }
impl_tuple_args! { A B C D E F G H I J K L M N }
impl_tuple_args! { A B C D E F G H I J K L M N O }
impl_tuple_args! { A B C D E F G H I J K L M N O P }
impl_tuple_args! { A B C D E F G H I J K L M N O P Q }
impl_tuple_args! { A B C D E F G H I J K L M N O P Q R }
impl_tuple_args! { A B C D E F G H I J K L M N O P Q R S }
impl_tuple_args! { A B C D E F G H I J K L M N O P Q R S T }
impl_tuple_args! { A B C D E F G H I J K L M N O P Q R S T U }
impl_tuple_args! { A B C D E F G H I J K L M N O P Q R S T U V }
impl_tuple_args! { A B C D E F G H I J K L M N O P Q R S T U V W }
impl_tuple_args! { A B C D E F G H I J K L M N O P Q R S T U V W X }
impl_tuple_args! { A B C D E F G H I J K L M N O P Q R S T U V W X Y }
impl_tuple_args! { A B C D E F G H I J K L M N O P Q R S T U V W X Y Z }
