//! The Oracle time-stamp data types: TIMESTAMP, TIMESTAMP WITH TIME ZONE, TIMESTAMP WITH LOCAL TIME ZONE

mod tosql;

use super::{ Ctx, interval::Interval };
use crate::{ Result, catch, oci::* };
use std::{ mem, ptr, cmp::Ordering };

pub(crate) fn to_string(fmt: &str, fsprec: u8, ts: *const OCIDateTime, ctx: &dyn Ctx) -> Result<String> {
    let mut name: [u8;128] = unsafe { mem::MaybeUninit::uninit().assume_init() };
    let mut size = name.len() as u32;
    catch!{ctx.err_ptr() =>
        OCIDateTimeToText(
            ctx.as_ptr(), ctx.err_ptr(), ts,
            if fmt.len() == 0 { ptr::null() } else { fmt.as_ptr() }, fmt.len() as u8, fsprec,
            ptr::null(), 0,
            &mut size as *mut u32, name.as_mut_ptr()
        )
    }
    let txt = &name[0..size as usize];
    Ok( String::from_utf8_lossy(txt).to_string() )
}

pub(crate) fn from_timestamp<'a,T>(ts: &Descriptor<T>, ctx: &'a dyn Ctx) -> Result<Timestamp<'a, T>>
    where T: DescriptorType<OCIType=OCIDateTime>
{
    let datetime = Descriptor::new(ctx.env_ptr())?;
    catch!{ctx.err_ptr() =>
        OCIDateTimeAssign(
            ctx.as_ptr(), ctx.err_ptr(),
            ts.get(), datetime.get()
        )
    }
    Ok( Timestamp { ctx, datetime } )
}

pub(crate) fn convert_into<'a,T,U>(ts: &Descriptor<T>, ctx: &'a dyn Ctx) -> Result<Timestamp<'a, U>>
    where T: DescriptorType<OCIType=OCIDateTime>
        , U: DescriptorType<OCIType=OCIDateTime>
{
    let datetime: Descriptor<U> = Descriptor::new(ctx.env_ptr())?;
    catch!{ctx.err_ptr() =>
        OCIDateTimeConvert(
            ctx.as_ptr(), ctx.err_ptr(),
            ts.get(), datetime.get()
        )
    }
    Ok( Timestamp { ctx, datetime } )
}

pub struct Timestamp<'e, T>
    where T: DescriptorType<OCIType=OCIDateTime>
{
    datetime: Descriptor<T>,
    ctx: &'e dyn Ctx,
}

impl<'e, T> Timestamp<'e, T>
    where T: DescriptorType<OCIType=OCIDateTime>
{
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
    pub fn new(ctx: &'e dyn Ctx) -> Result<Self> {
        let datetime = Descriptor::new(ctx.env_ptr())?;
        Ok( Self { ctx, datetime } )
    }

    /// Changes a timestamp context.
    pub fn move_to(&mut self, ctx: &'e dyn Ctx) {
        self.ctx = ctx;
    }

    /**
        Creates a timestamp and populates its fields.

        Time zone, as a string, is represented in the format "[+|-][HH:MM]". If the time zone is not
        specified, then the session default time zone is assumed.

        Time zone is ignored for timestamps that do not have one.

        For timestamps with a time zone, the date and time fields are assumed to be in the local time
        of the specified time zone.

        # Example
        ```
        use std::cmp::Ordering;
        use sibyl::{ self as oracle, Timestamp, TimestampTZ, TimestampLTZ };
        let env = oracle::env()?;

        let ts = Timestamp::with_datetime(1969, 7, 20, 20, 18, 4, 0, "", &env)?;
        assert_eq!(ts.get_date()?, (1969, 7, 20));
        assert_eq!(ts.get_time()?, (20, 18, 4,0));

        let res = ts.get_tz_offset();
        assert!(res.is_err());
        match res {
            Err( oracle::Error::Oracle(errcode, _errmsg) ) => assert_eq!(1878, errcode),
            _ => panic!("unexpected error")
        }

        let ts = oracle::TimestampTZ::with_datetime(1969, 7, 20, 20, 18, 4, 0, "UTC", &env)?;
        assert_eq!(ts.get_date()?, (1969, 7, 20));
        assert_eq!(ts.get_time()?, (20, 18, 4,0));
        assert_eq!(ts.get_tz_offset()?, (0,0));

        let ts1 = TimestampLTZ::from_string("1969-7-20 8:18:04 pm", "YYYY-MM-DD HH:MI:SS PM", &env)?;
        // Here it gets a little tricky... The timestamp above is in the local time zone
        // (whatever "local" is on the machine where this code is running).
        // To create the same timestamp using `from_datetime` we need to know that time zone
        let tzn = ts1.get_tz_name()?;
        // And then provide it to the `from_datetime` method
        let ts2 = TimestampLTZ::with_datetime(1969, 7, 20, 20, 18, 4, 0, &tzn, &env)?;
        assert_eq!(ts2.compare(&ts1)?, Ordering::Equal);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn with_datetime(year: i16, month: u8, day: u8, hour: u8, min: u8, sec: u8, fsec: u32, tz: &str, ctx: &'e dyn Ctx) -> Result<Self> {
        let datetime = Descriptor::new(ctx.env_ptr())?;
        catch!{ctx.err_ptr() =>
            OCIDateTimeConstruct(
                ctx.as_ptr(), ctx.err_ptr(), datetime.get(),
                year, month, day, hour, min, sec, fsec, tz.as_ptr(), tz.len()
            )
        }
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
        assert_eq!(ts.get_date()?, (1969,7,20));
        assert_eq!(ts.get_time()?, (20,18,4,160000000));
        assert_eq!(ts.get_tz_offset()?, (0,0));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn from_string(txt: &str, fmt: &str, ctx: &'e dyn Ctx) -> Result<Self> {
        let datetime = Descriptor::new(ctx.env_ptr())?;
        catch!{ctx.err_ptr() =>
            OCIDateTimeFromText(
                ctx.as_ptr(), ctx.err_ptr(),
                txt.as_ptr(), txt.len(), fmt.as_ptr(), fmt.len() as u8,
                ptr::null(), 0,
                datetime.get()
            )
        }
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

        let tzts = TimestampTZ::with_datetime(1969, 7, 24, 16, 50, 35, 0, "UTC", &env)?;

        let ts : Timestamp = tzts.convert_into(&env)?;
        // It just discards the timezone
        assert_eq!(ts.get_date_and_time()?, (1969, 7, 24, 16, 50, 35, 0));

        let lts : TimestampLTZ = tzts.convert_into(&env)?;
        // It just slaps in the local time zone without shifting the time
        assert_eq!(lts.get_date_and_time()?, (1969, 7, 24, 16, 50, 35, 0));

        let (tzh, tzm) = lts.get_tz_offset()?;
        assert_ne!((tzh, tzm), (0, 0));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn convert_into<U>(&self, ctx: &'e dyn Ctx) -> Result<Timestamp<'e, U>>
        where U: DescriptorType<OCIType=OCIDateTime>
    {
        convert_into(&self.datetime, ctx)
    }

    pub(crate) fn as_ptr(&self) -> *const OCIDateTime {
        self.datetime.get() as *const OCIDateTime
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
    pub fn from_timestamp(other: &Self, ctx: &'e dyn Ctx) -> Result<Self> {
        from_timestamp(&other.datetime, ctx)
    }

    /**
        Adds an interval to self and returns the result as a new timestamp.

        # Example
        ```
        use sibyl::{ self as oracle, TimestampTZ, IntervalDS };
        let env = oracle::env()?;

        let ts1 = TimestampTZ::with_datetime(1969,7,20,20,18,4,0,"UTC", &env)?;
        let int = IntervalDS::with_duration(0,21,35,56,0,&env)?;
        let ts2 = ts1.add(&int)?;

        assert_eq!(ts2.to_string("YYYY-MM-DD HH24:MI:SS.FF TZR",1)?, "1969-07-21 17:54:00.0 UTC");
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn add<I: DescriptorType<OCIType=OCIInterval>>(&self, interval: &Interval<I>) -> Result<Self> {
        let ctx = self.ctx;
        let datetime = Descriptor::new(ctx.env_ptr())?;
        catch!{ctx.err_ptr() =>
            OCIDateTimeIntervalAdd(
                ctx.as_ptr(), ctx.err_ptr(),
                self.as_ptr(), interval.as_ptr(),
                datetime.get()
            )
        }
        Ok( Self { ctx, datetime } )
    }

    /**
        Subtracts an interval from self and returns the result as a new timestamp.

        # Example
        ```
        use sibyl::{ self as oracle, TimestampTZ, IntervalDS };
        let env = oracle::env()?;

        let ts1 = TimestampTZ::with_datetime(1969,7,21,17,54,0,0,"UTC", &env)?;
        let int = IntervalDS::with_duration(0,21,35,56,0,&env)?;
        let ts2 = ts1.sub(&int)?;

        assert_eq!(ts2.to_string("YYYY-MM-DD HH24:MI:SS.FF TZR",1)?, "1969-07-20 20:18:04.0 UTC");
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn sub<I: DescriptorType<OCIType=OCIInterval>>(&self, interval: &Interval<I>) -> Result<Self> {
        let ctx = self.ctx;
        let datetime = Descriptor::new(ctx.env_ptr())?;
        catch!{ctx.err_ptr() =>
            OCIDateTimeIntervalSub(
                ctx.as_ptr(), ctx.err_ptr(),
                self.as_ptr(), interval.as_ptr(),
                datetime.get()
            )
        }
        Ok( Self { ctx, datetime } )
    }

    /**
        Returns the differnce between self and the `other` timestamp as an interval.

        # Example
        ```
        use sibyl::{ self as oracle, TimestampTZ, IntervalDS };
        let env = oracle::env()?;

        let ts1 = TimestampTZ::with_datetime(1969,7,20,20,18,4,0,"UTC", &env)?;
        let ts2 = TimestampTZ::with_datetime(1969,7,21,17,54,0,0,"UTC", &env)?;
        let int: IntervalDS = ts2.subtract(&ts1)?;
        let (days, hours, min, sec, nanosec) = int.get_duration()?;

        assert_eq!((days, hours, min, sec, nanosec), (0, 21, 35, 56, 0));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn subtract<U, I>(&self, other: &Timestamp<U>) -> Result<Interval<I>>
        where U: DescriptorType<OCIType=OCIDateTime>
            , I: DescriptorType<OCIType=OCIInterval>
    {
        let ctx = self.ctx;
        let interval: Interval<I> = Interval::new(self.ctx)?;
        catch!{ctx.err_ptr() =>
            OCIDateTimeSubtract(
                ctx.as_ptr(), ctx.err_ptr(),
                self.as_ptr(), other.as_ptr(),
                interval.as_mut_ptr()
            )
        }
        Ok( interval )
    }

    /**
        Compares this timestamp with the `other` one.

        # Example
        ```
        use std::cmp::Ordering;
        use sibyl::{ self as oracle, TimestampTZ };
        let env = oracle::env()?;

        let ts1 = TimestampTZ::with_datetime(1969,7,20,20,18,4,0,"+00:00", &env)?;
        let ts2 = TimestampTZ::with_datetime(1969,7,20,16,18,4,0,"-04:00", &env)?;

        assert_eq!(ts2.compare(&ts1)?, Ordering::Equal);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn compare<U>(&self, other: &Timestamp<U>) -> Result<Ordering>
        where U: DescriptorType<OCIType=OCIDateTime>
    {
        let mut cmp = 0i32;
        catch!{self.ctx.err_ptr() =>
            OCIDateTimeCompare(
                self.ctx.as_ptr(), self.ctx.err_ptr(),
                self.as_ptr(), other.as_ptr(),
                &mut cmp
            )
        }
        let ordering = if cmp < 0 { Ordering::Less } else if cmp == 0 { Ordering::Equal } else { Ordering::Greater };
        Ok( ordering )
    }

    /**
        Returns the date (year, month, day) portion of a timestamp.

        # Example
        ```
        use sibyl::{ self as oracle, Timestamp };
        let env = oracle::env()?;

        let ts = Timestamp::with_datetime(1969,7,20,20,18,4,0,"", &env)?;

        assert_eq!(ts.get_date()?, (1969, 7, 20));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn get_date(&self) -> Result<(i16, u8, u8)> {
        let mut year  = 0i16;
        let mut month = 0u8;
        let mut day   = 0u8;
        catch!{self.ctx.err_ptr() =>
            OCIDateTimeGetDate(
                self.ctx.as_ptr(), self.ctx.err_ptr(), self.as_ptr(),
                &mut year, &mut month, &mut day
            )
        }
        Ok( (year, month, day) )
    }

    /**
        Returns the time (hour, min, second, nanosecond) of a timestamp.

        # Example
        ```
        use sibyl::{ self as oracle, Timestamp };
        let env = oracle::env()?;

        let ts = Timestamp::with_datetime(1969,7,20,20,18,4,0,"", &env)?;

        assert_eq!(ts.get_time()?, (20, 18, 4, 0));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn get_time(&self) -> Result<(u8, u8, u8, u32)> {
        let mut hour = 0u8;
        let mut min  = 0u8;
        let mut sec  = 0u8;
        let mut fsec = 0u32;
        catch!{self.ctx.err_ptr() =>
            OCIDateTimeGetTime(
                self.ctx.as_ptr(), self.ctx.err_ptr(), self.as_ptr(),
                &mut hour, &mut min, &mut sec, &mut fsec
            )
        }
        Ok( (hour, min, sec, fsec) )
    }

    /**
        Returns the date and the time (year, month, day, hour, min, second, fractional second)
        of a timestamp.

        # Example
        ```
        use sibyl::{ self as oracle, Timestamp };
        let env = oracle::env()?;

        let ts = Timestamp::with_datetime(1969,7,20,20,18,4,0,"", &env)?;

        assert_eq!(ts.get_date_and_time()?, (1969, 7, 20, 20, 18, 4, 0));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn get_date_and_time(&self) -> Result<(i16, u8, u8, u8, u8, u8, u32)> {
        let (year, month, day) = self.get_date()?;
        let (hour, min, sec, nanos) = self.get_time()?;
        Ok((year, month, day, hour, min, sec, nanos))
    }

    /**
        Returns the time zone name portion of a timestamp

        # Example
        ```
        use sibyl::{ self as oracle, TimestampTZ };
        let env = oracle::env()?;

        let ts = TimestampTZ::from_string("July 20, 1969 8:18:04.16 pm UTC", "MONTH DD, YYYY HH:MI:SS.FF PM TZR", &env)?;
        assert_eq!(ts.get_tz_name()?, "UTC");

        let ts = TimestampTZ::with_datetime(1969,7,20,20,18,4,0,"+00:00", &env)?;
        assert_eq!(ts.get_tz_name()?, "+00:00");

        let ts = TimestampTZ::with_datetime(1969,7,20,20,18,4,0,"EST", &env)?;
        assert_eq!(ts.get_tz_name()?, "EST");
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn get_tz_name(&self) -> Result<String> {
        let mut name: [u8;64] = unsafe { mem::MaybeUninit::uninit().assume_init() };
        let mut size = name.len() as u32;
        catch!{self.ctx.err_ptr() =>
            OCIDateTimeGetTimeZoneName(
                self.ctx.as_ptr(), self.ctx.err_ptr(), self.as_ptr(),
                name.as_mut_ptr(), &mut size
            )
        }
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
        let (tzh, tzm) = ts.get_tz_offset()?;

        assert_eq!((tzh, tzm), (0,0));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn get_tz_offset(&self) -> Result<(i8, i8)> {
        let mut hours = 0i8;
        let mut min   = 0i8;
        catch!{self.ctx.err_ptr() =>
            OCIDateTimeGetTimeZoneOffset(
                self.ctx.as_ptr(), self.ctx.err_ptr(), self.as_ptr(),
                &mut hours, &mut min
            )
        }
        Ok( (hours, min) )
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

        let ts = TimestampTZ::with_datetime(1969,7,20,20,18,4,0, "UTC", &env)?;
        let txt = ts.to_string("Dy, Mon DD, YYYY HH:MI:SS.FF PM TZR", 3)?;

        assert_eq!(txt, "Sun, Jul 20, 1969 08:18:04.000 PM UTC");
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn to_string(&self, fmt: &str, fsprec: u8) -> Result<String> {
        to_string(fmt, fsprec, self.as_ptr(), self.ctx)
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
impl<'e> Timestamp<'e, OCITimestampTZ> {
    /**
        Creates new timestamp from the system current date and time.

        # Example
        ```
        use sibyl::{ self as oracle, TimestampTZ };
        let env = oracle::env()?;

        let ts = TimestampTZ::from_systimestamp(&env)?;
        let (year, _month, _day) = ts.get_date()?;

        assert!(year >= 2021);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn from_systimestamp(ctx: &'e dyn Ctx) -> Result<Self> {
        let datetime = Descriptor::new(ctx.env_ptr())?;
        catch!{ctx.err_ptr() =>
            OCIDateTimeSysTimeStamp(ctx.as_ptr(), ctx.err_ptr(), datetime.get())
        }
        Ok( Self { ctx, datetime } )
    }
}

impl std::fmt::Debug for Timestamp<'_, OCITimestampTZ> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.to_string("YYYY-DD-MM HH24:MI:SSXFF TZR", 3) {
            Ok(txt)  => fmt.write_fmt(format_args!("TimestampTZ({})", txt)),
            Err(err) => fmt.write_fmt(format_args!("TimestampTZ({})", err))
        }
    }
}

impl std::fmt::Debug for Timestamp<'_, OCITimestampLTZ> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.to_string("YYYY-DD-MM HH24:MI:SSXFF TZR", 3) {
            Ok(txt)  => fmt.write_fmt(format_args!("TimestampLTZ({})", txt)),
            Err(err) => fmt.write_fmt(format_args!("TimestampLTZ({})", err))
        }
    }
}

impl std::fmt::Debug for Timestamp<'_, OCITimestamp> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.to_string("YYYY-DD-MM HH24:MI:SSXFF", 3) {
            Ok(txt)  => fmt.write_fmt(format_args!("Timestamp({})", txt)),
            Err(err) => fmt.write_fmt(format_args!("Timestamp({})", err))
        }
    }
}
