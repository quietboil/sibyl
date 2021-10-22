use crate::oci::*;
use libc::c_void;

/// A trait for types that can be used as SQL OUT arguments
pub trait ToSqlOut {
    /// Returns output buffer characteristics as a tuple of (SQLT type, buffer pointer, buffer size, IN data length)
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
