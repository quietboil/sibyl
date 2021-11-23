//! The Oracle time interval data types: INTERVAL YEAR TO MONTH and INTERVAL DAY TO SECOND

mod tosql;

use super::{Ctx, Number};
use crate::{Result, oci::{self, *}};
use libc::size_t;
use std::{mem, cmp::Ordering};

pub(crate) fn to_string(lfprec: u8, fsprec: u8, int: *const OCIInterval, ctx: &dyn Ctx) -> Result<String> {
    let mut name: [u8;32] = unsafe { mem::MaybeUninit::uninit().assume_init() };
    let mut size = mem::MaybeUninit::<size_t>::uninit();
    oci::interval_to_text(
        ctx.ctx_ptr(), ctx.err_ptr(),
        int, lfprec, fsprec,
        name.as_mut_ptr(), name.len(), size.as_mut_ptr()
    )?;
    let size = unsafe { size.assume_init() } as usize;
    let txt = &name[0..size];
    Ok( String::from_utf8_lossy(txt).to_string() )
}

pub(crate) fn to_number(int: *const OCIInterval, ctx: &dyn Ctx) -> Result<OCINumber> {
    let mut num = mem::MaybeUninit::<OCINumber>::uninit();
    oci::interval_to_number(ctx.ctx_ptr(), ctx.err_ptr(), int, num.as_mut_ptr())?;
    Ok( unsafe { num.assume_init() } )
}

pub(crate) fn from_interval<'a,T>(int: &Descriptor<T>, ctx: &'a dyn Ctx) -> Result<Interval<'a,T>>
    where T: DescriptorType<OCIType=OCIInterval>
{
    let interval = Descriptor::new(ctx.env_ptr())?;
    oci::interval_assign(ctx.ctx_ptr(), ctx.err_ptr(), int.get(), interval.get())?;
    Ok( Interval { ctx, interval } )
}

pub struct Interval<'a, T: DescriptorType<OCIType=OCIInterval>> {
    interval: Descriptor<T>,
    ctx: &'a dyn Ctx,
}

impl<'a, T> Interval<'a, T>
    where T: DescriptorType<OCIType=OCIInterval>
{
    /// Returns new uninitialized interval.
    pub fn new(ctx: &'a dyn Ctx) -> Result<Self> {
        let interval = Descriptor::new(ctx.env_ptr())?;
        Ok( Self { ctx, interval } )
    }

    /**
        When given an interval string, returns the interval represented by the string.

        # Example
        ```
        use sibyl::{ self as oracle, IntervalDS };
        let env = oracle::env()?;

        let int = IntervalDS::from_string("3 11:45:28.150000000", &env)?;

        assert_eq!(int.get_duration()?, (3,11,45,28,150000000));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn from_string(txt: &str, ctx: &'a dyn Ctx) -> Result<Self> {
        let interval = Descriptor::new(ctx.env_ptr())?;
        oci::interval_from_text(ctx.ctx_ptr(), ctx.err_ptr(), txt.as_ptr(), txt.len(), interval.get())?;
        Ok( Self { ctx, interval } )
    }

    /**
        Converts an Oracle NUMBER to an interval.

        `num` is in years for YEAR TO MONTH intervals and in days for DAY TO SECOND intervals

        # Example
        ```
        use sibyl::{ self as oracle, IntervalDS, Number };
        let env = oracle::env()?;

        let num = Number::from_real(5.5, &env)?;
        let int = IntervalDS::from_number(&num)?;

        assert_eq!(int.get_duration()?, (5,12,0,0,0));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn from_number(num: &'a Number) -> Result<Self> {
        let interval = Descriptor::new(num.ctx.env_ptr())?;
        oci::interval_from_number(num.ctx.ctx_ptr(), num.ctx.err_ptr(), interval.get(), num.as_ptr())?;
        Ok( Self { ctx: num.ctx, interval } )
    }

    /// Changes an interval context.
    pub fn move_to(&mut self, ctx: &'a dyn Ctx) {
        self.ctx = ctx;
    }

    pub(crate) fn as_ptr(&self) -> *const OCIInterval {
        self.interval.get() as *const OCIInterval
    }

    pub(crate) fn as_mut_ptr(&self) -> *mut OCIInterval {
        self.interval.get() as *mut OCIInterval
    }

    /**
        Copies one interval to another.

        # Example
        ```
        use sibyl::{ self as oracle, IntervalDS };
        let env = oracle::env()?;

        let int = IntervalDS::from_string("3 11:45:28.150000000", &env)?;
        let cpy = IntervalDS::from_interval(&int)?;

        assert_eq!(cpy.get_duration()?, (3,11,45,28,150000000));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn from_interval(other: &Self) -> Result<Self> {
        from_interval(&other.interval, other.ctx)
    }

    /**
        Returns number of years (for YEAR TO MONTH intervals) or days (for DAY TO SECOND intervals)

        Fractional portions of the interval are included in the Oracle NUMBER produced.
        Excess precision is truncated.

        # Example
        ```
        use sibyl::{ self as oracle, IntervalDS };
        let env = oracle::env()?;

        let int = IntervalDS::from_string("3 12:00:00.000000000", &env)?;
        let num = int.to_number()?;

        assert_eq!(num.to_real::<f64>()?, 3.5);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn to_number(&self) -> Result<Number> {
        let mut num = Number::new(self.ctx);
        oci::interval_to_number(self.ctx.ctx_ptr(), self.ctx.err_ptr(), self.as_ptr(), num.as_mut_ptr())?;
        Ok( num )
    }

    /**
        Returns a string representing the interval.

        - `lfprec` is a leading field precision: the number of digits used to represent the leading field.
        - `fsprec` is a fractional second precision of the interval: the number of digits used to represent the fractional seconds.

        The interval literal is output as 'year' or 'year-month' for INTERVAL YEAR TO MONTH intervals
        and as 'seconds' or 'minutes[:seconds]' or 'hours[:minutes[:seconds]]' or 'days[ hours[:minutes[:seconds]]]'
        for INTERVAL DAY TO SECOND intervals (where optional fields are surrounded by brackets)

        # Example
        ```
        use sibyl::{ self as oracle, IntervalDS, Number };
        let env = oracle::env()?;

        let num = Number::from_real(3.1415927, &env)?;
        let int = IntervalDS::from_number(&num)?;

        assert_eq!(int.to_string(1, 3)?, "+3 03:23:53.609");
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn to_string(&self, lfprec: u8, fsprec: u8) -> Result<String> {
        to_string(lfprec, fsprec, self.as_ptr(), self.ctx)
    }

    /**
        Compares two intervals.

        # Example
        ```
        use sibyl::{ self as oracle, IntervalDS };
        use std::cmp::Ordering;
        let env = oracle::env()?;

        let i1 = IntervalDS::from_string("3 12:00:00.000000001", &env)?;
        let i2 = IntervalDS::from_string("3 12:00:00.000000002", &env)?;

        assert_eq!(i1.compare(&i2)?, Ordering::Less);
        assert_eq!(i2.compare(&i1)?, Ordering::Greater);

        let i3 = IntervalDS::from_interval(&i2)?;
        assert_eq!(i2.compare(&i3)?, Ordering::Equal);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn compare(&self, other: &Self) -> Result<Ordering> {
        let mut res = 0i32;
        oci::interval_compare(self.ctx.ctx_ptr(), self.ctx.err_ptr(), self.as_ptr(), other.as_ptr(), &mut res)?;
        let ordering = if res < 0 { Ordering::Less } else if res == 0 { Ordering::Equal } else { Ordering::Greater };
        Ok( ordering )
    }

    /**
        Adds two intervals to produce a resulting interval.

        # Example
        ```
        use sibyl::{ self as oracle, IntervalDS };
        let env = oracle::env()?;

        let i1 = IntervalDS::from_string("+0 02:13:40.000000000", &env)?;
        let i2 = IntervalDS::from_string("+0 00:46:20.000000000", &env)?;
        let res = i1.add(&i2)?;

        assert_eq!(res.get_duration()?, (0,3,0,0,0));

        let i3 = oracle::IntervalDS::from_string("-0 00:13:40.000000000", &env)?;
        let res = i1.add(&i3)?;

        assert_eq!(res.get_duration()?, (0,2,0,0,0));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn add(&self, other: &Self) -> Result<Self> {
        let ctx = self.ctx;
        let interval = Descriptor::new(ctx.env_ptr())?;
        oci::interval_add(ctx.ctx_ptr(), ctx.err_ptr(), self.as_ptr(), other.as_ptr(), interval.get())?;
        Ok( Self { ctx, interval } )
    }

    /**
        Subtracts an interval from this interval and returns the difference as a new interval.

        # Example
        ```
        use sibyl::{ self as oracle, IntervalDS };
        let env = oracle::env()?;

        let i1 = IntervalDS::from_string("+0 02:13:40.000000000", &env)?;
        let i2 = IntervalDS::from_string("+0 01:13:40.000000000", &env)?;
        let res = i1.sub(&i2)?;
        assert_eq!(res.get_duration()?, (0,1,0,0,0));

        let i3 = IntervalDS::from_string("-0 01:46:20.000000000", &env)?;
        let res = i1.sub(&i3)?;
        assert_eq!(res.get_duration()?, (0,4,0,0,0));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn sub(&self, other: &Self) -> Result<Self> {
        let ctx = self.ctx;
        let interval = Descriptor::new(self.ctx.env_ptr())?;
        oci::interval_subtract(ctx.ctx_ptr(), ctx.err_ptr(), self.as_ptr(), other.as_ptr(), interval.get())?;
        Ok( Self { ctx, interval } )
    }

    /**
        Multiplies an interval by an Oracle NUMBER to produce an interval.

        # Example
        ```
        use sibyl::{ self as oracle, IntervalDS, Number };
        let env = oracle::env()?;

        let int = IntervalDS::from_string("+0 00:10:15.000000000", &env)?;
        let num = Number::from_int(4, &env)?;
        let res = int.mul(&num)?;

        assert_eq!(res.get_duration()?, (0,0,41,0,0));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn mul(&self, num: &Number) -> Result<Self> {
        let ctx = self.ctx;
        let interval = Descriptor::new(ctx.env_ptr())?;
        oci::interval_multiply(ctx.ctx_ptr(), ctx.err_ptr(), self.as_ptr(), num.as_ptr(), interval.get())?;
        Ok( Self { ctx, interval } )
    }

    /**
        Divides an interval by an Oracle NUMBER to produce an interval.

        # Example
        ```
        use sibyl::{ self as oracle, IntervalDS, Number };
        let env = oracle::env()?;

        let int = IntervalDS::from_string("+0 00:50:15.000000000", &env)?;
        let num = Number::from_int(5, &env)?;
        let res = int.div(&num)?;

        assert_eq!(res.get_duration()?, (0,0,10,3,0));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn div(&self, num: &Number) -> Result<Self> {
        let ctx = self.ctx;
        let interval = Descriptor::new(ctx.env_ptr())?;
        oci::interval_divide(ctx.ctx_ptr(), ctx.err_ptr(), self.as_ptr(), num.as_ptr(), interval.get())?;
        Ok( Self { ctx, interval } )
    }
}

impl<'a> Interval<'a, OCIIntervalDayToSecond> {
    /**
        Returns interval with the region ID set (if the region is specified
        in the input string) and the current absolute offset, or an absolute
        offset with the region ID set to 0

        The input string must be of the form [+/-]TZH:TZM or 'TZR [TZD]'

        # Example
        ```
        use sibyl::{ self as oracle, IntervalDS };
        let env = oracle::env()?;

        let int = IntervalDS::from_tz("EST", &env)?;

        assert_eq!(int.to_string(1, 1)?, "-0 05:00:00.0");
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn from_tz(txt: &str, ctx: &'a dyn Ctx) -> Result<Self> {
        let interval = Descriptor::new(ctx.env_ptr())?;
        oci::interval_from_tz(ctx.ctx_ptr(), ctx.err_ptr(), txt.as_ptr(), txt.len(), interval.get())?;
        Ok( Self { ctx, interval } )
    }

    /**
        Returns new interval with a preset duration.

        # Example
        ```
        use sibyl::{ self as oracle, IntervalDS };
        let env = oracle::env()?;

        // 3 days, 14 hours, 15 minutes, 26 seconds, 535897932 nanoseconds
        let int = IntervalDS::with_duration(3, 14, 15, 26, 535_897_932, &env)?;

        assert_eq!(int.get_duration()?, (3, 14, 15, 26, 535897932));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn with_duration(dd: i32, hh: i32, mi: i32, ss: i32, ns: i32, ctx: &'a dyn Ctx) -> Result<Self> {
        let interval = Descriptor::new(ctx.env_ptr())?;
        oci::interval_set_day_second(ctx.ctx_ptr(), ctx.err_ptr(), dd, hh, mi, ss, ns, interval.get())?;
        Ok( Self { ctx, interval } )
    }

    /**
        Gets values of day, hour, minute, second, and nanoseconds from an interval.

        # Example
        ```
        use sibyl::{ self as oracle, IntervalDS };
        let env = oracle::env()?;

        let int = IntervalDS::from_tz("EST", &env)?;

        assert_eq!(int.get_duration()?, (0, -5, 0, 0, 0));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn get_duration(&self) -> Result<(i32,i32,i32,i32,i32)> {
        let mut day  = 0i32;
        let mut hour = 0i32;
        let mut min  = 0i32;
        let mut sec  = 0i32;
        let mut fsec = 0i32;
        oci::interval_get_day_second(
            self.ctx.ctx_ptr(), self.ctx.err_ptr(),
            &mut day, &mut hour, &mut min, &mut sec, &mut fsec,
            self.as_ptr()
        )?;
        Ok( (day, hour, min, sec, fsec) )
    }

    /**
        Sets day, hour, minute, second, and nanosecond in an interval.

        # Example
        ```
        use sibyl::{ self as oracle, IntervalDS };
        let env = oracle::env()?;

        let mut int = IntervalDS::with_duration(3, 14, 15, 26, 535_897_932, &env)?;
        assert_eq!(int.get_duration()?, (3, 14, 15, 26, 535897932));

        int.set_duration(0, -5, 0, 0, 0)?;
        assert_eq!(int.get_duration()?, (0, -5, 0, 0, 0));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn set_duration(&mut self, dd: i32, hh: i32, mi: i32, ss: i32, ns: i32) -> Result<()> {
        oci::interval_set_day_second(self.ctx.ctx_ptr(), self.ctx.err_ptr(), dd, hh, mi, ss, ns, self.as_mut_ptr())
    }
}

impl<'a> Interval<'a, OCIIntervalYearToMonth> {
    /**
        Returns new interval with a preset duration.

        # Example
        ```
        use sibyl::{ self as oracle, IntervalYM };
        let env = oracle::env()?;

        // 3 years, 1 month
        let int = IntervalYM::with_duration(3, 1, &env)?;

        assert_eq!(int.get_duration()?, (3, 1));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn with_duration(year: i32, month: i32, ctx: &'a dyn Ctx) -> Result<Self> {
        let interval = Descriptor::new(ctx.env_ptr())?;
        oci::interval_set_year_month(ctx.ctx_ptr(), ctx.err_ptr(), year, month, interval.get())?;
        Ok( Self { ctx, interval } )
    }

    /**
        Gets values of year and month from an interval.

        # Example
        ```
        use sibyl::{ self as oracle, IntervalYM };
        let env = oracle::env()?;

        let int = IntervalYM::with_duration(3, 1, &env)?;
        let (year, month) = int.get_duration()?;

        assert_eq!((year, month), (3, 1));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn get_duration(&self) -> Result<(i32,i32)> {
        let mut year  = 0i32;
        let mut month = 0i32;
        oci::interval_get_year_month(self.ctx.ctx_ptr(), self.ctx.err_ptr(), &mut year, &mut month, self.as_ptr())?;
        Ok( (year, month) )
    }

    /**
        Sets year and month in an interval.

        # Example
        ```
        use sibyl::{ self as oracle, IntervalYM };
        let env = oracle::env()?;

        let mut int = IntervalYM::with_duration(3, 1, &env)?;
        assert_eq!(int.get_duration()?, (3, 1));

        int.set_duration(0, 17)?;
        assert_eq!(int.get_duration()?, (0, 17));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn set_duration(&mut self, year: i32, month: i32) -> Result<()> {
        oci::interval_set_year_month(self.ctx.ctx_ptr(), self.ctx.err_ptr(), year, month, self.as_mut_ptr())
    }
}

impl std::fmt::Debug for Interval<'_, OCIIntervalDayToSecond> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.get_duration() {
            Ok(duration) => fmt.write_fmt(format_args!("IntervalDS {:?}", duration)),
            Err(err)     => fmt.write_fmt(format_args!("IntervalDS {:?}", err)),
        }
    }
}

impl std::fmt::Debug for Interval<'_, OCIIntervalYearToMonth> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.get_duration() {
            Ok(duration) => fmt.write_fmt(format_args!("IntervalDS {:?}", duration)),
            Err(err)     => fmt.write_fmt(format_args!("IntervalDS {:?}", err)),
        }
    }
}
