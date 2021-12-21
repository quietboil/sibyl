//! Functions to manipulate Oracle Numbers:
//! NUMBER, NUMERIC, INT, SHORTINT, REAL, DOUBLE PRECISION, FLOAT and DECIMAL.

mod convert;
mod tosql;

pub(crate) use self::convert::{Integer, Real, from_number, to_string, to_real};

use super::{Ctx, interval::Interval};
use crate::{Result, oci::{self, *}};
use std::{cmp::Ordering, mem, ops::{Deref, DerefMut}};

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
        (*ptr).bytes[0] = 1;
        (*ptr).bytes[1] = 128;
        num.assume_init()
    }
}

// pub(crate) fn new_number<'a>(num: OCINumber, ctx: &'a dyn Ctx) -> Number<'a> {
//     Number { ctx, num }
// }

fn compare(num1: &OCINumber, num2: &OCINumber, err: &OCIError) -> Result<Ordering> {
    let mut cmp = 0i32;
    oci::number_cmp(err, num1, num2, &mut cmp)?;
    let ordering = if cmp < 0 {
        Ordering::Less
    } else if cmp == 0 {
        Ordering::Equal
    } else {
        Ordering::Greater
    };
    Ok(ordering)
}

macro_rules! impl_query {
    ($this:ident => $f:path) => {{
        let mut res: i32 = 0;
        $f($this.ctx.as_ref(), &$this.num, &mut res)?;
        Ok(res != 0)
    }};
}

macro_rules! impl_fn {
    ($this:ident => $f:path) => {{
        let ctx = $this.ctx;
        let mut num = mem::MaybeUninit::<OCINumber>::uninit();
        $f(ctx.as_ref(), &$this.num, num.as_mut_ptr())?;
        let num = unsafe { num.assume_init() };
        Ok(Number { num, ctx })
    }};
}

macro_rules! impl_op {
    ($this:ident, $other:ident => $f:path) => {{
        let ctx = $this.ctx;
        let mut num = mem::MaybeUninit::<OCINumber>::uninit();
        $f(ctx.as_ref(), &$this.num, &$other.num, num.as_mut_ptr())?;
        let num = unsafe { num.assume_init() };
        Ok(Number { num, ctx })
    }};
}

macro_rules! impl_opi {
    ($this:ident, $int:ident => $f:path) => {{
        let ctx = $this.ctx;
        let mut num = mem::MaybeUninit::<OCINumber>::uninit();
        $f(ctx.as_ref(), &$this.num, $int, num.as_mut_ptr())?;
        let num = unsafe { num.assume_init() };
        Ok(Number { num, ctx })
    }};
}

/// Represents OTS types NUMBER, NUMERIC, INT, SHORTINT, REAL, DOUBLE PRECISION, FLOAT and DECIMAL.
pub struct Number<'a> {
    ctx: &'a dyn Ctx,
    num: OCINumber,
}

impl AsRef<OCINumber> for Number<'_> {
    fn as_ref(&self) -> &OCINumber {
        &self.num
    }
}

impl AsMut<OCINumber> for Number<'_> {
    fn as_mut(&mut self) -> &mut OCINumber {
        &mut self.num
    }
}

impl Deref for Number<'_> {
    type Target = OCINumber;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl DerefMut for Number<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl<'a> Number<'a> {
    pub(crate) fn to_interval<T: DescriptorType<OCIType=OCIInterval>>(&self) -> Result<Interval<'a, T>> {
        let mut interval = Descriptor::<T>::new(&self.ctx)?;
        oci::interval_from_number(self.ctx.as_context(), self.ctx.as_ref(), &mut interval, &self.num)?;
        Ok(Interval::from(interval, self.ctx))
    }

    pub(crate) fn from(src: &OCINumber, ctx: &'a dyn Ctx) -> Result<Self> {
        let num = from_number(src, ctx.as_ref())?;
        Ok(Self {num, ctx})
    }

    pub(crate) fn make(num: OCINumber, ctx: &'a dyn Ctx) -> Self {
        Self {num, ctx}
    }

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

        let num = Number::zero(&env)?;

        assert!(num.is_zero()?);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn zero(ctx: &'a dyn Ctx) -> Result<Self> {
        let mut num = mem::MaybeUninit::<OCINumber>::uninit();
        oci::number_set_zero(ctx.as_ref(), num.as_mut_ptr())?;
        let num = unsafe { num.assume_init() };
        Ok(Self { ctx, num })
    }

    /**
        Creates a new Number that is equal to Pi.

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::pi(&env)?;

        assert_eq!(num.to_string("TM")?, "3.1415926535897932384626433832795028842");
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn pi(ctx: &'a dyn Ctx) -> Result<Self> {
        let mut num = mem::MaybeUninit::<OCINumber>::uninit();
        oci::number_set_pi(ctx.as_ref(), num.as_mut_ptr())?;
        let num = unsafe { num.assume_init() };
        Ok(Self { ctx, num })
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
        oci::number_from_text(
            ctx.as_ref(),
            txt.as_ptr(),
            txt.len() as u32,
            fmt.as_ptr(),
            fmt.len() as u32,
            num.as_mut_ptr(),
        )?;
        Ok(Self {
            ctx,
            num: unsafe { num.assume_init() },
        })
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
    pub fn from_int<T: Integer>(val: T, ctx: &'a dyn Ctx) -> Result<Self> {
        let num = val.into_number(ctx.as_ref())?;
        Ok(Self { ctx, num })
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
    pub fn from_real<T: Real>(val: T, ctx: &'a dyn Ctx) -> Result<Self> {
        let num = val.into_number(ctx.as_ref())?;
        Ok(Self { ctx, num })
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
        let num = from_number(&other.num, other.ctx.as_ref())?;
        Ok(Self {num, ..*other})
    }

    /**
        Assigns self the value of the specified number

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let src = Number::from_int(33550336, &env)?;
        let mut dst = Number::zero(&env)?;
        assert_eq!(dst.to_int::<i32>()?, 0);

        dst.assign(&src)?;
        assert_eq!(dst.to_int::<i32>()?, 33550336);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn assign(&mut self, src: &Number) -> Result<()> {
        oci::number_assign(self.ctx.as_ref(), &src.num, &mut self.num)
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
        to_string(fmt, &self.num, self.ctx.as_ref())
    }

    /**
        Converts this Number into an integer (u128, u64, u32, u16, u8, i128, i64, i32, i16, i8).

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::pi(&env)?;
        let val = num.to_int::<i32>()?;

        assert_eq!(val, 3);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn to_int<T: Integer>(&self) -> Result<T> {
        <T>::from_number(&self.num, self.ctx.as_ref())
    }

    /**
        Returns floating point representation of self

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::pi(&env)?;
        let val = num.to_real::<f64>()?;

        assert!(3.14159265358978 < val && val < 3.14159265358980);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn to_real<T: Real>(&self) -> Result<T> {
        to_real(&self.num, self.ctx.as_ref())
    }

    /**
        Test if this number is equal to zero

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let mut num = Number::zero(&env)?;

        assert!(num.is_zero()?);

        num.inc()?;

        assert!(!num.is_zero()?);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn is_zero(&self) -> Result<bool> {
        impl_query!(self => oci::number_is_zero)
    }

    /**
        Tests if this number is an integer

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::zero(&env)?;

        assert!(num.is_int()?);

        let num = Number::pi(&env)?;

        assert!(!num.is_int()?);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn is_int(&self) -> Result<bool> {
        impl_query!(self => oci::number_is_int)
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

        let mut num = Number::zero(&env)?;
        num.inc()?;

        assert_eq!(num.to_int::<i32>()?, 1);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn inc(&mut self) -> Result<()> {
        oci::number_inc(self.ctx.as_ref(), &mut self.num)
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
        oci::number_dec(self.ctx.as_ref(), &mut self.num)
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

        let num = Number::zero(&env)?;

        assert_eq!(num.sign()?, Ordering::Equal);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn sign(&self) -> Result<Ordering> {
        let mut res = 0i32;
        oci::number_sign(self.ctx.as_ref(), &self.num, &mut res)?;
        let ordering = if res == 0 {
            Ordering::Equal
        } else if res < 0 {
            Ordering::Less
        } else {
            Ordering::Greater
        };
        Ok(ordering)
    }

    /**
        Compares self to a number.

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        use std::cmp::Ordering;
        let env = oracle::env()?;

        let pi = Number::pi(&env)?;
        let e = Number::from_real(2.71828182845905, &env)?;

        assert_eq!(pi.compare(&e)?, Ordering::Greater);
        assert_eq!(e.compare(&pi)?, Ordering::Less);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn compare(&self, other: &Self) -> Result<Ordering> {
        compare(&self.num, &other.num, self.ctx.as_ref())
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
        impl_op!(self, num => oci::number_add)
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
        impl_op!(self, num => oci::number_sub)
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
        impl_op!(self, num => oci::number_mul)
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
        impl_op!(self, num => oci::number_div)
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
        impl_op!(self, num => oci::number_mod)
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
        impl_op!(self, num => oci::number_power)
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
        impl_opi!(self, num => oci::number_int_power)
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
        impl_opi!(self, num => oci::number_shift)
    }

    /**
        Truncates a number at a specified decimal place and returns the result as a new Number
        `num` is the number of decimal digits to the right of the decimal point to truncate at.
        Negative values are allowed.

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::pi(&env)?;
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
        impl_opi!(self, num => oci::number_trunc)
    }

    /**
        Rounds a number to a specified decimal place and returns the result as a new Number.
        `num` is the number of decimal digits to the right of the decimal point to truncate at.
        Negative values are allowed.

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::pi(&env)?;
        let res = num.round(7)?;

        assert_eq!(res.to_string("TM")?, "3.1415927");
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn round(&self, num: i32) -> Result<Self> {
        impl_opi!(self, num => oci::number_round)
    }

    /**
        Performs a floating point round with respect to the number of digits and returns the result
        as a new Number.

        `num` is the number of decimal digits desired in the result.

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::pi(&env)?;
        let res = num.prec(10)?;

        assert_eq!(res.to_string("TM")?, "3.141592654");
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn prec(&self, num: i32) -> Result<Self> {
        impl_opi!(self, num => oci::number_prec)
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
        impl_fn!(self => oci::number_neg)
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
        impl_fn!(self => oci::number_abs)
    }

    /**
        Returns the smallers integer greater than or equal to a number

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::pi(&env)?;
        let res = num.ceil()?;

        assert!(res.is_int()?);
        assert_eq!(res.to_int::<i32>()?, 4);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn ceil(&self) -> Result<Self> {
        impl_fn!(self => oci::number_ceil)
    }

    /**
        Returns the largest integer less than or equal to a number

        # Example
        ```
        use sibyl::{ self as oracle, Number };
        let env = oracle::env()?;

        let num = Number::pi(&env)?;
        let res = num.floor()?;

        assert!(res.is_int()?);
        assert_eq!(res.to_int::<i32>()?, 3);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn floor(&self) -> Result<Self> {
        impl_fn!(self => oci::number_floor)
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
        impl_fn!(self => oci::number_sqrt)
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
        impl_fn!(self => oci::number_sin)
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
        impl_fn!(self => oci::number_arc_sin)
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
        impl_fn!(self => oci::number_hyp_sin)
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
        impl_fn!(self => oci::number_cos)
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
        impl_fn!(self => oci::number_arc_cos)
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
        impl_fn!(self => oci::number_hyp_cos)
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
        impl_fn!(self => oci::number_tan)
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
        impl_fn!(self => oci::number_arc_tan)
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
        impl_op!(self, num => oci::number_arc_tan2)
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
        impl_fn!(self => oci::number_hyp_tan)
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
        impl_fn!(self => oci::number_exp)
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
        impl_fn!(self => oci::number_ln)
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
        oci::number_log(ctx.as_ref(), &num.num, &self.num, res.as_mut_ptr())?;
        let num = unsafe { res.assume_init() }; 
        Ok(Number {num, ctx})
    }

    pub fn size(&self) -> usize {
        mem::size_of::<OCINumber>()
    }
}

impl std::fmt::Debug for Number<'_> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.to_string("TM") {
            Ok(txt) => fmt.write_fmt(format_args!("Number({})", txt)),
            Err(err) => fmt.write_fmt(format_args!("Number({})", err)),
        }
    }
}
