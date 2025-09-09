//! The Oracle time-stamp data types: TIMESTAMP, TIMESTAMP WITH TIME ZONE, TIMESTAMP WITH LOCAL TIME ZONE

mod tosql;

use super::{ Ctx, interval::Interval };
use crate::{ Result, oci::{self, *} };
use std::{ mem, ptr, cmp::Ordering, ops::{Deref, DerefMut} };

pub(crate) fn to_string(fmt: &str, fsprec: u8, ts: &OCIDateTime, ctx: &dyn Ctx) -> Result<String> {
    let name = mem::MaybeUninit::<[u8;128]>::uninit();
    let mut name = unsafe { name.assume_init() };
    let mut size = name.len() as u32;
    oci::date_time_to_text(
        ctx.as_context(), ctx.as_ref(), ts,
        if fmt.len() == 0 { ptr::null() } else { fmt.as_ptr() }, fmt.len() as u8, fsprec,
        &mut size as *mut u32, name.as_mut_ptr()
    )?;
    let txt = &name[0..size as usize];
    Ok( String::from_utf8_lossy(txt).to_string() )
}

pub(crate) fn from_timestamp<'a,T>(ts: &Descriptor<T>, ctx: &'a dyn Ctx) -> Result<DateTime<'a, T>>
where T: DescriptorType<OCIType=OCIDateTime>
{
    let mut datetime = Descriptor::<T>::new(&ctx)?;
    oci::date_time_assign(ctx.as_context(), ctx.as_ref(), ts, datetime.as_mut())?;
    Ok( DateTime { ctx, datetime } )
}

pub(crate) fn convert_into<'a,T,U>(ts: &Descriptor<T>, ctx: &'a dyn Ctx) -> Result<DateTime<'a, U>>
where T: DescriptorType<OCIType=OCIDateTime>
    , U: DescriptorType<OCIType=OCIDateTime>
{
    let mut datetime: Descriptor<U> = Descriptor::new(&ctx)?;
    oci::date_time_convert(ctx.as_context(), ctx.as_ref(), ts.as_ref(), datetime.as_mut())?;
    Ok( DateTime { ctx, datetime } )
}

/// Represents datetime data types.
pub struct DateTime<'a, T> where T: DescriptorType<OCIType=OCIDateTime> {
    datetime: Descriptor<T>,
    ctx: &'a dyn Ctx,
}

impl<'a,T> AsRef<T::OCIType> for DateTime<'a,T> where T: DescriptorType<OCIType=OCIDateTime> {
    fn as_ref(&self) -> &T::OCIType {
        self.datetime.as_ref()
    }
}

impl<'a,T> AsMut<T::OCIType> for DateTime<'a,T> where T: DescriptorType<OCIType=OCIDateTime> {
    fn as_mut(&mut self) -> &mut T::OCIType {
        self.datetime.as_mut()
    }
}

impl<'a,T> Deref for DateTime<'a,T> where T: DescriptorType<OCIType=OCIDateTime> {
    type Target = T::OCIType;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<'a,T> DerefMut for DateTime<'a,T> where T: DescriptorType<OCIType=OCIDateTime> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl<'a, T> DateTime<'a, T> where T: DescriptorType<OCIType=OCIDateTime> {
    /**
        Creates an uninitialized timestamp.

        # Example
        ```
        use sibyl::{ self as oracle, Timestamp };
        let env = oracle::env()?;

        let _ts = Timestamp::new(&env)?;
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn new(ctx: &'a dyn Ctx) -> Result<Self> {
        let datetime = Descriptor::<T>::new(&ctx)?;
        Ok( Self { ctx, datetime } )
    }

    /**
        Creates a timestamp and populates its fields.

        Time zone, as a string, is represented in the format "\[+|-\]\[HH:MM\]". If the time zone is not
        specified, then the session default time zone is assumed.

        Time zone is ignored for timestamps that do not have one.

        For timestamps with a time zone, the date and time fields are assumed to be in the local time
        of the specified time zone.

        # Example
        ```
        use std::cmp::Ordering;
        use sibyl::{ self as oracle, Timestamp, TimestampTZ, TimestampLTZ };
        let env = oracle::env()?;

        let ts = Timestamp::with_date_and_time(1969, 7, 20, 20, 18, 4, 0, "", &env)?;
        assert_eq!(ts.date()?, (1969, 7, 20));
        assert_eq!(ts.time()?, (20, 18, 4,0));

        let res = ts.tz_offset();
        assert!(res.is_err());
        match res {
            Err( oracle::Error::Oracle(errcode, _errmsg) ) => assert_eq!(1878, errcode),
            _ => panic!("unexpected error")
        }

        let ts = oracle::TimestampTZ::with_date_and_time(1969, 7, 20, 20, 18, 4, 0, "UTC", &env)?;
        assert_eq!(ts.date()?, (1969, 7, 20));
        assert_eq!(ts.time()?, (20, 18, 4,0));
        assert_eq!(ts.tz_offset()?, (0,0));

        let ts1 = TimestampLTZ::from_string("1969-7-20 8:18:04 pm", "YYYY-MM-DD HH:MI:SS PM", &env)?;
        // Here it gets a little tricky... The timestamp above is in the local time zone
        // (whatever "local" is on the machine where this code is running).
        // To create the same timestamp using `from_datetime` we need to know that time zone
        let tzn = ts1.tz_name()?;
        // And then provide it to the `from_datetime` method
        let ts2 = TimestampLTZ::with_date_and_time(1969, 7, 20, 20, 18, 4, 0, &tzn, &env)?;
        assert_eq!(ts2.compare(&ts1)?, Ordering::Equal);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn with_date_and_time(year: i16, month: u8, day: u8, hour: u8, min: u8, sec: u8, fsec: u32, tz: &str, ctx: &'a dyn Ctx) -> Result<Self> {
        let mut datetime = Descriptor::<T>::new(&ctx)?;
        oci::date_time_construct(
            ctx.as_context(), ctx.as_ref(), &mut datetime,
            year, month, day, hour, min, sec, fsec, tz.as_ptr(), tz.len()
        )?;
        Ok( Self { ctx, datetime } )
    }

    /**
        Creates new timestamp from the given string according to the specified format.

        If the timestamp is in the user session, the conversion occurs in the session's NLS_LANGUAGE and
        the session's NLS_CALENDAR; otherwise, the default is used.

        Refer to Oracle [Format Models](https://docs.oracle.com/en/database/oracle/oracle-database/19/sqlrf/Format-Models.html)
        for the description of format.

        # Example
        ```
        use sibyl::{ self as oracle, TimestampTZ };
        let env = oracle::env()?;

        let ts = TimestampTZ::from_string("July 20, 1969 8:18:04.16 pm UTC", "MONTH DD, YYYY HH:MI:SS.FF PM TZR", &env)?;
        assert_eq!(ts.date()?, (1969,7,20));
        assert_eq!(ts.time()?, (20,18,4,160000000));
        assert_eq!(ts.tz_offset()?, (0,0));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn from_string(txt: &str, fmt: &str, ctx: &'a dyn Ctx) -> Result<Self> {
        let mut datetime = Descriptor::<T>::new(&ctx)?;
        oci::date_time_from_text(
            ctx.as_context(), ctx.as_ref(),
            txt.as_ptr(), txt.len(), fmt.as_ptr(), fmt.len() as u8,
            ptr::null(), 0,
            &mut datetime
        )?;
        Ok( Self { ctx, datetime } )
    }

    /**
        Converts this datetime type to another.

        The session default time zone (ORA_SDTZ) is used when converting a datetime
        without a time zone to one with a time zone.

        # Example
        ```
        use sibyl::{ self as oracle, Timestamp, TimestampTZ, TimestampLTZ };
        let env = oracle::env()?;

        let tzts = TimestampTZ::with_date_and_time(1969, 7, 24, 16, 50, 35, 0, "UTC", &env)?;

        let ts : Timestamp = tzts.convert_into(&env)?;
        // It just discards the timezone
        assert_eq!(ts.date_and_time()?, (1969, 7, 24, 16, 50, 35, 0));

        let lts : TimestampLTZ = tzts.convert_into(&env)?;
        // It just slaps in the local time zone without shifting the time
        assert_eq!(lts.date_and_time()?, (1969, 7, 24, 16, 50, 35, 0));

        let (tzh, tzm) = lts.tz_offset()?;
        assert_ne!((tzh, tzm), (0, 0));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn convert_into<U>(&self, ctx: &'a dyn Ctx) -> Result<DateTime<'a, U>>
    where U: DescriptorType<OCIType=OCIDateTime>
    {
        convert_into(&self.datetime, ctx)
    }

    /**
        Creates a copy of the other timestamp

        # Example
        ```
        use std::cmp::Ordering;
        use sibyl::{ self as oracle, TimestampTZ };
        let env = oracle::env()?;

        let ts1 = TimestampTZ::from_systimestamp(&env)?;
        let ts2 = TimestampTZ::from_timestamp(&ts1, &env)?;

        assert_eq!(ts2.compare(&ts1)?, Ordering::Equal);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn from_timestamp(other: &Self, ctx: &'a dyn Ctx) -> Result<Self> {
        from_timestamp(&other.datetime, ctx)
    }

    /**
        Adds an interval to self and returns the result as a new timestamp.

        # Example
        ```
        use sibyl::{ self as oracle, TimestampTZ, IntervalDS };
        let env = oracle::env()?;

        let ts1 = TimestampTZ::with_date_and_time(1969,7,20,20,18,4,0,"UTC", &env)?;
        let int = IntervalDS::with_duration(0,21,35,56,0,&env)?;
        let ts2 = ts1.add(&int)?;

        assert_eq!(ts2.to_string("YYYY-MM-DD HH24:MI:SS.FF TZR",1)?, "1969-07-21 17:54:00.0 UTC");
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn add<I: DescriptorType<OCIType=OCIInterval>>(&self, interval: &Interval<I>) -> Result<Self> {
        let ctx = self.ctx;
        let mut datetime = Descriptor::<T>::new(&ctx)?;
        oci::date_time_interval_add(ctx.as_context(), ctx.as_ref(), &self.datetime, interval, &mut datetime)?;
        Ok( Self { ctx, datetime } )
    }

    /**
        Subtracts an interval from self and returns the result as a new timestamp.

        # Example
        ```
        use sibyl::{ self as oracle, TimestampTZ, IntervalDS };
        let env = oracle::env()?;

        let ts1 = TimestampTZ::with_date_and_time(1969,7,21,17,54,0,0,"UTC", &env)?;
        let int = IntervalDS::with_duration(0,21,35,56,0,&env)?;
        let ts2 = ts1.sub(&int)?;

        assert_eq!(ts2.to_string("YYYY-MM-DD HH24:MI:SS.FF TZR",1)?, "1969-07-20 20:18:04.0 UTC");
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn sub<I: DescriptorType<OCIType=OCIInterval>>(&self, interval: &Interval<I>) -> Result<Self> {
        let ctx = self.ctx;
        let mut datetime = Descriptor::<T>::new(&ctx)?;
        oci::date_time_interval_sub(ctx.as_context(), ctx.as_ref(), &self.datetime, interval, &mut datetime)?;
        Ok( Self { ctx, datetime } )
    }

    /**
        Returns the differnce between self and the `other` timestamp as an interval.

        # Example
        ```
        use sibyl::{ self as oracle, TimestampTZ, IntervalDS };
        let env = oracle::env()?;

        let ts1 = TimestampTZ::with_date_and_time(1969,7,20,20,18,4,0,"UTC", &env)?;
        let ts2 = TimestampTZ::with_date_and_time(1969,7,21,17,54,0,0,"UTC", &env)?;
        let int: IntervalDS = ts2.subtract(&ts1)?;
        let (days, hours, min, sec, nanosec) = int.duration()?;

        assert_eq!((days, hours, min, sec, nanosec), (0, 21, 35, 56, 0));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn subtract<U, I>(&self, other: &DateTime<U>) -> Result<Interval<'_,I>>
    where U: DescriptorType<OCIType=OCIDateTime>
        , I: DescriptorType<OCIType=OCIInterval>
    {
        let ctx = self.ctx;
        let mut interval: Interval<I> = Interval::new(self.ctx)?;
        oci::date_time_subtract(ctx.as_context(), ctx.as_ref(), &self.datetime, &other.datetime, &mut interval)?;
        Ok( interval )
    }

    /**
        Compares this timestamp with the `other` one.

        # Example
        ```
        use std::cmp::Ordering;
        use sibyl::{ self as oracle, TimestampTZ };
        let env = oracle::env()?;

        let ts1 = TimestampTZ::with_date_and_time(1969,7,20,20,18,4,0,"+00:00", &env)?;
        let ts2 = TimestampTZ::with_date_and_time(1969,7,20,16,18,4,0,"-04:00", &env)?;

        assert_eq!(ts2.compare(&ts1)?, Ordering::Equal);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn compare<U>(&self, other: &DateTime<U>) -> Result<Ordering>
    where U: DescriptorType<OCIType=OCIDateTime>
    {
        let mut cmp = 0i32;
        oci::date_time_compare(self.ctx.as_context(), self.ctx.as_ref(), self, other, &mut cmp)?;
        let ordering = if cmp < 0 { Ordering::Less } else if cmp == 0 { Ordering::Equal } else { Ordering::Greater };
        Ok( ordering )
    }

    /**
        Returns the date (year, month, day) portion of a timestamp.

        # Example
        ```
        use sibyl::{ self as oracle, Timestamp };
        let env = oracle::env()?;

        let ts = Timestamp::with_date_and_time(1969,7,20,20,18,4,0,"", &env)?;

        assert_eq!(ts.date()?, (1969, 7, 20));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn date(&self) -> Result<(i16, u8, u8)> {
        let mut year  = oci::Aligned::new(0i16);
        let mut month = oci::Aligned::new(0u8);
        let mut day   = oci::Aligned::new(0u8);
        oci::date_time_get_date(self.ctx.as_context(), self.ctx.as_ref(), self, year.as_mut_ptr(), month.as_mut_ptr(), day.as_mut_ptr())?;
        Ok( (year.into(), month.into(), day.into()) )
    }

    /**
        Returns the time (hour, min, second, nanosecond) of a timestamp.

        # Example
        ```
        use sibyl::{ self as oracle, Timestamp };
        let env = oracle::env()?;

        let ts = Timestamp::with_date_and_time(1969,7,20,20,18,4,0,"", &env)?;

        assert_eq!(ts.time()?, (20, 18, 4, 0));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn time(&self) -> Result<(u8, u8, u8, u32)> {
        let mut hour = oci::Aligned::new(0u8);
        let mut min  = oci::Aligned::new(0u8);
        let mut sec  = oci::Aligned::new(0u8);
        let mut fsec = 0u32;
        oci::date_time_get_time(self.ctx.as_context(), self.ctx.as_ref(), self, hour.as_mut_ptr(), min.as_mut_ptr(), sec.as_mut_ptr(), &mut fsec)?;
        Ok( (hour.into(), min.into(), sec.into(), fsec) )
    }

    /**
        Returns the date and the time (year, month, day, hour, min, second, fractional second)
        of a timestamp.

        # Example
        ```
        use sibyl::{ self as oracle, Timestamp };
        let env = oracle::env()?;

        let ts = Timestamp::with_date_and_time(1969,7,20,20,18,4,0,"", &env)?;

        assert_eq!(ts.date_and_time()?, (1969, 7, 20, 20, 18, 4, 0));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn date_and_time(&self) -> Result<(i16, u8, u8, u8, u8, u8, u32)> {
        let (year, month, day) = self.date()?;
        let (hour, min, sec, nanos) = self.time()?;
        Ok((year, month, day, hour, min, sec, nanos))
    }

    /**
        Returns the time zone name portion of a timestamp

        # Example
        ```
        use sibyl::{ self as oracle, TimestampTZ };
        let env = oracle::env()?;

        let ts = TimestampTZ::from_string("July 20, 1969 8:18:04.16 pm UTC", "MONTH DD, YYYY HH:MI:SS.FF PM TZR", &env)?;
        assert_eq!(ts.tz_name()?, "UTC");

        let ts = TimestampTZ::with_date_and_time(1969,7,20,20,18,4,0,"+00:00", &env)?;
        assert_eq!(ts.tz_name()?, "+00:00");

        let ts = TimestampTZ::with_date_and_time(1969,7,20,20,18,4,0,"EST", &env)?;
        assert_eq!(ts.tz_name()?, "EST");
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn tz_name(&self) -> Result<String> {
        let name = mem::MaybeUninit::<[u8;64]>::uninit();
        let mut name = unsafe { name.assume_init() };
        let mut size = name.len() as u32;
        oci::date_time_get_time_zone_name(self.ctx.as_context(), self.ctx.as_ref(), self, name.as_mut_ptr(), &mut size)?;
        let txt = &name[0..size as usize];
        Ok( String::from_utf8_lossy(txt).to_string() )
    }

    /**
        Returns the time zone hour and the time zone minute portion from a timestamp.

        # Example
        ```
        use sibyl::{ self as oracle, TimestampTZ };
        let env = oracle::env()?;

        let ts = TimestampTZ::from_string("July 20, 1969 8:18:04.16 pm UTC", "MONTH DD, YYYY HH:MI:SS.FF PM TZR", &env)?;
        let (tzh, tzm) = ts.tz_offset()?;

        assert_eq!((tzh, tzm), (0,0));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn tz_offset(&self) -> Result<(i8, i8)> {
        let mut hours = 0i32;
        let mut min   = 0i32;
        oci::date_time_get_time_zone_offset(self.ctx.as_context(), self.ctx.as_ref(), self, &mut hours as *mut i32 as _, &mut min as *mut i32 as _)?;
        Ok( (hours as _, min as _) )
    }

    /**
        Converts the given date to a string according to the specified format.

        If timestamp originated in the user session, the conversion occurs in
        the session's NLS_LANGUAGE and the session's NLS_CALENDAR; otherwise,
        the default is used.

        If the conversion format is an empty (zero-length) string, then the date is converted to
        a character string in the default format for that type.

        Refer to Oracle [Format Models](https://docs.oracle.com/en/database/oracle/oracle-database/19/sqlrf/Format-Models.html)
        for the description of format.

        # Example
        ```
        use sibyl::{ self as oracle, TimestampTZ };
        let env = oracle::env()?;

        let ts = TimestampTZ::with_date_and_time(1969,7,20,20,18,4,0, "UTC", &env)?;
        let txt = ts.to_string("Dy, Mon DD, YYYY HH:MI:SS.FF PM TZR", 3)?;

        assert_eq!(txt, "Sun, Jul 20, 1969 08:18:04.000 PM UTC");
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn to_string(&self, fmt: &str, fsprec: u8) -> Result<String> {
        to_string(fmt, fsprec, self, self.ctx)
    }
}

// For some reason timestamps created by OCIDateTimeSysTimeStamp always have a time zone
// even when their descriptors are OCI_DTYPE_TIMESTAMP. That leads to:
// * "ORA-01483: invalid length for DATE or NUMBER bind variable" when they are inserted
//   into the appropriate TS columns or
// * "ORA-00932: inconsistent datatypes: expected %s got %s" when a clone is created via
//   OCIDateTimeAssign or
// * "ORA-01870: the intervals or datetimes are not mutually comparable" when 2 timestamps
//   of the same type, one of which was created via OCIDateTimeSysTimeStamp, are compared.
// For now, let's just limit `from_systimestamp` to `TimestampTZ` where it produces the expected
// results.
impl<'a> DateTime<'a, OCITimestampTZ> {
    /**
        Creates new timestamp from the system current date and time.

        # Example
        ```
        use sibyl::{ self as oracle, TimestampTZ };
        let env = oracle::env()?;

        let ts = TimestampTZ::from_systimestamp(&env)?;
        let (year, _month, _day) = ts.date()?;

        assert!(year >= 2021);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn from_systimestamp(ctx: &'a dyn Ctx) -> Result<Self> {
        let mut datetime = Descriptor::<OCITimestampTZ>::new(&ctx)?;
        oci::date_time_sys_time_stamp(ctx.as_context(), ctx.as_ref(), &mut datetime)?;
        Ok( Self { ctx, datetime } )
    }
}

impl std::fmt::Debug for DateTime<'_, OCITimestampTZ> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.to_string("YYYY-DD-MM HH24:MI:SSXFF TZR", 3) {
            Ok(txt)  => fmt.write_fmt(format_args!("TimestampTZ({})", txt)),
            Err(err) => fmt.write_fmt(format_args!("TimestampTZ({})", err))
        }
    }
}

impl std::fmt::Debug for DateTime<'_, OCITimestampLTZ> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.to_string("YYYY-DD-MM HH24:MI:SSXFF TZR", 3) {
            Ok(txt)  => fmt.write_fmt(format_args!("TimestampLTZ({})", txt)),
            Err(err) => fmt.write_fmt(format_args!("TimestampLTZ({})", err))
        }
    }
}

impl std::fmt::Debug for DateTime<'_, OCITimestamp> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.to_string("YYYY-DD-MM HH24:MI:SSXFF", 3) {
            Ok(txt)  => fmt.write_fmt(format_args!("Timestamp({})", txt)),
            Err(err) => fmt.write_fmt(format_args!("Timestamp({})", err))
        }
    }
}
