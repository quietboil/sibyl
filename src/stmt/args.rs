//! SQL statement arguments

mod num;
mod str;
mod string;
mod bin;
mod binvec;
mod bool;

use super::bind::Params;
use crate::types::OracleDataType;
use crate::{oci::*, Result};
use std::mem::size_of;

/// A trait for types that can be used as SQL arguments
pub trait ToSql : Send + Sync {
    /**
    Binds itself to the SQL parameter placeholder

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
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize>;

    /**
    A callback that is called to update OUT (or INOUT) argumetns. For example, to set the length
    of the received data.
    */
    fn update_from_bind(&mut self, _pos: usize, _params: &Params) {}
}

impl ToSql for () {
    fn bind_to(&mut self, pos: usize, _params: &mut Params, _stmt: &OCIStmt, _err: &OCIError) -> Result<usize> {
        Ok(pos + 1)
    }
}


impl<T> ToSql for Descriptor<T> where T: DescriptorType, T::OCIType: OCIStruct {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind(pos, T::sql_type(), self.as_ptr() as _, size_of::<*mut T::OCIType>(), size_of::<*mut T::OCIType>(), stmt, err)?;
        Ok(pos + 1)
    }
}

impl<T> ToSql for &Descriptor<T> where T: DescriptorType, T::OCIType: OCIStruct {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind(pos, T::sql_type(), self.as_ptr() as _, size_of::<*mut T::OCIType>(), size_of::<*mut T::OCIType>(), stmt, err)?;
        Ok(pos + 1)
    }
}

impl<T> ToSql for &mut Descriptor<T> where T: DescriptorType, T::OCIType: OCIStruct {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        params.bind(pos, T::sql_type(), self.as_mut_ptr() as _, size_of::<*mut T::OCIType>(), size_of::<*mut T::OCIType>(), stmt, err)?;
        Ok(pos + 1)
    }
}

impl<T> ToSql for Option<T> where T: OracleDataType {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        if let Some(val) = self {
            val.bind_to(pos, params, stmt, err)
        } else {
            params.bind_null(pos, T::sql_null_type(), stmt, err)?;
            Ok(pos + 1)
        }
    }

    fn update_from_bind(&mut self, pos: usize, params: &Params) {
        if params.is_null(pos).unwrap_or(true) {
            self.take();
        } else if let Some(val) = self {
            val.update_from_bind(pos, params);
        }
    }
}

impl<T> ToSql for &Option<T> where T: OracleDataType {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        if let Some(val) = self {
            // Coerse val into ref mut to satisfy `bind_to`
            let val = val as *const T as *mut T;
            let val = unsafe { &mut *val };
            val.bind_to(pos, params, stmt, err)
        } else {
            Ok(pos + 1)
        }
    }
}

impl<T> ToSql for &mut Option<T> where T: OracleDataType {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        if let Some(val) = self {
            val.bind_to(pos, params, stmt, err)
        } else {
            Err(crate::Error::Interface("OUT (or INOUT) argument cannot be None".into()))
        }
    }

    fn update_from_bind(&mut self, pos: usize, params: &Params) {
        if params.is_null(pos).unwrap_or(true) {
            self.take();
        } else if let Some(val) = self {
            val.update_from_bind(pos, params);
        }
    }
}

impl<T> ToSql for (&str, T) where T: ToSql {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let idx = params.index_of(self.0)?;
        self.1.bind_to(idx, params, stmt, err)?;
        Ok(pos)
    }

    fn update_from_bind(&mut self, pos: usize, params: &Params) {
        let idx = params.index_of(self.0).unwrap_or(pos);
        self.1.update_from_bind(idx, params);
    }
}

impl<T1,T2> ToSql for ((&str, T1), (&str, T2)) where T1: ToSql, T2: ToSql {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let idx = params.index_of(self.0.0)?;
        self.0.1.bind_to(idx, params, stmt, err)?;
        let idx = params.index_of(self.1.0)?;
        self.1.1.bind_to(idx, params, stmt, err)?;
        Ok(pos)
    }

    fn update_from_bind(&mut self, pos: usize, params: &Params) {
        let idx = params.index_of(self.0.0).unwrap_or(pos);
        self.0.1.update_from_bind(idx, params);
        let idx = params.index_of(self.1.0).unwrap_or(pos);
        self.1.1.update_from_bind(idx, params);
    }
}

macro_rules! impl_tuple_args {
    ($head:ident $($tail:ident)+) => {
        impl<$head $(, $tail)*> ToSql for ($head $(, $tail)*) where $head: ToSql $(, $tail: ToSql)* {
            #[allow(non_snake_case)]
            fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                let (ref mut $head $(, ref mut $tail)*) = *self;
                let pos = $head.bind_to(pos, params, stmt, err)?;
                $(
                    let pos = $tail.bind_to(pos, params, stmt, err)?;
                )*
                Ok(pos)
            }
            #[allow(non_snake_case)]
            fn update_from_bind(&mut self, mut pos: usize, params: &Params) {
                let (ref mut $head $(, ref mut $tail)*) = *self;
                $head.update_from_bind(pos, params);
                $(
                    pos += 1;
                    $tail.update_from_bind(pos, params);
                )*
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
