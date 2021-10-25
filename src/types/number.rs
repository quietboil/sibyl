//! Functions to manipulate Oracle Numbers:
//! NUMBER, NUMERIC, INT, SHORTINT, REAL, DOUBLE PRECISION, FLOAT and DECIMAL.

mod convert;
mod tosql;

use self::convert::{IntoNumber, FromNumber};
use super::Ctx;
use crate::{ Result, oci::* };
use libc::c_void;
use std::{ mem, ptr, cmp::Ordering };

/// Marker trait for integer numbers
pub trait Integer : IntoNumber + FromNumber {}

macro_rules! impl_int {
    ($($t:ty),+) => {
        $(
            impl Integer for $t {}
        )+
    };
}

impl_int!(i8, i16, i32, i64, i128, isize);
impl_int!(u8, u16, u32, u64, u128, usize);

/// Marker trait for floating ppoint numbers
pub trait Real : IntoNumber + FromNumber {}
impl Real for f32 {}
impl Real for f64 {}

/// C mapping of the Oracle NUMBER
#[repr(C)] pub struct OCINumber { _private: [u8; 22] }

pub(crate) fn to_string(fmt: &str, num: *const OCINumber, err: *mut OCIError) -> Result<String> {
    let mut txt : [u8;64] = unsafe { mem::MaybeUninit::uninit().assume_init() };
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

pub(crate) fn from_number<'a>(from_num: &OCINumber, ctx: &'a dyn Ctx) -> Result<Number<'a>> {
    let mut num = mem::MaybeUninit::<OCINumber>::uninit();
    catch!{ctx.err_ptr() =>
        OCINumberAssign(ctx.err_ptr(), from_num as *const OCINumber, num.as_mut_ptr())
    }
    Ok( Number { ctx, num: unsafe { num.assume_init() } } )
}

/**
    Creates an OCI number initialized as zero. This simplified version of `u128_into_number`
    is used to create an output variable buffer.

    **Note** that we cannot always pass `uninit` version of `OCINumber` to be used for output.
    While `uninit` variant works on Windows, it fails with ORA-01458 on Linux.
*/
pub(crate) fn new() -> OCINumber {
    let mut num = mem::MaybeUninit::<OCINumber>::uninit();
    let ptr = num.as_mut_ptr();
    unsafe {
        (*ptr)._private[0] = 1;
        (*ptr)._private[1] = 128;
        num.assume_init()
    }
}

pub(crate) fn new_number<'a>(num: OCINumber, ctx: &'a dyn Ctx) -> Number<'a> {
    Number { ctx, num }
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

pub(crate) fn real_into_number<T:Real>(val: T, err: *mut OCIError) -> Result<OCINumber> {
    let mut num = mem::MaybeUninit::<OCINumber>::uninit();
    catch!{err =>
        OCINumberFromReal(
            err,
            &val as *const T as *const c_void, mem::size_of::<T>() as u32,
            num.as_mut_ptr()
        )
    }
    Ok( unsafe { num.assume_init() } )
}

pub(crate) fn to_real<T:Real>(num: &OCINumber, err: *mut OCIError) -> Result<T> {
    let mut res = mem::MaybeUninit::<T>::uninit();
    catch!{err =>
        OCINumberToReal(err, num as *const OCINumber, mem::size_of::<T>() as u32, res.as_mut_ptr() as *mut c_void)
    }
    Ok( unsafe { res.assume_init() } )
}

fn compare(num1: &OCINumber, num2: &OCINumber, err: *mut OCIError) -> Result<Ordering> {
    let mut res = mem::MaybeUninit::<i32>::uninit();
    catch!{err =>
        OCINumberCmp(err, num1 as *const OCINumber, num2 as *const OCINumber, res.as_mut_ptr())
    }
    let res = unsafe { res.assume_init() };
    let ordering = if res < 0 { Ordering::Less } else if res == 0 { Ordering::Equal } else { Ordering::Greater };
    Ok( ordering )
}

macro_rules! impl_query {
    ($this:ident => $f:ident) => {
        let mut res : i32 = 0;
        catch!{$this.ctx.err_ptr() =>
            $f($this.ctx.err_ptr(), $this.as_ptr(), &mut res)
        }
        Ok( res != 0 )
    };
}

macro_rules! impl_fn {
    ($this:ident => $f:ident) => {
        let ctx = $this.ctx;
        let mut num = mem::MaybeUninit::<OCINumber>::uninit();
        catch!{ctx.err_ptr() =>
            $f(ctx.err_ptr(), $this.as_ptr(), num.as_mut_ptr())
        }
        Ok( Number { ctx, num: unsafe { num.assume_init() } } )
    };
}

macro_rules! impl_op {
    ($this:ident, $arg:ident => $f:ident) => {
        let ctx = $this.ctx;
        let mut num = mem::MaybeUninit::<OCINumber>::uninit();
        catch!{ctx.err_ptr() =>
            $f(ctx.err_ptr(), $this.as_ptr(), $arg.as_ptr(), num.as_mut_ptr())
        }
        Ok( Number { ctx, num: unsafe { num.assume_init() } } )
    };
}

macro_rules! impl_opi {
    ($this:ident, $arg:ident => $f:ident) => {
        let ctx = $this.ctx;
        let mut num = mem::MaybeUninit::<OCINumber>::uninit();
        catch!{ctx.err_ptr() =>
            $f(ctx.err_ptr(), $this.as_ptr(), $arg, num.as_mut_ptr())
        }
        Ok( Number { ctx, num: unsafe { num.assume_init() } } )
    };
}

/// Represents OTS types NUMBER, NUMERIC, INT, SHORTINT, REAL, DOUBLE PRECISION, FLOAT and DECIMAL.
pub struct Number<'a> {
    pub(crate) ctx: &'a dyn Ctx,
    num: OCINumber,
}

impl<'a> Number<'a> {
    /// Returns a new uninitialized number.
    pub fn new(ctx: &'a dyn Ctx) -> Self {
        Self { ctx, num: new() }
    }

    /**
        Creates a new Number that is equal to zero.

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::zero(&env);

        assert!(num.is_zero()?);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn zero(ctx: &'a dyn Ctx) -> Self {
        let mut num = mem::MaybeUninit::<OCINumber>::uninit();
        unsafe {
            OCINumberSetZero(ctx.err_ptr(), num.as_mut_ptr());
        }
        Self { ctx, num: unsafe { num.assume_init() } }
    }

    /**
        Creates a new Number that is equal to Pi.

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::pi(&env);

        assert_eq!(num.to_string("TM")?, "3.1415926535897932384626433832795028842");
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn pi(ctx: &'a dyn Ctx) -> Self {
        let mut num = mem::MaybeUninit::<OCINumber>::uninit();
        unsafe {
            OCINumberSetPi(ctx.err_ptr(), num.as_mut_ptr());
        }
        Self { ctx, num: unsafe { num.assume_init() } }
    }

    /**
        Creates a new Number from a string using specified format.

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::from_string("6.62607004E-34", "9D999999999EEEE", &env)?;

        assert_eq!(num.to_string("TME")?, "6.62607004E-34");
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn from_string(txt: &str, fmt: &str, ctx: &'a dyn Ctx) -> Result<Self> {
        let mut num = mem::MaybeUninit::<OCINumber>::uninit();
        catch!{ctx.err_ptr() =>
            OCINumberFromText(
                ctx.err_ptr(),
                txt.as_ptr(), txt.len() as u32,
                fmt.as_ptr(), fmt.len() as u32,
                ptr::null(), 0,
                num.as_mut_ptr()
            )
        }
        Ok( Self { ctx, num: unsafe { num.assume_init() } } )
    }

    /**
        Creates a new Number from an integer.

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::from_int(42, &env)?;

        assert!(num.is_int()?);
        assert_eq!(num.to_int::<i32>()?, 42);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn from_int<T:Integer>(val: T, ctx: &'a dyn Ctx) -> Result<Self> {
        let num = val.into_number(ctx.err_ptr())?;
        Ok( Self { ctx, num } )
    }

    /**
        Creates a new Number from a floating point number.

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::from_real(2.7182818284590452353602874713527, &env)?;

        assert_eq!(num.to_string("TM")?, "2.71828182845905");
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn from_real<T:Real>(val: T, ctx: &'a dyn Ctx) -> Result<Self> {
        let num = val.into_number(ctx.err_ptr())?;
        Ok( Self { ctx, num } )
    }

    /**
        Creates a clone of the other number.

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::from_int(8128, &env)?;
        let dup = Number::from_number(&num)?;

        assert_eq!(dup.to_int::<i32>()?, 8128);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn from_number(other: &'a Number) -> Result<Self> {
        from_number(&other.num, other.ctx)
    }

    /**
        Returns a raw pointer to the OCINumber struct.

        The caller must ensure that the Number outlives the pointer this function returns,
        or else it will end up pointing to garbage.
    */
    pub(crate) fn as_ptr(&self) -> *const OCINumber {
        &self.num
    }

    /**
        Returns an unsafe mutable pointer to the OCINumber struct.

        The caller must ensure that the Number outlives the pointer this function returns,
        or else it will end up pointing to garbage.
    */
    pub(crate) fn as_mut_ptr(&mut self) -> *mut OCINumber {
        &mut self.num
    }

    /**
        Assigns self the value of the specified number

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let src = Number::from_int(33550336, &env)?;
        let mut dst = Number::zero(&env);
        assert_eq!(dst.to_int::<i32>()?, 0);

        dst.assign(&src)?;
        assert_eq!(dst.to_int::<i32>()?, 33550336);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn assign(&mut self, num: &Number) -> Result<()> {
        catch!{self.ctx.err_ptr() =>
            OCINumberAssign(self.ctx.err_ptr(), num.as_ptr(), self.as_mut_ptr())
        }
        Ok(())
    }

    /**
        Converts the given number to a character string according to the specified format.

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::from_int(42, &env)?;
        let txt = num.to_string("FM0G999")?;

        assert_eq!(txt, "0,042");
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn to_string(&self, fmt: &str) -> Result<String> {
        to_string(fmt, self.as_ptr(), self.ctx.err_ptr())
    }

    /**
        Converts this Number into an integer (u128, u64, u32, u16, u8, i128, i64, i32, i16, i8).

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::pi(&env);
        let val = num.to_int::<i32>()?;

        assert_eq!(val, 3);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn to_int<T:Integer>(&self) -> Result<T> {
        <T>::from_number(&self.num, self.ctx.err_ptr())
    }

    /**
        Returns floating point representation of self

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::pi(&env);
        let val = num.to_real::<f64>()?;

        assert!(3.14159265358978 < val && val < 3.14159265358980);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn to_real<T:Real>(&self) -> Result<T> {
        to_real(&self.num, self.ctx.err_ptr())
    }

    /**
        Test if this number is equal to zero

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let mut num = Number::zero(&env);

        assert!(num.is_zero()?);

        num.inc()?;

        assert!(!num.is_zero()?);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn is_zero(&self) -> Result<bool> {
        impl_query!{ self => OCINumberIsZero }
    }

    /**
        Tests if this number is an integer

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::zero(&env);

        assert!(num.is_int()?);

        let num = Number::pi(&env);

        assert!(!num.is_int()?);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn is_int(&self) -> Result<bool> {
        impl_query!{ self => OCINumberIsInt }
    }

    /**
        Increments Oracle number in place

        It is assumed that the input is an integer between 0 and 100^21-2.
        If the is input too large, it will be treated as 0 - the result will be an Oracle number 1.
        If the input is not a positive integer, the result will be unpredictable.

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let mut num = Number::zero(&env);
        num.inc()?;

        assert_eq!(num.to_int::<i32>()?, 1);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn inc(&mut self) -> Result<()> {
        catch!{self.ctx.err_ptr() =>
            OCINumberInc(self.ctx.err_ptr(), self.as_mut_ptr())
        }
        Ok(())
    }

    /**
        Decrements Oracle number in place

        It is assumed that the input is an integer between 0 and 100^21-2.
        If the is input too large, it will be treated as 0 - the result will be an Oracle number 1.
        If the input is not a positive integer, the result will be unpredictable.

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let mut num = Number::from_int(97, &env)?;
        num.dec()?;

        assert_eq!(num.to_int::<i32>()?, 96);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn dec(&mut self) -> Result<()> {
        catch!{self.ctx.err_ptr() =>
            OCINumberDec(self.ctx.err_ptr(), self.as_mut_ptr())
        }
        Ok(())
    }

    /**
        Returns sign of a number (as a result of comparing it to zero).

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        use std::cmp::Ordering;
        let env = oracle::env()?;

        let num = Number::from_int(19, &env)?;

        assert_eq!(num.sign()?, Ordering::Greater);

        let num = Number::from_int(-17, &env)?;

        assert_eq!(num.sign()?, Ordering::Less);

        let num = Number::zero(&env);

        assert_eq!(num.sign()?, Ordering::Equal);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn sign(&self) -> Result<Ordering> {
        let mut res = mem::MaybeUninit::<i32>::uninit();
        catch!{self.ctx.err_ptr() =>
            OCINumberSign(self.ctx.err_ptr(), self.as_ptr(), res.as_mut_ptr())
        }
        let res = unsafe { res.assume_init() };
        let ordering = if res == 0 { Ordering::Equal } else if res < 0 { Ordering::Less } else { Ordering::Greater };
        Ok( ordering )
    }

    /**
        Compares self to a number.

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        use std::cmp::Ordering;
        let env = oracle::env()?;

        let pi = Number::pi(&env);
        let e = Number::from_real(2.71828182845905, &env)?;

        assert_eq!(pi.compare(&e)?, Ordering::Greater);
        assert_eq!(e.compare(&pi)?, Ordering::Less);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn compare(&self, other: &Self) -> Result<Ordering> {
        compare(&self.num, &other.num, self.ctx.err_ptr())
    }

    /**
        Adds a Number to this Number and returns the sum as a new Number

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::from_int(19, &env)?;
        let arg = Number::from_int(50, &env)?;
        let res = num.add(&arg)?;

        assert_eq!(res.to_int::<i32>()?, 69);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn add(&self, num: &Number) -> Result<Self> {
        impl_op!{ self, num => OCINumberAdd }
    }

    /**
        Subtracts a Number from this Number and returns the difference as a new Number

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::from_int(90, &env)?;
        let arg = Number::from_int(21, &env)?;
        let res = num.sub(&arg)?;

        assert_eq!(res.to_int::<i32>()?, 69);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn sub(&self, num: &Number) -> Result<Self> {
        impl_op!{ self, num => OCINumberSub }
    }

    /**
        Multiplies a Number to this Number and returns the product as a new Number

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::from_real(3.5, &env)?;
        let arg = Number::from_int(8, &env)?;
        let res = num.mul(&arg)?;

        assert!(res.is_int()?);
        assert_eq!(res.to_int::<i32>()?, 28);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn mul(&self, num: &Number) -> Result<Self> {
        impl_op!{ self, num => OCINumberMul }
    }

    /**
        Divides a Number (dividend) by a Number (divisor) and returns the quotient as a new Number

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::from_int(256, &env)?;
        let arg = Number::from_int(8, &env)?;
        let res = num.div(&arg)?;

        assert!(res.is_int()?);
        assert_eq!(res.to_int::<i32>()?, 32);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn div(&self, num: &Number) -> Result<Self> {
        impl_op!{ self, num => OCINumberDiv }
    }

    /**
        Finds the remainder of the division of two Numbers and returns it as a new Number

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::from_int(255, &env)?;
        let arg = Number::from_int(32, &env)?;
        let res = num.rem(&arg)?;

        assert!(res.is_int()?);
        assert_eq!(res.to_int::<i32>()?, 31);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn rem(&self, num: &Number) -> Result<Self> {
        impl_op!{ self, num => OCINumberMod }
    }

    /**
        Raises a number to an arbitrary power and returns the result as a new Number

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::from_real(2.55, &env)?;
        let arg = Number::from_real(3.2, &env)?;
        let res = num.pow(&arg)?;
        let val = res.to_real::<f64>()?;

        assert!(19.995330061114 < val && val < 19.995330061115);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn pow(&self, num: &Number) -> Result<Self> {
        impl_op!{ self, num => OCINumberPower }
    }

    /**
        Raises a number to an integer power and returns the result as a new Number

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::from_real(2.55, &env)?;
        let res = num.powi(3)?;
        let val = res.to_real::<f64>()?;

        assert!(16.581374999 < val && val < 16.581375001);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn powi(&self, num: i32) -> Result<Self> {
        impl_opi!{ self, num => OCINumberIntPower }
    }

    /**
        Multiplies a number by by a power of 10 and returns the result as a new Number

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::from_real(2.55, &env)?;
        let res = num.pow10(2)?;
        assert_eq!(res.to_int::<i32>()?, 255);

        let res = res.pow10(-1)?;
        assert_eq!(res.to_real::<f64>()?, 25.5);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn pow10(&self, num: i32) -> Result<Self> {
        impl_opi!{ self, num => OCINumberShift }
    }

    /**
        Truncates a number at a specified decimal place and returns the result as a new Number
        `num` is the number of decimal digits to the right of the decimal point to truncate at.
        Negative values are allowed.

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::pi(&env);
        let res = num.trunc(7)?;
        assert_eq!(res.to_string("TM")?, "3.1415926");

        let res = res.pow10(5)?;
        assert_eq!(res.to_real::<f64>()?, 314159.26);

        let res = res.trunc(-3)?;
        assert_eq!(res.to_real::<f64>()?, 314000.0);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn trunc(&self, num: i32) -> Result<Self> {
        impl_opi!{ self, num => OCINumberTrunc }
    }

    /**
        Rounds a number to a specified decimal place and returns the result as a new Number.
        `num` is the number of decimal digits to the right of the decimal point to truncate at.
        Negative values are allowed.

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::pi(&env);
        let res = num.round(7)?;

        assert_eq!(res.to_string("TM")?, "3.1415927");
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn round(&self, num: i32) -> Result<Self> {
        impl_opi!{ self, num => OCINumberRound }
    }

    /**
        Performs a floating point round with respect to the number of digits and returns the result
        as a new Number.

        `num` is the number of decimal digits desired in the result.

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::pi(&env);
        let res = num.prec(10)?;

        assert_eq!(res.to_string("TM")?, "3.141592654");
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn prec(&self, num: i32) -> Result<Self> {
        impl_opi!{ self, num => OCINumberPrec }
    }

    /**
        Negates a number and returns the result as a new Number

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::from_int(42, &env)?;
        let res = num.neg()?;

        assert_eq!(res.to_int::<i32>()?, -42);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn neg(&self) -> Result<Self> {
        impl_fn!{ self => OCINumberNeg }
    }

    /**
        Returns the absolute value of a number

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::from_int(-42, &env)?;
        let res = num.abs()?;

        assert_eq!(res.to_int::<i32>()?, 42);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn abs(&self) -> Result<Self> {
        impl_fn!{ self => OCINumberAbs }
    }

    /**
        Returns the smallers integer greater than or equal to a number

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::pi(&env);
        let res = num.ceil()?;

        assert!(res.is_int()?);
        assert_eq!(res.to_int::<i32>()?, 4);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn ceil(&self) -> Result<Self> {
        impl_fn!{ self => OCINumberCeil }
    }

    /**
        Returns the largest integer less than or equal to a number

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::pi(&env);
        let res = num.floor()?;

        assert!(res.is_int()?);
        assert_eq!(res.to_int::<i32>()?, 3);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn floor(&self) -> Result<Self> {
        impl_fn!{ self => OCINumberFloor }
    }

    /**
        Returns the square root of a number

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::from_int(121, &env)?;
        let res = num.sqrt()?;

        assert!(res.is_int()?);
        assert_eq!(res.to_int::<i32>()?, 11);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn sqrt(&self) -> Result<Self> {
        impl_fn!{ self => OCINumberSqrt }
    }

    /**
        Return the sine in radians of a number

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::from_real(0.52359877559, &env)?;
        let res = num.sin()?;
        let val = res.to_real::<f64>()?;

        assert!(0.499999999 < val && val < 0.500000001);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn sin(&self) -> Result<Self> {
        impl_fn!{ self => OCINumberSin }
    }

    /**
        Return the arcsine in radians of a number

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::from_real(0.5, &env)?;
        let res = num.asin()?;
        let val = res.to_real::<f64>()?;

        assert!(0.523598775 < val && val < 0.523598776);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn asin(&self) -> Result<Self> {
        impl_fn!{ self => OCINumberArcSin }
    }

    /**
        Return the hyperbolic sine in radians of a number

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::from_real(0.88137358702, &env)?;
        let res = num.sinh()?;
        let val = res.to_real::<f64>()?;

        assert!(0.999999999 < val && val < 1.000000001);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn sinh(&self) -> Result<Self> {
        impl_fn!{ self => OCINumberHypSin }
    }

    /**
        Return the cosine in radians of a number

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::from_real(1.0471975512, &env)?;
        let res = num.cos()?;
        let val = res.to_real::<f64>()?;

        assert!(0.499999999 < val && val < 0.500000001);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn cos(&self) -> Result<Self> {
        impl_fn!{ self => OCINumberCos }
    }

    /**
        Return the arccosine in radians of a number

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::from_real(0.5, &env)?;
        let res = num.acos()?;
        let val = res.to_real::<f64>()?;

        assert!(1.047197551 < val && val < 1.047197552);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn acos(&self) -> Result<Self> {
        impl_fn!{ self => OCINumberArcCos }
    }

    /**
        Return the hyperbolic cosine in radians of a number

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::from_real(0.96242365012, &env)?;
        let res = num.cosh()?;
        let val = res.to_real::<f64>()?;

        assert!(1.499999999 < val && val < 1.500000001);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn cosh(&self) -> Result<Self> {
        impl_fn!{ self => OCINumberHypCos }
    }

    /**
        Return the tangent in radians of a number

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::from_real(0.785398163397, &env)?;
        let res = num.tan()?;
        let val = res.to_real::<f64>()?;

        assert!(0.999999999 < val && val < 1.000000001);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn tan(&self) -> Result<Self> {
        impl_fn!{ self => OCINumberTan }
    }

    /**
        Return the arctangent in radians of a number

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::from_int(1, &env)?;
        let res = num.atan()?;
        let val = res.to_real::<f64>()?;

        assert!(0.785398163 < val && val < 0.785398164);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn atan(&self) -> Result<Self> {
        impl_fn!{ self => OCINumberArcTan }
    }

    /**
        Returns the four quadrant arctangent of `self` and `num` in radians

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let x = Number::from_int(4, &env)?;
        let y = Number::from_int(-3, &env)?;
        let res = x.atan2(&y)?;
        let val = res.to_real::<f64>()?;

        assert!(2.2142974355 < val && val < 2.2142974356);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn atan2(&self, num: &Number) -> Result<Self> {
        impl_op!{ self, num => OCINumberArcTan2 }
    }

    /**
        Returns the hyperbolic tangent in radians of a number

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::from_real(0.54930614434, &env)?;
        let res = num.tanh()?;
        let val = res.to_real::<f64>()?;

        assert!(0.499999999 < val && val < 0.500000001);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn tanh(&self) -> Result<Self> {
        impl_fn!{ self => OCINumberHypTan }
    }

    /**
        Returns `e^(self)` - the exponential function

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::from_real(2.71828182845905, &env)?;
        let res = num.exp()?;
        let val = res.to_real::<f64>()?;

        assert!(15.154262241 < val && val < 15.154262242);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn exp(&self) -> Result<Self> {
        impl_fn!{ self => OCINumberExp }
    }

    /**
        Returns the natural logarithm of the number

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::from_real(2.71828182845905, &env)?;
        let res = num.ln()?;
        let val = res.to_real::<f64>()?;

        assert!(0.9999999999 < val && val < 1.0000000001);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn ln(&self) -> Result<Self> {
        impl_fn!{ self => OCINumberLn }
    }

    /**
        Returns the logarithm of the numer using with respect to an arbitrary base.

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::from_int(65536, &env)?;
        let base = Number::from_int(4, &env)?;
        let res = num.log(&base)?;

        assert_eq!(res.to_int::<i32>()?, 8);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn log(&self, num: &Number) -> Result<Self> {
        let ctx = self.ctx;
        let mut res = mem::MaybeUninit::<OCINumber>::uninit();
        catch!{ctx.err_ptr() =>
            OCINumberLog(ctx.err_ptr(), num.as_ptr(), self.as_ptr(), res.as_mut_ptr())
        }
        Ok( Number { ctx, num: unsafe { res.assume_init() } } )
    }

    pub fn size(&self) -> usize {
        mem::size_of::<OCINumber>()
    }
}

impl std::fmt::Debug for Number<'_> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.to_string("TM") {
            Ok(txt)  => fmt.write_fmt(format_args!("Number({})", txt)),
            Err(err) => fmt.write_fmt(format_args!("Number({})", err))
        }
    }
}
