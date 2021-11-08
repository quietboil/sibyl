//! SQL statement arguments

use crate::oci::*;
use libc::c_void;

/// A trait for types that can be used as SQL IN arguments
pub trait ToSql {
    /// Returns a tuple with (SQLT type, pointer to the data buffer, length of the data in the buffer)
    fn to_sql(&self) -> (u16, *const c_void, usize);
}

macro_rules! impl_num_to_sql {
    ($($t:ty),+ => $sqlt:ident) => {
        $(
            impl ToSql for $t {
                fn to_sql(&self) -> (u16, *const c_void, usize) {
                    ( $sqlt, self as *const $t as *const c_void, std::mem::size_of::<$t>() )
                }
            }
            impl ToSql for &$t {
                fn to_sql(&self) -> (u16, *const c_void, usize) {
                    ( $sqlt, (*self) as *const $t as *const c_void, std::mem::size_of::<$t>() )
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
    fn to_sql(&self) -> (u16, *const c_void, usize) {
        ( SQLT_CHR, (*self).as_ptr() as *const c_void, (*self).len() )
    }
}

impl ToSql for &[u8] {
    fn to_sql(&self) -> (u16, *const c_void, usize) {
        ( SQLT_LBI, (*self).as_ptr() as *const c_void, (*self).len() )
    }
}

/// A trait for types that can be used as SQL OUT arguments
pub trait ToSqlOut {
    /// Returns output buffer characteristics as a tuple with (SQLT type, buffer pointer, buffer size, IN data length)
    fn to_sql_output(&mut self) -> (u16, *mut c_void, usize, usize);

    /// Called to set the received data length (always less than the initial capacity)
    fn set_len(&mut self, _new_len: usize) { }
}

macro_rules! impl_num_to_sql_output {
    ($($t:ty),+ => $sqlt:ident) => {
        $(
            impl ToSqlOut for $t {
                fn to_sql_output(&mut self) -> (u16, *mut c_void, usize, usize) {
                    ($sqlt, self as *mut $t as *mut c_void, std::mem::size_of::<$t>(), std::mem::size_of::<$t>())
                }
            }
        )+
    };
}

impl_num_to_sql_output!{ i8, i16, i32, i64, isize => SQLT_INT }
impl_num_to_sql_output!{ u8, u16, u32, u64, usize => SQLT_UIN }
impl_num_to_sql_output!{ f32 => SQLT_BFLOAT }
impl_num_to_sql_output!{ f64 => SQLT_BDOUBLE }

impl ToSqlOut for String {
    fn to_sql_output(&mut self) -> (u16, *mut c_void, usize, usize) {
        (SQLT_CHR, unsafe { (*self).as_mut_vec().as_mut_ptr() } as *mut c_void, (*self).capacity(), (*self).len())
    }
    fn set_len(&mut self, new_len: usize) {
        unsafe { (*self).as_mut_vec().set_len(new_len) }
    }
}

impl ToSqlOut for Vec<u8> {
    fn to_sql_output(&mut self) -> (u16, *mut c_void, usize, usize) {
        (SQLT_LBI, (*self).as_mut_ptr() as *mut c_void, (*self).capacity(), (*self).len())
    }
    fn set_len(&mut self, new_len: usize) {
        unsafe { (*self).set_len(new_len) }
    }
}

/// A trait for types that can be used as named or positional SQL IN arguments
pub trait SqlInArg {
    /// Returns the parameter name or None for positional arguments.
    fn name(&self) -> Option<&str>;
    /// Returns `ToSql` trait implementation for this argument.
    fn as_to_sql(&self) -> &dyn ToSql;
}

impl<T: ToSql> SqlInArg for T {
    fn name(&self) -> Option<&str>      { None }
    fn as_to_sql(&self) -> &dyn ToSql   { self }
}

impl<T: ToSql> SqlInArg for (&str, T) {
    fn name(&self) -> Option<&str>      { Some( self.0 ) }
    fn as_to_sql(&self) -> &dyn ToSql   { &self.1        }
}

/// A trait for types that can be used as named or positional SQL OUT arguments
pub trait SqlOutArg {
    /// Returns the parameter name or None for positional arguments.
    fn name(&self) -> Option<&str>;
    /// Returns `ToSqlOut` trait implementation for this argument.
    fn as_to_sql_out(&mut self) -> &mut dyn ToSqlOut;
}

impl<T: ToSqlOut> SqlOutArg for T {
    fn name(&self) -> Option<&str>                      { None }
    fn as_to_sql_out(&mut self) -> &mut dyn ToSqlOut    { self }
}

impl<T: ToSqlOut> SqlOutArg for (&str, &mut T) {
    fn name(&self) -> Option<&str>                      { Some( self.0 ) }
    fn as_to_sql_out(&mut self) -> &mut dyn ToSqlOut    { self.1 }
}
