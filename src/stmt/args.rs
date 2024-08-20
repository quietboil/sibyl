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
use std::cell::UnsafeCell;
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
    fn update_from_bind(&mut self, pos: usize, _params: &Params) -> Result<usize> {
        Ok(pos + 1)
    }
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

    fn update_from_bind(&mut self, pos: usize, params: &Params) -> Result<usize> {
        if params.is_null(pos)? {
            self.take();
            Ok(pos + 1)
        } else if let Some(val) = self {
            val.update_from_bind(pos, params)
        } else {
            Ok(pos + 1)
        }
    }
}

impl<T> ToSql for &Option<T> where T: OracleDataType {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        if let Some(val) = self {
            // Coerse val into ref mut to satisfy `bind_to`
            let val = val as *const T as *mut T as *const UnsafeCell<T>;
            let val: &UnsafeCell<T> = unsafe { &*val };
            let val = unsafe { &mut *val.get() };
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

    fn update_from_bind(&mut self, pos: usize, params: &Params) -> Result<usize> {
        if params.is_null(pos)? {
            self.take();
            Ok(pos + 1)
        } else if let Some(val) = self {
            val.update_from_bind(pos, params)
        } else {
            Ok(pos + 1)
        }
    }
}

impl ToSql for Vec<&mut dyn ToSql> {
    fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        for item in self.iter_mut() {
            pos = item.bind_to(pos, params, stmt, err)?;
        }
        Ok(pos)
    }

    fn update_from_bind(&mut self, mut pos: usize, params: &Params) -> Result<usize> {
        for item in self.iter_mut() {
            pos = item.update_from_bind(pos, params)?;
        }
        Ok(pos)
    }
}

impl ToSql for &mut Vec<&mut dyn ToSql> {
    fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        for item in self.iter_mut() {
            pos = item.bind_to(pos, params, stmt, err)?;
        }
        Ok(pos)
    }

    fn update_from_bind(&mut self, mut pos: usize, params: &Params) -> Result<usize> {
        for item in self.iter_mut() {
            pos = item.update_from_bind(pos, params)?;
        }
        Ok(pos)
    }
}

impl<T> ToSql for (&str, T) where T: ToSql {
    fn bind_to(&mut self, _pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let idx = params.index_of(self.0)?;
        self.1.bind_to(idx, params, stmt, err)
    }

    fn update_from_bind(&mut self, _pos: usize, params: &Params) -> Result<usize> {
        let idx = params.index_of(self.0)?;
        self.1.update_from_bind(idx, params)
    }
}

impl<T1,T2> ToSql for ((&str, T1), (&str, T2)) where T1: ToSql, T2: ToSql {
    fn bind_to(&mut self, _pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let idx = params.index_of(self.0.0)?;
        self.0.1.bind_to(idx, params, stmt, err)?;
        let idx = params.index_of(self.1.0)?;
        self.1.1.bind_to(idx, params, stmt, err)
    }

    fn update_from_bind(&mut self, _pos: usize, params: &Params) -> Result<usize> {
        let idx = params.index_of(self.0.0)?;
        self.0.1.update_from_bind(idx, params)?;
        let idx = params.index_of(self.1.0)?;
        self.1.1.update_from_bind(idx, params)
    }
}

macro_rules! impl_tuple_args {
    ($($item:ident)+) => {
        impl<$($item),+> ToSql for ($($item),+) where $($item: ToSql),+ {
            #[allow(non_snake_case)]
            fn bind_to(&mut self, mut pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
                let ($(ref mut $item),+) = *self;
                $(
                    pos = $item.bind_to(pos, params, stmt, err)?;
                )+
                Ok(pos)
            }
            #[allow(non_snake_case)]
            fn update_from_bind(&mut self, mut pos: usize, params: &Params) -> Result<usize> {
                let ($(ref mut $item),+) = *self;
                $(
                    pos = $item.update_from_bind(pos, params)?;
                )+
                Ok(pos)
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
