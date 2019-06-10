//! OCI functions to manipulate Oracle Number:
//! NUMBER, NUMERIC, INT, SHORTINT, REAL, DOUBLE PRECISION, FLOAT and DECIMAL.

use crate::*;
use super::*;
use libc::c_void;
use std::{ mem, ptr, cmp::Ordering };

/// C mapping of the Oracle NUMBER
#[repr(C)] pub struct OCINumber { _private: [u8; 22] }

pub(crate) fn to_string(fmt: &str, num: *const OCINumber, err: *mut OCIError) -> Result<String> {
    let mut txt : [u8;64];
    unsafe {
        txt = mem::uninitialized();
    }
    let mut txt_len = txt.len() as u32;
    catch!{err =>
        OCINumberToText(
            err, num,
            fmt.as_ptr(), fmt.len() as u32,
            ptr::null(), 0,
            &mut txt_len, txt.as_mut_ptr()
        )
    }
    let txt = &txt[0..txt_len as usize];
    Ok( String::from_utf8_lossy(txt).to_string() )
}

pub(crate) fn from_number<'a>(from_num: &OCINumber, env: &'a dyn UsrEnv) -> Result<Number<'a>> {
    let mut num : OCINumber;
    catch!{env.err_ptr() =>
        num = mem::uninitialized();
        OCINumberAssign(env.err_ptr(), from_num as *const OCINumber, &mut num)
    }
    Ok( Number { env, num } )
}

pub(crate) fn new() -> OCINumber {
    let num : OCINumber;
    unsafe {
        num = mem::uninitialized();
    }
    num
}

pub(crate) fn new_number<'a>(num: OCINumber, env: &'a dyn UsrEnv) -> Number<'a> {
    Number { env, num }
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-61FB0D0F-6EA7-45DD-AF40-310D86FB8BAE
    fn OCINumberAbs(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-F3DC6DF6-9110-4BAC-AB97-DC604CA04BCD
    fn OCINumberAdd(
        err:      *mut OCIError,
        number1:  *const OCINumber,
        number2:  *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-E7A8B43C-F8B0-4009-A770-94CD7E13EE75
    fn OCINumberArcCos(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-3956D4AC-62E5-41FD-BA48-2DA89E207259
    fn OCINumberArcSin(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-43E9438C-AA74-4392-889D-171F411EBBE2
    fn OCINumberArcTan(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-62C977EF-DB7E-457F-847A-BF0D46E36CD5
    fn OCINumberArcTan2(
        err:      *mut OCIError,
        number1:  *const OCINumber,
        number2:  *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-0C78F351-550E-48F0-8D4C-A9AD8A28DA66
    fn OCINumberAssign(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-48974097-47D4-4757-A627-4E09406AAFD5
    fn OCINumberCeil(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-554A4409-946B-47E9-B239-4140B8F3D1F9
    fn OCINumberCmp(
        err:      *mut OCIError,
        number1:  *const OCINumber,
        number2:  *const OCINumber,
        result:   *mut i32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-150F3245-ECFC-4352-AA73-AAF29BC6A74C
    fn OCINumberCos(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-370FD18E-47D3-4110-817C-658A2F059361
    fn OCINumberDec(
        err:      *mut OCIError,
        number:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-36A6C0EA-85A4-44EE-8489-FB7DB4257513
    fn OCINumberDiv(
        err:      *mut OCIError,
        number1:  *const OCINumber,
        number2:  *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-B56F44FC-158A-420B-830E-FB82894A62C8
    fn OCINumberExp(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-CF35CBDF-DC88-4E86-B586-0EEFD35C0458
    fn OCINumberFloor(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-E8940E06-F4EF-4172-AEE5-AF8E4F6B3AEE
    // fn OCINumberFromInt(
    //     err:      *mut OCIError,
    //     inum:     *const c_void,
    //     inum_len: u32,
    //     sign_typ: u32,
    //     number:   *mut OCINumber
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-EC8E2C9E-BCD2-4D1E-A052-3E657B552461
    fn OCINumberFromReal(
        err:      *mut OCIError,
        rnum:     *const c_void,
        rnum_len: u32,              // sizeof(float | double | long double)
        number:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-F2E458B5-BECC-482E-9223-B92BC696CA17
    fn OCINumberFromText(
        err:      *mut OCIError,
        txt:      *const u8,
        txt_len:  u32,
        fmt:      *const u8,
        fmt_len:  u32,
        nls_par:  *const u8,
        nls_len:  u32,
        number:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-08CCC2C4-5AB3-45EB-9E0D-28186A2AA234
    fn OCINumberHypCos(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-E7391F43-2DFB-4146-9AB7-816D009F31E5
    fn OCINumberHypSin(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-4254930A-DCDC-4590-8710-AC46EC4F3473
    fn OCINumberHypTan(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-A3B07A3A-7E18-421E-9085-BE4B3E742C83
    fn OCINumberInc(
        err:      *mut OCIError,
        number:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-D5CF4199-D6D2-4D31-A914-FB74F5BC5412
    fn OCINumberIntPower(
        err:      *mut OCIError,
        base:     *const OCINumber,
        exp:      i32,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-F1254BAD-7236-4728-A9DA-B8701D8BAA14
    fn OCINumberIsInt(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut i32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-40F344FC-3ED0-4893-AFB1-0853D02D79C9
    fn OCINumberIsZero(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut i32          // set to TRUE if equal to zero else FALSE
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-C1E572F2-F68D-4AF4-831A-2095BFEDDBC3
    fn OCINumberLn(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-561769B0-B559-44AA-8012-985EA7ADFB47
    fn OCINumberLog(
        err:      *mut OCIError,
        base:     *const OCINumber,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-B5DAB7F2-6AC6-4693-8F04-8C13F9538CE9
    fn OCINumberMod(
        err:      *mut OCIError,
        number1:  *const OCINumber,
        number2:  *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-8AAAC840-3776-4283-9DC5-5764CAC2359A
    fn OCINumberMul(
        err:      *mut OCIError,
        number1:  *const OCINumber,
        number2:  *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-8810FFCB-51E7-4890-B551-61BE85624764
    fn OCINumberNeg(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-E755AD46-4285-4DAF-B2A5-886333A2395D
    fn OCINumberPower(
        err:      *mut OCIError,
        base:     *const OCINumber,
        exp:      *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-BE4B0E6D-75B6-4256-A355-9DFAFEC477C9
    fn OCINumberPrec(
        err:      *mut OCIError,
        number:   *const OCINumber,
        num_dig:  i32,              // number of decimal digits desired in the result
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-F3B89623-73E3-428F-A677-5526AC5F4622
    fn OCINumberRound(
        err:      *mut OCIError,
        number:   *const OCINumber,
        num_dig:  i32,              // number of decimal digits to the right of the decimal point to round to. Negative values are allowed.
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-FA067559-D0F7-426D-940A-1D24F4C60C70
    fn OCINumberSetPi(
        err:      *mut OCIError,
        number:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-8152D558-61D9-49F4-9113-DA1455BB5C72
    fn OCINumberSetZero(
        err:      *mut OCIError,
        number:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-EA7D0DA0-A154-4A87-8215-E5B5A7D091E3
    fn OCINumberShift(
        err:      *mut OCIError,
        number:   *const OCINumber,
        num_dec:  i32,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-A535F6F1-0689-4FE1-9C07-C8D341582622
    fn OCINumberSign(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut i32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-65293408-5AF2-4A0C-9C51-82C1C929EE54
    fn OCINumberSin(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-9D68D274-B18C-43F4-AB37-BB99C9062B3E
    fn OCINumberSqrt(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-192725C3-8F5C-4D0A-848E-4EE9690F4A4E
    fn OCINumberSub(
        err:      *mut OCIError,
        number1:  *const OCINumber,
        number2:  *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-1EB45341-6026-47AD-A2EF-D92A20A46ECF
    fn OCINumberTan(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-067F138E-E689-4922-9ED7-4A7B0E46447E
    // fn OCINumberToInt(
    //     err:      *mut OCIError,
    //     number:   *const OCINumber,
    //     res_len:  u32,
    //     sign_typ: u32,
    //     result:   *mut c_void
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-76C4BC1E-EC64-4CF6-82A4-94D5DC242649
    fn OCINumberToReal(
        err:      *mut OCIError,
        number:   *const OCINumber,
        res_len:  u32,              // sizeof( float | double | long double)
        result:   *mut c_void
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-A850D4E3-2B7B-4DFE-A3E9-618515DACA9E
    // fn OCINumberToRealArray(
    //     err:      *mut OCIError,
    //     numbers:  &*const OCINumber,
    //     elems:    u32,
    //     res_len:  u32,              // sizeof( float | double | long double)
    //     result:   *mut c_void
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-129A5433-6927-43B7-A10F-5FE6AA354232
    fn OCINumberToText(
        err:      *mut OCIError,
        number:   *const OCINumber,
        fmt:      *const u8,
        fmt_len:  u32,
        nls_par:  *const u8,
        nls_len:  u32,
        buf_size: *mut u32,
        buf:      *mut u8
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-FD8D2A9A-222B-4A0E-B4E3-99588FF19BCA
    fn OCINumberTrunc(
        err:      *mut OCIError,
        number:   *const OCINumber,
        num_dig:  i32,
        result:   *mut OCINumber
    ) -> i32;
}

fn u128_to_number(mut val: u128) -> OCINumber {
    let mut num: OCINumber;
    unsafe {
        num = mem::uninitialized();
    }
    if val == 0 {
        num._private[0] = 1;
        num._private[1]= 128;
    } else {
        let mut digits: [u8;20];
        unsafe {
            digits = mem::uninitialized();
        }
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
        num._private[0] = len as u8 + 1;
        num._private[1] = exp;
        num._private[2..2 + len].copy_from_slice(&digits[idx..]);
    }
    num
}

fn i128_to_number(mut val: i128) -> OCINumber {
    if val >= 0 {
        (val as u128).into_number()
    } else {
        let mut num: OCINumber;
        let mut digits: [u8;21];
        unsafe {
            num = mem::uninitialized();
            digits = mem::uninitialized();
        }
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
        num._private[0] = len as u8 + 1;
        num._private[1] = exp;
        num._private[2..2 + len].copy_from_slice(&digits[idx..idx + len]);
        num
    }
}

fn u128_from_number(num: &OCINumber) -> Result<u128> {
    let len = num._private[0] as usize;
    let exp = num._private[1];
    if len == 0 || len >= num._private.len() {
        Err( Error::new("uninitialized number") )
    } else if len == 1 || 62 < exp && exp < 193 {
        Ok( 0 )
    } else if exp <= 62 {
        Err( Error::new("cannot convert negative number into an unsigned integer") )
    } else if exp > 212 {
        Err( Error::new("overflow") )
    } else {
        let mut exp = exp - 193;
        let mut val = (num._private[2] - 1) as u128;
        let mut idx = 3;
        while idx <= len && exp > 0 {
            let digit = (num._private[idx] - 1) as u128;
            val = val * 100 + digit;
            idx += 1;
            exp -= 1;
        }
        if exp > 0 {
            val *= 100u128.pow(exp as u32);
        } else if idx <= len {
            let digit = num._private[idx];
            if digit >= 50 {
                val += 1;
            }
        }
        Ok( val )
    }
}

fn i128_from_number(num: &OCINumber) -> Result<i128> {
    let len = num._private[0] as usize;
    let exp = num._private[1];
    if exp >= 193 {
        let val = u128_from_number(num)?;
        Ok( val as i128 )
    } else if len == 0 || len >= num._private.len() {
        Err( Error::new("uninitialized number") )
    } else if len == 1 || 62 < exp && exp < 193 {
        Ok( 0 )
    } else if exp < 43 {
        Err( Error::new("overflow") )
    } else {
        let mut exp = 62 - exp;
        let mut val = (101 - num._private[2]) as i128;
        let mut idx = 3;
        while idx <= len && exp > 0 && num._private[idx] <= 101 {
            let digit = (101 - num._private[idx]) as i128;
            val = val * 100 + digit;
            idx += 1;
            exp -= 1;
        }
        if exp > 0 {
            val *= 100i128.pow(exp as u32);
        } else if idx <= len && num._private[idx] <= 101 {
            let digit = num._private[idx];
            if digit <= 52 {
                val += 1;
            }
        }
        Ok( -val )
    }
}

pub trait Integer : Sized {
    fn into_number(self) -> OCINumber;
    fn from_number(num: &OCINumber) -> Result<Self>;
}

impl Integer for u128 {
    fn into_number(self) -> OCINumber {
        u128_to_number(self)
    }

    fn from_number(num: &OCINumber) -> Result<u128> {
        u128_from_number(&num)
    }
}

impl Integer for i128 {
    fn into_number(self) -> OCINumber {
        i128_to_number(self)
    }

    fn from_number(num: &OCINumber) -> Result<i128> {
        i128_from_number(&num)
    }
}

macro_rules! impl_int {
    ($($t:ty),+) => {
        $(
            impl Integer for $t {
                fn into_number(self) -> OCINumber {
                    let val = self as i128;
                    val.into_number()
                }
                fn from_number(num: &OCINumber) -> Result<$t> {
                    let val = i128_from_number(num)?;
                    if <$t>::min_value() as i128 <= val && val <= <$t>::max_value() as i128 {
                        Ok( val as $t)
                    } else {
                        Err( Error::new("overflow") )
                    }
                }
            }
        )+
    };
}

impl_int! { i8, i16, i32, i64, isize }

macro_rules! impl_uint {
    ($($t:ty),+) => {
        $(
            impl Integer for $t {
                fn into_number(self) -> OCINumber {
                    let val = self as u128;
                    val.into_number()
                }
                fn from_number(num: &OCINumber) -> Result<$t> {
                    let val = u128_from_number(num)?;
                    if val <= <$t>::max_value() as u128 {
                        Ok( val as $t)
                    } else {
                        Err( Error::new("overflow") )
                    }
                }
            }
        )+
    };
}

impl_uint! { u8, u16, u32, u64, usize }

pub trait Real {}
impl Real for f32 {}
impl Real for f64 {}

pub(crate) fn to_real<T:Real>(num: &OCINumber, err: *mut OCIError) -> Result<T> {
    let mut res: T;
    catch!{err =>
        res = mem::uninitialized();
        OCINumberToReal(err, num as *const OCINumber, mem::size_of::<T>() as u32, &mut res as *mut T as *mut c_void)
    }
    Ok(res)
}

macro_rules! impl_query {
    ($this:tt => $f:ident) => {
        let mut res : i32 = 0;
        catch!{$this.env.err_ptr() =>
            $f($this.env.err_ptr(), $this.as_ptr(), &mut res)
        }
        Ok( res != 0 )
    };
}

macro_rules! impl_fn {
    ($this:tt => $f:ident) => {
        let env = $this.env;
        let mut num : OCINumber;
        catch!{env.err_ptr() =>
            num = mem::uninitialized();
            $f(env.err_ptr(), $this.as_ptr(), &mut num)
        }
        Ok( Number { env, num } )
    };
}

macro_rules! impl_op {
    ($this:tt, $arg:ident => $f:ident) => {
        let env = $this.env;
        let mut res : OCINumber;
        catch!{env.err_ptr() =>
            res = mem::uninitialized();
            $f(env.err_ptr(), $this.as_ptr(), $arg.as_ptr(), &mut res)
        }
        Ok( Number { env, num: res } )
    };
}

macro_rules! impl_opi {
    ($this:tt, $arg:ident => $f:ident) => {
        let env = $this.env;
        let mut res : OCINumber;
        catch!{env.err_ptr() =>
            res = mem::uninitialized();
            $f(env.err_ptr(), $this.as_ptr(), $arg, &mut res)
        }
        Ok( Number { env, num: res } )
    };
}

/// Represents OTS types NUMBER, NUMERIC, INT, SHORTINT, REAL, DOUBLE PRECISION, FLOAT and DECIMAL.
pub struct Number<'e> {
    env: &'e dyn UsrEnv,
    num: OCINumber,
}

impl<'e> Number<'e> {
    /// Returns new uninitialized number.
    pub fn new(env: &'e dyn UsrEnv) -> Self {
        Self { env, num: new() }
    }

    /// Creates a new Number that is equal to zero.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::zero(&env);
    /// let is_zero = num.is_zero()?;
    ///
    /// assert!(is_zero);
    ///
    /// let val: i32 = num.to_int::<i32>()?;
    ///
    /// assert_eq!(0, val);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn zero(env: &'e dyn UsrEnv) -> Self {
        let mut num : OCINumber;
        unsafe {
            num = mem::uninitialized();
            OCINumberSetZero(env.err_ptr(), &mut num);
        }
        Self { env, num }
    }

    /// Creates a new Number that is equal to Pi.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::pi(&env);
    /// let txt = num.to_string("TM")?;
    ///
    /// assert_eq!("3.1415926535897932384626433832795028842", txt);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn pi(env: &'e dyn UsrEnv) -> Self {
        let mut num : OCINumber;
        unsafe {
            num = mem::uninitialized();
            OCINumberSetPi(env.err_ptr(), &mut num);
        }
        Self { env, num }
    }

    /// Creates a new Number from a string using specified format.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_string("6.62607004E-34", "9D999999999EEEE", &env)?;
    /// let txt = num.to_string("TME")?;
    ///
    /// assert_eq!("6.62607004E-34", txt);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn from_string(txt: &str, fmt: &str, env: &'e dyn UsrEnv) -> Result<Self> {
        let mut num : OCINumber;
        catch!{env.err_ptr() =>
            num = mem::uninitialized();
            OCINumberFromText(
                env.err_ptr(),
                txt.as_ptr(), txt.len() as u32,
                fmt.as_ptr(), fmt.len() as u32,
                ptr::null(), 0,
                &mut num
            )
        }
        Ok( Self { env, num } )
    }

    /// Creates a new Number from any Oracle standard machine-native integer type.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_int(42, &env);
    /// let val = num.to_int::<i32>()?;
    ///
    /// assert_eq!(42, val);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn from_int<T:Integer>(val: T, env: &'e dyn UsrEnv) -> Self {
        let num = val.into_number();
        Self { env, num }
    }

    /// Converts a machine-native floating point type to an Oracle number
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_real(2.7182818284590452353602874713527, &env)?;
    /// let txt = num.to_string("TM")?;
    ///
    /// assert_eq!("2.71828182845905", txt);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn from_real<T:Real>(val: T, env: &'e dyn UsrEnv) -> Result<Self> {
        let mut num : OCINumber;
        catch!{env.err_ptr() =>
            num = mem::uninitialized();
            OCINumberFromReal(
                env.err_ptr(),
                &val as *const T as *const c_void, mem::size_of::<T>() as u32,
                &mut num
            )
        }
        Ok( Self { env, num } )
    }

    /// Creates a "copy" of the other number.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_int(8128, &env);
    /// let dup = oracle::Number::from_number(&num, &env)?;
    /// let val: i32 = dup.to_int::<i32>()?;
    ///
    /// assert_eq!(8128, val);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn from_number(other: &Number, env: &'e UsrEnv) -> Result<Self> {
        from_number(&other.num, env)
    }

    /// Returns a raw pointer to the OCINumber struct.
    ///
    /// The caller must ensure that the Number outlives the pointer this function returns,
    /// or else it will end up pointing to garbage.
    pub(crate) fn as_ptr(&self) -> *const OCINumber {
        &self.num
    }

    /// Returns an unsafe mutable pointer to the OCINumber struct.
    ///
    /// The caller must ensure that the Number outlives the pointer this function returns,
    /// or else it will end up pointing to garbage.
    pub(crate) fn as_mut_ptr(&mut self) -> *mut OCINumber {
        &mut self.num
    }

    /// Assigns self the value of the specified number
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let src = oracle::Number::from_int(33550336, &env);
    /// let mut dst = oracle::Number::zero(&env);
    /// let val: i32 = dst.to_int::<i32>()?;
    ///
    /// assert_eq!(0, val);
    ///
    /// dst.assign(&src)?;
    /// let val: i32 = dst.to_int::<i32>()?;
    ///
    /// assert_eq!(33550336, val);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn assign(&mut self, num: &Number) -> Result<()> {
        catch!{self.env.err_ptr() =>
            OCINumberAssign(self.env.err_ptr(), num.as_ptr(), self.as_mut_ptr())
        }
        Ok(())
    }

    /// Converts the given number to a character string according to the specified format.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_int(42, &env);
    /// let txt = num.to_string("FM0G999")?;
    ///
    /// assert_eq!("0,042", txt);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn to_string(&self, fmt: &str) -> Result<String> {
        to_string(fmt, self.as_ptr(), self.env.err_ptr())
    }

    /// Converts this Number into a u128, u64, u32, u16, u8, i128, i64, i32, i16, i8
    /// i128 and u128 are not supported by the OCI
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::pi(&env);
    /// let val = num.to_int::<i32>()?;
    ///
    /// assert_eq!(3, val);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn to_int<T:Integer>(&self) -> Result<T> {
        <T>::from_number(&self.num)
    }

    /// Returns machine-native floating point representation of self
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::pi(&env);
    /// let val = num.to_real::<f64>()?;
    ///
    /// assert!(3.14159265358978 < val && val < 3.14159265358980);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn to_real<T:Real>(&self) -> Result<T> {
        to_real(&self.num, self.env.err_ptr())
    }

    /// Test if this number is equal to zero
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let mut num = oracle::Number::zero(&env);
    ///
    /// assert!(num.is_zero()?);
    ///
    /// num.inc()?;
    ///
    /// assert!(!num.is_zero()?);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn is_zero(&self) -> Result<bool> {
        impl_query!{ self => OCINumberIsZero }
    }

    /// Test if this number is an integer
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::zero(&env);
    ///
    /// assert!(num.is_int()?);
    ///
    /// let num = oracle::Number::pi(&env);
    ///
    /// assert!(!num.is_int()?);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn is_int(&self) -> Result<bool> {
        impl_query!{ self => OCINumberIsInt }
    }

    // Increments Oracle number in place
    ///
    /// It is assumed that the input is an integer between 0 and 100^21-2.
    /// If the is input too large, it will be treated as 0 - the result will be an Oracle number 1.
    /// If the input is not a positive integer, the result will be unpredictable.
    ///
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let mut num = oracle::Number::zero(&env);
    /// num.inc()?;
    /// let val = num.to_int::<i32>()?;
    ///
    /// assert_eq!(1, val);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn inc(&mut self) -> Result<()> {
        catch!{self.env.err_ptr() =>
            OCINumberInc(self.env.err_ptr(), self.as_mut_ptr())
        }
        Ok(())
    }

    // Decrements Oracle number in place
    ///
    /// It is assumed that the input is an integer between 0 and 100^21-2.
    /// If the is input too large, it will be treated as 0 - the result will be an Oracle number 1.
    /// If the input is not a positive integer, the result will be unpredictable.
    ///
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let mut num = oracle::Number::from_int(97, &env);
    /// num.dec()?;
    /// let val = num.to_int::<i32>()?;
    ///
    /// assert_eq!(96, val);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn dec(&mut self) -> Result<()> {
        catch!{self.env.err_ptr() =>
            OCINumberDec(self.env.err_ptr(), self.as_mut_ptr())
        }
        Ok(())
    }

    /// Returns sign of a number (as a result of comparing it to zero).
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_int(19, &env);
    /// let ord = num.sign()?;
    ///
    /// assert_eq!(std::cmp::Ordering::Greater as u32, ord as u32);
    ///
    /// let num = oracle::Number::from_int(-17, &env);
    /// let ord = num.sign()?;
    ///
    /// assert_eq!(std::cmp::Ordering::Less as u32, ord as u32);
    ///
    /// let num = oracle::Number::zero(&env);
    /// let ord = num.sign()?;
    ///
    /// assert_eq!(std::cmp::Ordering::Equal, ord);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn sign(&self) -> Result<Ordering> {
        let mut res : i32;
        catch!{self.env.err_ptr() =>
            res = mem::uninitialized();
            OCINumberSign(self.env.err_ptr(), self.as_ptr(), &mut res)
        }
        let ordering = if res < 0 { Ordering::Less } else if res == 0 { Ordering::Equal } else { Ordering::Greater };
        Ok( ordering )
    }

    /// Compares self to a number.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let pi = oracle::Number::pi(&env);
    /// let e = oracle::Number::from_real(2.71828182845905, &env)?;
    /// let cmp = pi.cmp(&e)?;
    ///
    /// assert_eq!(std::cmp::Ordering::Greater as u32, cmp as u32);
    ///
    /// let cmp = e.cmp(&pi)?;
    ///
    /// assert_eq!(std::cmp::Ordering::Less as u32, cmp as u32);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn cmp(&self, num: &Number) -> Result<Ordering> {
        let mut res : i32;
        catch!{self.env.err_ptr() =>
            res = mem::uninitialized();
            OCINumberCmp(self.env.err_ptr(), self.as_ptr(), num.as_ptr(), &mut res)
        }
        let ordering = if res < 0 { Ordering::Less } else if res == 0 { Ordering::Equal } else { Ordering::Greater };
        Ok( ordering )
    }

    /// Adds a Number to this Number and returns the sum as a new Number
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_int(19, &env);
    /// let arg = oracle::Number::from_int(50, &env);
    /// let res = num.add(&arg)?;
    /// let val = res.to_int::<i32>()?;
    ///
    /// assert_eq!(69, val);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn add(&self, num: &Number) -> Result<Number> {
        impl_op!{ self, num => OCINumberAdd }
    }

    /// Subtracts a Number from this Number and returns the difference as a new Number
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_int(90, &env);
    /// let arg = oracle::Number::from_int(21, &env);
    /// let res = num.sub(&arg)?;
    /// let val = res.to_int::<i32>()?;
    ///
    /// assert_eq!(69, val);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn sub(&self, num: &Number) -> Result<Number> {
        impl_op!{ self, num => OCINumberSub }
    }

    /// Multiplies a Number to this Number and returns the product as a new Number
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_real(3.5, &env)?;
    /// let arg = oracle::Number::from_int(8, &env);
    /// let res = num.mul(&arg)?;
    ///
    /// assert!(res.is_int()?);
    ///
    /// let val = res.to_int::<i32>()?;
    ///
    /// assert_eq!(28, val);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn mul(&self, num: &Number) -> Result<Number> {
        impl_op!{ self, num => OCINumberMul }
    }

    /// Divides a Number (dividend) by a Number (divisor) and returns the quotient as a new Number
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_int(256, &env);
    /// let arg = oracle::Number::from_int(8, &env);
    /// let res = num.div(&arg)?;
    /// let val = res.to_int::<i32>()?;
    ///
    /// assert_eq!(32, val);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn div(&self, num: &Number) -> Result<Number> {
        impl_op!{ self, num => OCINumberDiv }
    }

    /// Finds the remainder of the division of two Numbers and returns it as a new Number
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_int(255, &env);
    /// let arg = oracle::Number::from_int(32, &env);
    /// let res = num.rem(&arg)?;
    /// let val = res.to_int::<i32>()?;
    ///
    /// assert_eq!(31, val);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn rem(&self, num: &Number) -> Result<Number> {
        impl_op!{ self, num => OCINumberMod }
    }

    /// Raises a number to an arbitrary power and returns the result as a new Number
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_real(2.55, &env)?;
    /// let arg = oracle::Number::from_real(3.2, &env)?;
    /// let res = num.pow(&arg)?;
    /// let val = res.to_real::<f64>()?;
    ///
    /// assert!(19.995330061114 < val && val < 19.995330061115);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn pow(&self, num: &Number) -> Result<Number> {
        impl_op!{ self, num => OCINumberPower }
    }

    /// Raises a number to an integer power and returns the result as a new Number
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_real(2.55, &env)?;
    /// let res = num.powi(3)?;
    /// let val = res.to_real::<f64>()?;
    ///
    /// assert!(16.581374999 < val && val < 16.581375001);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn powi(&self, num: i32) -> Result<Number> {
        impl_opi!{ self, num => OCINumberIntPower }
    }

    /// Multiplies a number by by a power of 10 and returns the result as a new Number
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_real(2.55, &env)?;
    /// let res = num.shift(2)?;
    /// let val = res.to_int::<i32>()?;
    ///
    /// assert_eq!(255, val);
    ///
    /// let res = res.shift(-1)?;
    /// let val = res.to_real::<f64>()?;
    ///
    /// assert_eq!(25.5, val);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn shift(&self, num: i32) -> Result<Number> {
        impl_opi!{ self, num => OCINumberShift }
    }

    /// Truncates a number at a specified decimal place and returns the result as a new Number
    /// `num` is the number of decimal digits to the right of the decimal point to truncate at.
    /// Negative values are allowed.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::pi(&env);
    /// let res = num.trunc(7)?;
    /// let val = res.to_string("TM")?;
    ///
    /// assert_eq!("3.1415926", val);
    ///
    /// let res = res.shift(5)?;
    /// let val = res.to_real::<f64>()?;
    ///
    /// assert_eq!(314159.26, val);
    ///
    /// let res = res.trunc(-3)?;
    /// let val = res.to_real::<f64>()?;
    ///
    /// assert_eq!(314000.0, val);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn trunc(&self, num: i32) -> Result<Number> {
        impl_opi!{ self, num => OCINumberTrunc }
    }

    /// Rounds a number to a specified decimal place and returns the result as a new Number.
    /// `num` is the number of decimal digits to the right of the decimal point to truncate at.
    /// Negative values are allowed.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::pi(&env);
    /// let res = num.round(7)?;
    /// let val = res.to_string("TM")?;
    ///
    /// assert_eq!("3.1415927", val);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn round(&self, num: i32) -> Result<Number> {
        impl_opi!{ self, num => OCINumberRound }
    }

    /// Performs a floating point round with respect to the number of digits and returns the result
    /// as a new Number.
    /// `num` is the number of decimal digits desired in the result.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::pi(&env);
    /// let res = num.prec(10)?;
    /// let val = res.to_string("TM")?;
    ///
    /// assert_eq!("3.141592654", val);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn prec(&self, num: i32) -> Result<Number> {
        impl_opi!{ self, num => OCINumberPrec }
    }

    /// Negates a number and returns the result as a new Number
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_int(42, &env);
    /// let res = num.neg()?;
    /// let val = res.to_int::<i32>()?;
    ///
    /// assert_eq!(-42, val);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn neg(&self) -> Result<Number> {
        impl_fn!{ self => OCINumberNeg }
    }

    /// Returns the absolute value of a number
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_int(-42, &env);
    /// let res = num.abs()?;
    /// let val = res.to_int::<i32>()?;
    ///
    /// assert_eq!(42, val);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn abs(&self) -> Result<Number> {
        impl_fn!{ self => OCINumberAbs }
    }

    /// Returns the smallers integer greater than or equal to a number
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::pi(&env);
    /// let res = num.ceil()?;
    ///
    /// assert!(res.is_int()?);
    ///
    /// let val = res.to_int::<i32>()?;
    ///
    /// assert_eq!(4, val);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn ceil(&self) -> Result<Number> {
        impl_fn!{ self => OCINumberCeil }
    }

    /// Returns the largest integer less than or equal to a number
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::pi(&env);
    /// let res = num.floor()?;
    ///
    /// assert!(res.is_int()?);
    ///
    /// let val = res.to_int::<i32>()?;
    ///
    /// assert_eq!(3, val);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn floor(&self) -> Result<Number> {
        impl_fn!{ self => OCINumberFloor }
    }

    /// Returns the square root of a number
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_int(121, &env);
    /// let res = num.sqrt()?;
    ///
    /// assert!(res.is_int()?);
    ///
    /// let val = res.to_int::<i32>()?;
    /// assert_eq!(11, val);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn sqrt(&self) -> Result<Number> {
        impl_fn!{ self => OCINumberSqrt }
    }

    /// Return the sine in radians of a number
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_real(0.52359877559, &env)?;
    /// let res = num.sin()?;
    /// let val = res.to_real::<f64>()?;
    ///
    /// assert!(0.499999999 < val && val < 0.500000001);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn sin(&self) -> Result<Number> {
        impl_fn!{ self => OCINumberSin }
    }

    /// Return the arcsine in radians of a number
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_real(0.5, &env)?;
    /// let res = num.asin()?;
    /// let val = res.to_real::<f64>()?;
    ///
    /// assert!(0.523598775 < val && val < 0.523598776);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn asin(&self) -> Result<Number> {
        impl_fn!{ self => OCINumberArcSin }
    }

    /// Return the hyperbolic sine in radians of a number
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_real(0.88137358702, &env)?;
    /// let res = num.sinh()?;
    /// let val = res.to_real::<f64>()?;
    ///
    /// assert!(0.999999999 < val && val < 1.000000001);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn sinh(&self) -> Result<Number> {
        impl_fn!{ self => OCINumberHypSin }
    }

    /// Return the cosine in radians of a number
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_real(1.0471975512, &env)?;
    /// let res = num.cos()?;
    /// let val = res.to_real::<f64>()?;
    ///
    /// assert!(0.499999999 < val && val < 0.500000001);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn cos(&self) -> Result<Number> {
        impl_fn!{ self => OCINumberCos }
    }

    /// Return the arccosine in radians of a number
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_real(0.5, &env)?;
    /// let res = num.acos()?;
    /// let val = res.to_real::<f64>()?;
    ///
    /// assert!(1.047197551 < val && val < 1.047197552);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn acos(&self) -> Result<Number> {
        impl_fn!{ self => OCINumberArcCos }
    }

    /// Return the hyperbolic cosine in radians of a number
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_real(0.96242365012, &env)?;
    /// let res = num.cosh()?;
    /// let val = res.to_real::<f64>()?;
    ///
    /// assert!(1.499999999 < val && val < 1.500000001);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn cosh(&self) -> Result<Number> {
        impl_fn!{ self => OCINumberHypCos }
    }

    /// Return the tangent in radians of a number
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_real(0.785398163397, &env)?;
    /// let res = num.tan()?;
    /// let val = res.to_real::<f64>()?;
    ///
    /// assert!(0.999999999 < val && val < 1.000000001);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn tan(&self) -> Result<Number> {
        impl_fn!{ self => OCINumberTan }
    }

    /// Return the arctangent in radians of a number
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_int(1, &env);
    /// let res = num.atan()?;
    /// let val = res.to_real::<f64>()?;
    ///
    /// assert!(0.785398163 < val && val < 0.785398164);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn atan(&self) -> Result<Number> {
        impl_fn!{ self => OCINumberArcTan }
    }

    /// Returns the four quadrant arctangent of `self` and `num` in radians
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let x = oracle::Number::from_int(4, &env);
    /// let y = oracle::Number::from_int(-3, &env);
    /// let res = x.atan2(&y)?;
    /// let val = res.to_real::<f64>()?;
    ///
    /// assert!(2.2142974355 < val && val < 2.2142974356);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn atan2(&self, num: &Number) -> Result<Number> {
        impl_op!{ self, num => OCINumberArcTan2 }
    }

    /// Returns the hyperbolic tangent in radians of a number
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_real(0.54930614434, &env)?;
    /// let res = num.tanh()?;
    /// let val = res.to_real::<f64>()?;
    ///
    /// assert!(0.499999999 < val && val < 0.500000001);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn tanh(&self) -> Result<Number> {
        impl_fn!{ self => OCINumberHypTan }
    }

    /// Returns `e^(self)` - the exponential function
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_real(2.71828182845905, &env)?;
    /// let res = num.exp()?;
    /// let val = res.to_real::<f64>()?;
    ///
    /// assert!(15.154262241 < val && val < 15.154262242);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn exp(&self) -> Result<Number> {
        impl_fn!{ self => OCINumberExp }
    }

    /// Returns the natual logarithm of the number
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_real(2.71828182845905, &env)?;
    /// let res = num.ln()?;
    /// let val = res.to_real::<f64>()?;
    ///
    /// assert!(0.9999999999 < val && val < 1.0000000001);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn ln(&self) -> Result<Number> {
        impl_fn!{ self => OCINumberLn }
    }

    /// Returns the logarithm of the numer using with respect to an arbitrary base.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_int(65536, &env);
    /// let base = oracle::Number::from_int(4, &env);
    /// let res = num.log(&base)?;
    /// let val = res.to_int::<i32>()?;
    ///
    /// assert_eq!(8, val);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn log(&self, num: &Number) -> Result<Number> {
        let env = self.env;
        let mut res : OCINumber;
        catch!{env.err_ptr() =>
            res = mem::uninitialized();
            OCINumberLog(env.err_ptr(), num.as_ptr(), self.as_ptr(), &mut res)
        }
        Ok( Number { env, num: res } )
    }

    pub fn size(&self) -> usize {
        mem::size_of::<OCINumber>()
    }
}

impl ToSql for Number<'_> {
    fn to_sql(&self) -> (u16, *const c_void, usize) {
        ( SQLT_VNU, self.as_ptr() as *const c_void, std::mem::size_of::<OCINumber>() )
    }
}

impl ToSqlOut for Number<'_> {
    fn to_sql_output(&mut self, _col_size: usize) -> (u16, *mut c_void, usize) {
        (SQLT_VNU, self.as_mut_ptr() as *mut c_void, std::mem::size_of::<OCINumber>())
    }
}

impl ToSqlOut for Box<OCINumber> {
    fn to_sql_output(&mut self, _col_size: usize) -> (u16, *mut c_void, usize) {
        (SQLT_VNU, self.as_mut() as *mut OCINumber as *mut c_void, std::mem::size_of::<OCINumber>())
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    #[test]
    fn num_from_to_int() -> std::result::Result<(),Box<dyn std::error::Error>> {
        let env = env()?;

        let num = Number::from_int(0, &env);
        let val = num.to_int::<i32>()?;
        assert_eq!(0, val);

        let num = Number::from_int(42, &env);
        let val = num.to_int::<i32>()?;
        assert_eq!(42, val);

        let num = Number::from_int(250_000_000_000i64, &env);
        let val = num.to_int()?;
        assert_eq!(250_000_000_000i64, val);

        let num = Number::from_int(250_000_190_000i64, &env);
        let val = num.to_int()?;
        assert_eq!(250_000_190_000i64, val);

        let num = Number::from_int(-150_000_000_000i64, &env);
        let val = num.to_int()?;
        assert_eq!(-150_000_000_000i64, val);

        let num = Number::from_int(-31415926535897932384626433832795028842i128, &env);
        let txt = num.to_string("TM")?;
        assert_eq!("-31415926535897932384626433832795028842", txt);
        let val = num.to_int()?;
        assert_eq!(-31415926535897932384626433832795028842i128, val);

        let num = Number::from_int(std::u128::MAX, &env);
        let txt = num.to_string("TM")?;
        assert_eq!("340282366920938463463374607431768211455", txt);
        let val = num.to_int()?;
        assert_eq!(std::u128::MAX, val);

        let num = Number::pi(&env);
        let val = num.to_int::<i32>()?;
        assert_eq!(3, val);

        let num = Number::pi(&env);
        let num = num.shift(2)?;
        let val = num.to_int::<i32>()?;
        assert_eq!(314, val);
        let neg = num.neg()?;
        let val = neg.to_int::<i32>()?;
        assert_eq!(-314, val);
        let num = num.shift(5)?;
        let val = num.to_int::<i32>()?;
        assert_eq!(31415927, val);
        let neg = num.neg()?;
        let val = neg.to_int::<i32>()?;
        assert_eq!(-31415927, val);

        Ok(())
    }
}