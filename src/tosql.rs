use crate::*;
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
        )+
    };
}

impl_num_to_sql!{ i8, i16, i32, i64, isize => SQLT_INT }
impl_num_to_sql!{ u8, u16, u32, u64, usize => SQLT_UIN }
impl_num_to_sql!{ f32 => SQLT_BFLOAT }
impl_num_to_sql!{ f64 => SQLT_BDOUBLE }

impl ToSql for str {
    fn to_sql(&self) -> (u16, *const c_void, usize) {
        ( SQLT_CHR, self.as_ptr() as *const c_void, self.len() )
    }
}

impl ToSql for String {
    fn to_sql(&self) -> (u16, *const c_void, usize) {
        ( SQLT_CHR, self.as_ptr() as *const c_void, self.len() )
    }
}

impl ToSql for [u8] {
    fn to_sql(&self) -> (u16, *const c_void, usize) {
        ( SQLT_LBI, self.as_ptr() as *const c_void, self.len() )
    }
}

impl ToSql for Vec<u8> {
    fn to_sql(&self) -> (u16, *const c_void, usize) {
        ( SQLT_LBI, self.as_ptr() as *const c_void, self.len() )
    }
}

impl<T: ToSql + ?Sized> ToSql for &T {
    fn to_sql(&self) -> (u16, *const c_void, usize) {
        (*self).to_sql()
    }
}
