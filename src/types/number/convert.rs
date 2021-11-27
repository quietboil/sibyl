/// Convertion between Oracle Numbers and Rust numerics

use std::mem;
use crate::{ Result, err::Error, oci::* };
use super::{ real_into_number, to_real };

fn u128_into_number(mut val: u128) -> OCINumber {
    let mut num = mem::MaybeUninit::<OCINumber>::uninit();
    let ptr = num.as_mut_ptr();
    if val == 0 {
        unsafe {
            (*ptr).bytes[0] = 1;
            (*ptr).bytes[1]= 128;
        }
    } else {
        let mut digits = [0u8;20];
        let mut idx = digits.len();
        let mut exp = 192u8;
        while val != 0 {
            let digit = (val % 100) as u8;
            if digit > 0 || idx < digits.len() {
                idx -= 1;
                digits[idx] = digit + 1;
            }
            val /= 100;
            exp += 1;
        }
        let len = digits.len() - idx;
        unsafe {
            (*ptr).bytes[0] = len as u8 + 1;
            (*ptr).bytes[1] = exp;
            (*ptr).bytes[2..2 + len].copy_from_slice(&digits[idx..]);
        }
    }
    unsafe { num.assume_init() }
}

fn i128_into_number(mut val: i128) -> OCINumber {
    if val >= 0 {
        u128_into_number(val as u128)
    } else {
        let mut digits = [0u8;21];
        let mut idx = digits.len() - 1;
        let mut exp = 63u8;
        val = -val;
        digits[idx] = 102;
        while val != 0 {
            let digit = (val % 100) as u8;
            if digit > 0 || idx < digits.len() - 1 {
                idx -= 1;
                digits[idx] = 101 - digit;
            }
            val /= 100;
            exp -= 1;
        }
        let len = if idx > 0 { digits.len() - idx } else { digits.len() - 1 };
        let mut num = mem::MaybeUninit::<OCINumber>::uninit();
        let ptr = num.as_mut_ptr();
        unsafe {
            (*ptr).bytes[0] = len as u8 + 1;
            (*ptr).bytes[1] = exp;
            (*ptr).bytes[2..2 + len].copy_from_slice(&digits[idx..idx + len]);
            num.assume_init()
        }
    }
}

fn u128_from_number(num: &OCINumber) -> Result<u128> {
    let len = num.bytes[0] as usize;
    let exp = num.bytes[1];
    if len == 0 || len >= num.bytes.len() {
        Err( Error::new("uninitialized number") )
    } else if len == 1 || 62 < exp && exp < 193 {
        Ok( 0 )
    } else if exp <= 62 {
        Err( Error::new("cannot convert negative number into an unsigned integer") )
    } else if exp > 212 {
        Err( Error::new("overflow") )
    } else {
        let mut exp = exp - 193;
        let mut val = (num.bytes[2] - 1) as u128;
        let mut idx = 3;
        while idx <= len && exp > 0 {
            let digit = (num.bytes[idx] - 1) as u128;
            val = val * 100 + digit;
            idx += 1;
            exp -= 1;
        }
        if exp > 0 {
            val *= 100u128.pow(exp as u32);
        } else if idx <= len {
            let digit = num.bytes[idx];
            if digit >= 50 {
                val += 1;
            }
        }
        Ok( val )
    }
}

fn i128_from_number(num: &OCINumber) -> Result<i128> {
    let len = num.bytes[0] as usize;
    let exp = num.bytes[1];
    if exp >= 193 {
        let val = u128_from_number(num)?;
        Ok( val as i128 )
    } else if len == 0 || len >= num.bytes.len() {
        Err( Error::new("uninitialized number") )
    } else if len == 1 || 62 < exp && exp < 193 {
        Ok( 0 )
    } else if exp < 43 {
        Err( Error::new("overflow") )
    } else {
        let mut exp = 62 - exp;
        let mut val = (101 - num.bytes[2]) as i128;
        let mut idx = 3;
        while idx <= len && exp > 0 && num.bytes[idx] <= 101 {
            let digit = (101 - num.bytes[idx]) as i128;
            val = val * 100 + digit;
            idx += 1;
            exp -= 1;
        }
        if exp > 0 {
            val *= 100i128.pow(exp as u32);
        } else if idx <= len && num.bytes[idx] <= 101 {
            let digit = num.bytes[idx];
            if digit <= 52 {
                val += 1;
            }
        }
        Ok( -val )
    }
}

/// Trait for types that can be converted into `OCINumber`
pub trait IntoNumber : Sized + Copy {
    fn into_number(self, err: *mut OCIError) -> Result<OCINumber>;
}

impl IntoNumber for i128 {
    fn into_number(self, _err: *mut OCIError) -> Result<OCINumber> {
        Ok(i128_into_number(self))
    }
}

impl IntoNumber for u128 {
    fn into_number(self, _err: *mut OCIError) -> Result<OCINumber> {
        Ok(u128_into_number(self))
    }
}

macro_rules! impl_int_into_num {
    ($($t:ty),+ => $dt:ty) => {
        $(
            impl IntoNumber for $t {
                fn into_number(self, err: *mut OCIError) -> Result<OCINumber> {
                    let val = self as $dt;
                    val.into_number(err)
                }
            }
        )+
    };
}

impl_int_into_num!(i8, i16, i32, i64, isize => i128);
impl_int_into_num!(u8, u16, u32, u64, usize => u128);

/// Trait for types that can be created from `OCINumber`
pub trait FromNumber : Sized + Copy {
    fn from_number(num: &OCINumber, err: *mut OCIError) -> Result<Self>;
}

impl FromNumber for i128 {
    fn from_number(num: &OCINumber, _err: *mut OCIError) -> Result<Self> {
        i128_from_number(&num)
    }
}

impl FromNumber for u128 {
    fn from_number(num: &OCINumber, _err: *mut OCIError) -> Result<Self> {
        u128_from_number(&num)
    }
}

macro_rules! impl_int_from_num {
    ($($t:ty),+ => $dt:ty, $f:ident) => {
        $(
            impl FromNumber for $t {
                fn from_number(num: &OCINumber, _err: *mut OCIError) -> Result<Self> {
                    let val = $f(num)?;
                    if <$t>::min_value() as $dt <= val && val <= <$t>::max_value() as $dt {
                        Ok( val as $t)
                    } else {
                        Err( Error::new("overflow") )
                    }
                }
            }
        )+
    };
}

impl_int_from_num!(i8, i16, i32, i64, isize => i128, i128_from_number);
impl_int_from_num!(u8, u16, u32, u64, usize => u128, u128_from_number);

impl IntoNumber for f64 {
    fn into_number(self, err: *mut OCIError) -> Result<OCINumber> {
        real_into_number(self, err)
    }
}

impl IntoNumber for f32 {
    fn into_number(self, err: *mut OCIError) -> Result<OCINumber> {
        real_into_number(self, err)
    }
}

impl FromNumber for f64 {
    fn from_number(num: &OCINumber, err: *mut OCIError) -> Result<Self> {
        to_real::<f64>(num, err)
    }
}

impl FromNumber for f32 {
    fn from_number(num: &OCINumber, err: *mut OCIError) -> Result<Self> {
        to_real::<f32>(num, err)
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn num_from_to_int() -> Result<()> {
        let env = env()?;

        let num = Number::from_int(0, &env)?;
        let val = num.to_int::<i32>()?;
        assert_eq!(0, val);

        let num = Number::from_int(42, &env)?;
        let val = num.to_int::<i32>()?;
        assert_eq!(42, val);

        let num = Number::from_int(250_000_000_000i64, &env)?;
        let val = num.to_int::<i64>()?;
        assert_eq!(250_000_000_000i64, val);

        let num = Number::from_int(250_000_190_000i64, &env)?;
        let val = num.to_int::<i64>()?;
        assert_eq!(250_000_190_000i64, val);

        let num = Number::from_int(-150_000_000_000i64, &env)?;
        let val = num.to_int::<i64>()?;
        assert_eq!(-150_000_000_000i64, val);

        let num = Number::from_int(-31415926535897932384626433832795028842i128, &env)?;
        let txt = num.to_string("TM")?;
        assert_eq!("-31415926535897932384626433832795028842", txt);
        let val = num.to_int::<i128>()?;
        assert_eq!(-31415926535897932384626433832795028842i128, val);

        let num = Number::from_int(std::u128::MAX, &env)?;
        let txt = num.to_string("TM")?;
        assert_eq!("340282366920938463463374607431768211455", txt);
        let val = num.to_int::<u128>()?;
        assert_eq!(std::u128::MAX, val);

        let num = Number::pi(&env)?;
        let val = num.to_int::<i32>()?;
        assert_eq!(3, val);

        let num = Number::pi(&env)?;
        let num = num.pow10(2)?;
        let val = num.to_int::<i32>()?;
        assert_eq!(314, val);
        let neg = num.neg()?;
        let val = neg.to_int::<i32>()?;
        assert_eq!(-314, val);
        let num = num.pow10(5)?;
        let val = num.to_int::<i32>()?;
        assert_eq!(31415927, val);
        let neg = num.neg()?;
        let val = neg.to_int::<i32>()?;
        assert_eq!(-31415927, val);

        Ok(())
    }
}
