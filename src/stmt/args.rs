//! SQL statement arguments

use crate::{oci::*, ptr::{ScopedPtr, ScopedMutPtr}};
use libc::c_void;

/// A trait for types that can be used as SQL IN arguments
pub trait ToSql : Send + Sync {
    /// Returns SQLT type
    fn sql_type(&self) -> u16;
    /// Returns a pointer to the data
    fn sql_data_ptr(&self) -> ScopedPtr<c_void>;
    /// Returns IN data length
    fn sql_data_len(&self) -> usize;
}

/// A trait for types that can be used as SQL OUT or INOUT arguments
pub trait ToSqlOut : Send + Sync {
    /// Returns SQLT type
    fn sql_type(&self) -> u16;
    /// Returns a pointer to the mutable data buffer
    fn sql_mut_data_ptr(&mut self) -> ScopedMutPtr<c_void>;
    /// Returns IN data length
    fn sql_data_len(&self) -> usize;
    /// Returns OUT arguments (buffer) capacity
    fn sql_capacity(&self) -> usize { self.sql_data_len() }
    /// Sets the length of the received data
    /// This is only applicable to types like `Vec` and `String`
    fn sql_set_len(&mut self, _new_len: usize) {}
}

macro_rules! impl_num_to_sql {
    ($($t:ty),+ => $sqlt:ident) => {
        $(
            impl ToSql for $t {
                fn sql_type(&self) -> u16 { $sqlt }
                fn sql_data_ptr(&self) -> ScopedPtr<c_void> { ScopedPtr::new(self as *const $t as _) }
                fn sql_data_len(&self) -> usize { std::mem::size_of::<$t>() }
            }
            impl ToSql for &$t {
                fn sql_type(&self) -> u16 { $sqlt }
                fn sql_data_ptr(&self) -> ScopedPtr<c_void> { ScopedPtr::new((*self) as *const $t as _) }
                fn sql_data_len(&self) -> usize { std::mem::size_of::<$t>() }
            }
            impl ToSqlOut for $t {
                fn sql_type(&self) -> u16 { $sqlt }
                fn sql_mut_data_ptr(&mut self) -> ScopedMutPtr<c_void> { ScopedMutPtr::new(self as *mut $t as _) }
                fn sql_data_len(&self) -> usize { std::mem::size_of::<$t>() }
            }
        )+
    };
}

impl_num_to_sql!{ i8, i16, i32, i64, isize => SQLT_INT }
impl_num_to_sql!{ u8, u16, u32, u64, usize => SQLT_UIN }
impl_num_to_sql!{ f32 => SQLT_BFLOAT }
impl_num_to_sql!{ f64 => SQLT_BDOUBLE }

impl ToSql for &str {
    fn sql_type(&self) -> u16 { SQLT_CHR }
    fn sql_data_ptr(&self) -> ScopedPtr<c_void> { ScopedPtr::new((*self).as_ptr() as _) }
    fn sql_data_len(&self) -> usize { (*self).len() }
}

impl ToSql for &[u8] {
    fn sql_type(&self) -> u16 { SQLT_LBI }
    fn sql_data_ptr(&self) -> ScopedPtr<c_void> { ScopedPtr::new((*self).as_ptr() as _) }
    fn sql_data_len(&self) -> usize { (*self).len() }
}

impl ToSqlOut for String {
    fn sql_type(&self) -> u16 { SQLT_CHR }
    fn sql_mut_data_ptr(&mut self) -> ScopedMutPtr<c_void> { ScopedMutPtr::new(unsafe { self.as_mut_vec().as_mut_ptr() } as _) }
    fn sql_data_len(&self) -> usize { self.len() }
    fn sql_capacity(&self) -> usize { self.capacity() }
    fn sql_set_len(&mut self, new_len: usize) { unsafe { self.as_mut_vec().set_len(new_len) } }
}

impl ToSqlOut for Vec<u8> {
    fn sql_type(&self) -> u16 { SQLT_LBI }
    fn sql_mut_data_ptr(&mut self) -> ScopedMutPtr<c_void> { ScopedMutPtr::new((*self).as_mut_ptr() as _) }
    fn sql_data_len(&self) -> usize { self.len() }
    fn sql_capacity(&self) -> usize { self.capacity() }
    fn sql_set_len(&mut self, new_len: usize) { unsafe { self.set_len(new_len) } }
}

/// A trait for types that can be used as named or positional SQL IN arguments
pub trait SqlInArg : Send + Sync {
    /// Returns the parameter name or None for positional arguments.
    fn name(&self) -> Option<&str>;
    /// Returns `ToSql` trait implementation for this argument.
    fn to_sql(&self) -> &dyn ToSql;
}

/// A trait for types that can be used as named or positional SQL OUT arguments
pub trait SqlOutArg : Send + Sync {
    /// Returns the parameter name or None for positional arguments.
    fn name(&self) -> Option<&str>;
    /// Returns `ToSqlOut` trait implementation for this argument.
    fn to_sql_out(&mut self) -> &mut dyn ToSqlOut;
}

impl<T: ToSql> SqlInArg for T {
    fn name(&self) -> Option<&str>  { None }
    fn to_sql(&self) -> &dyn ToSql  { self }
}

impl<T: ToSql> SqlInArg for (&str, T) {
    fn name(&self) -> Option<&str>  { Some(self.0) }
    fn to_sql(&self) -> &dyn ToSql  { &self.1      }
}

impl<T: ToSqlOut> SqlOutArg for T {
    fn name(&self) -> Option<&str>                  { None }
    fn to_sql_out(&mut self) -> &mut dyn ToSqlOut   { self }
}

impl<T: ToSqlOut> SqlOutArg for (&str, &mut T) {
    fn name(&self) -> Option<&str>                  { Some(self.0) }
    fn to_sql_out(&mut self) -> &mut dyn ToSqlOut   { self.1       }
}
