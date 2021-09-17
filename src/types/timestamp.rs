//! The Oracle time-stamp data types: TIMESTAMP, TIMESTAMP WITH TIME ZONE, TIMESTAMP WITH LOCAL TIME ZONE

mod tosql;

use crate::*;
use crate::desc::{ Descriptor, DescriptorType };
use super::*;
use super::interval::Interval;
use libc::{ c_void, size_t };
use std::{ mem, ptr, cmp::Ordering };

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-3B02C8CC-F35C-422F-B35C-47765C998E57
    fn OCIDateTimeAssign (
        hndl:       *mut c_void,
        err:        *mut OCIError,
        from:       *const OCIDateTime,
        to:         *mut OCIDateTime
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-5C2A63E3-85EC-4346-A636-33B9B4CCBA41
    // fn OCIDateTimeCheck (
    //     hndl:       *mut c_void,
    //     err:        *mut OCIError,
    //     date:       *const OCIDateTime,
    //     result:     *mut u32
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-5FFD4B08-30E1-461E-8E55-940787D6D8EC
    fn OCIDateTimeCompare (
        hndl:       *mut c_void,
        err:        *mut OCIError,
        date1:      *const OCIDateTime,
        date2:      *const OCIDateTime,
        result:     *mut i32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-FC053036-BE93-42D7-A82C-4DDB6843E167
    fn OCIDateTimeConstruct (
        hndl:       *mut c_void,
        err:        *mut OCIError,
        datetime:   *mut OCIDateTime,
        year:       i16,
        month:      u8,
        day:        u8,
        hour:       u8,
        min:        u8,
        sec:        u8,
        fsec:       u32,
        timezone:   *const u8,
        tz_len:     size_t
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-744793B2-CD2F-47AC-825A-6FF5BEE12BAB
    fn OCIDateTimeConvert (
        hndl:       *mut c_void,
        err:        *mut OCIError,
        indate:     *const OCIDateTime,
        outdate:    *mut OCIDateTime
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-16189076-75E9-4B46-B418-89CD8DDB42EA
    // fn OCIDateTimeFromArray(
    //     hndl:       *mut c_void,
    //     err:        *mut OCIError,
    //     inarray:    *const u8,
    //     len:        u32,
    //     dt_type:    u8,
    //     datetime:   *mut OCIDateTime,
    //     reftz:      *const OCIInterval,
    //     fsprec:     u8
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-1A453A79-4EEF-462D-B4B3-45820F9EEA4C
    fn OCIDateTimeFromText(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        date_str:   *const u8,
        dstr_length: size_t,
        fmt:        *const u8,
        fmt_length: u8,
        lang_name:  *const u8,
        lang_length: size_t,
        datetime:   *mut OCIDateTime,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-FE6F9482-913D-43FD-BE5A-FCD9FA7B83AD
    fn OCIDateTimeGetDate(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        datetime:   *const OCIDateTime,
        year:       *mut i16,
        month:      *mut u8,
        day:        *mut u8,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-D935ABA2-DEEA-4ABA-AA9C-C27E3E5AC1FD
    fn OCIDateTimeGetTime(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        datetime:   *const OCIDateTime,
        hour:       *mut u8,
        min:        *mut u8,
        sec:        *mut u8,
        fsec:       *mut u32,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-489C51F6-43DB-43DB-980F-2A42AFAFB332
    fn OCIDateTimeGetTimeZoneName(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        datetime:   *const OCIDateTime,
        buf:        *mut u8,
        buflen:     *mut u32,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-B8DA860B-FD7D-481B-8347-156969B6EE04
    fn OCIDateTimeGetTimeZoneOffset(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        datetime:   *const OCIDateTime,
        hour:       *mut i8,
        min:        *mut i8,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-810C6FB3-9B81-4A7C-9B5B-5D2D93B781FA
    fn OCIDateTimeIntervalAdd(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        datetime:   *const OCIDateTime,
        inter:      *const OCIInterval,
        outdatetime: *mut OCIDateTime,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-DEDBFEF5-52DD-4036-93FE-C21B6ED4E8A5
    fn OCIDateTimeIntervalSub(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        datetime:   *const OCIDateTime,
        inter:      *const OCIInterval,
        outdatetime: *mut OCIDateTime,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-BD2F6432-81FF-4CD6-9C3D-85E401894528
    fn OCIDateTimeSubtract(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        indate1:    *const OCIDateTime,
        indate2:    *const OCIDateTime,
        inter:      *mut OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-086776F8-1153-417D-ABC6-A864A2A62788
    fn OCIDateTimeSysTimeStamp(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        sys_date:   *mut OCIDateTime,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-DCA1CF9E-AF92-42E1-B784-8BFC0C9FF8BE
    // fn OCIDateTimeToArray(
    //     hndl:       *mut c_void,
    //     err:        *mut OCIError,
    //     datetime:   *const OCIDateTime,
    //     reftz:      *const OCIInterval,
    //     outarray:   *mut u8,
    //     len:        *mut u32,
    //     fsprec:     u8
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-828401C8-8E88-4C53-A66A-24901CCF93C6
    fn OCIDateTimeToText(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        date:       *const OCIDateTime,
        fmt:        *const u8,
        fmt_length: u8,
        fsprec:     u8,
        lang_name:  *const u8,
        lang_length: size_t,
        buf_size:   *mut u32,
        buf:        *mut u8,
    ) -> i32;
}

pub(crate) fn to_string(fmt: &str, fsprec: u8, ts: *const OCIDateTime, usrenv: &dyn UsrEnv) -> Result<String> {
    let mut name: [u8;128] = unsafe { mem::MaybeUninit::uninit().assume_init() };
    let mut size = name.len() as u32;
    catch!{usrenv.err_ptr() =>
        OCIDateTimeToText(
            usrenv.as_ptr(), usrenv.err_ptr(), ts,
            if fmt.len() == 0 { ptr::null() } else { fmt.as_ptr() }, fmt.len() as u8, fsprec,
            ptr::null(), 0,
            &mut size as *mut u32, name.as_mut_ptr()
        )
    }
    let txt = &name[0..size as usize];
    Ok( String::from_utf8_lossy(txt).to_string() )
}

pub(crate) fn from_timestamp<'a,T>(ts: &Descriptor<T>, usrenv: &'a dyn UsrEnv) -> Result<Timestamp<'a, T>>
    where T: DescriptorType<OCIType=OCIDateTime>
{
    let datetime = Descriptor::new(usrenv.env_ptr())?;
    catch!{usrenv.err_ptr() =>
        OCIDateTimeAssign(
            usrenv.as_ptr(), usrenv.err_ptr(),
            ts.get(), datetime.get()
        )
    }
    Ok( Timestamp { usrenv, datetime } )
}

pub(crate) fn convert_into<'a,T,U>(ts: &Descriptor<T>, usrenv: &'a dyn UsrEnv) -> Result<Timestamp<'a, U>>
    where T: DescriptorType<OCIType=OCIDateTime>
        , U: DescriptorType<OCIType=OCIDateTime>
{
    let datetime: Descriptor<U> = Descriptor::new(usrenv.env_ptr())?;
    catch!{usrenv.err_ptr() =>
        OCIDateTimeConvert(
            usrenv.as_ptr(), usrenv.err_ptr(),
            ts.get(), datetime.get()
        )
    }
    Ok( Timestamp { usrenv, datetime } )
}

pub struct Timestamp<'e, T>
    where T: DescriptorType<OCIType=OCIDateTime>
{
    datetime: Descriptor<T>,
    usrenv: &'e dyn UsrEnv,
}

impl<'e, T> Timestamp<'e, T>
    where T: DescriptorType<OCIType=OCIDateTime>
{
    pub fn get_type(&self) -> u32 {
        self.datetime.get_type()
    }

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
    pub fn new(usrenv: &'e dyn UsrEnv) -> Result<Self> {
        let datetime = Descriptor::new(usrenv.env_ptr())?;
        Ok( Self { usrenv, datetime } )
    }

    /// Changes a timestamp context.
    pub fn move_to(&mut self, usrenv: &'e dyn UsrEnv) {
        self.usrenv = usrenv;
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
    pub fn with_datetime(year: i16, month: u8, day: u8, hour: u8, min: u8, sec: u8, fsec: u32, tz: &str, usrenv: &'e dyn UsrEnv) -> Result<Self> {
        let datetime = Descriptor::new(usrenv.env_ptr())?;
        catch!{usrenv.err_ptr() =>
            OCIDateTimeConstruct(
                usrenv.as_ptr(), usrenv.err_ptr(), datetime.get(),
                year, month, day, hour, min, sec, fsec, tz.as_ptr(), tz.len()
            )
        }
        Ok( Self { usrenv, datetime } )
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
    pub fn from_string(txt: &str, fmt: &str, usrenv: &'e dyn UsrEnv) -> Result<Self> {
        let datetime = Descriptor::new(usrenv.env_ptr())?;
        catch!{usrenv.err_ptr() =>
            OCIDateTimeFromText(
                usrenv.as_ptr(), usrenv.err_ptr(),
                txt.as_ptr(), txt.len(), fmt.as_ptr(), fmt.len() as u8,
                ptr::null(), 0,
                datetime.get()
            )
        }
        Ok( Self { usrenv, datetime } )
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
    pub fn convert_into<U>(&self, usrenv: &'e dyn UsrEnv) -> Result<Timestamp<'e, U>>
        where U: DescriptorType<OCIType=OCIDateTime>
    {
        convert_into(&self.datetime, usrenv)
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
    pub fn from_timestamp(other: &Self, usrenv: &'e dyn UsrEnv) -> Result<Self> {
        from_timestamp(&other.datetime, usrenv)
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
        let usrenv = self.usrenv;
        let datetime = Descriptor::new(usrenv.env_ptr())?;
        catch!{usrenv.err_ptr() =>
            OCIDateTimeIntervalAdd(
                usrenv.as_ptr(), usrenv.err_ptr(),
                self.as_ptr(), interval.as_ptr(),
                datetime.get()
            )
        }
        Ok( Self { usrenv, datetime } )
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
        let usrenv = self.usrenv;
        let datetime = Descriptor::new(usrenv.env_ptr())?;
        catch!{usrenv.err_ptr() =>
            OCIDateTimeIntervalSub(
                usrenv.as_ptr(), usrenv.err_ptr(),
                self.as_ptr(), interval.as_ptr(),
                datetime.get()
            )
        }
        Ok( Self { usrenv, datetime } )
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
        let usrenv = self.usrenv;
        let interval: Interval<I> = Interval::new(self.usrenv)?;
        catch!{usrenv.err_ptr() =>
            OCIDateTimeSubtract(
                usrenv.as_ptr(), usrenv.err_ptr(),
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
        let mut res = mem::MaybeUninit::<i32>::uninit();
        catch!{self.usrenv.err_ptr() =>
            OCIDateTimeCompare(
                self.usrenv.as_ptr(), self.usrenv.err_ptr(),
                self.as_ptr(), other.as_ptr(),
                res.as_mut_ptr()
            )
        }
        let res = unsafe { res.assume_init() };
        let ordering = if res < 0 { Ordering::Less } else if res == 0 { Ordering::Equal } else { Ordering::Greater };
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
        let mut year  = mem::MaybeUninit::<i16>::uninit();
        let mut month = mem::MaybeUninit::<u8>::uninit();
        let mut day   = mem::MaybeUninit::<u8>::uninit();
        catch!{self.usrenv.err_ptr() =>
            OCIDateTimeGetDate(
                self.usrenv.as_ptr(), self.usrenv.err_ptr(), self.as_ptr(),
                year.as_mut_ptr(), month.as_mut_ptr(), day.as_mut_ptr()
            )
        }
        Ok( unsafe { (year.assume_init(), month.assume_init(), day.assume_init()) } )
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
        let mut hour = mem::MaybeUninit::<u8>::uninit();
        let mut min  = mem::MaybeUninit::<u8>::uninit();
        let mut sec  = mem::MaybeUninit::<u8>::uninit();
        let mut fsec = mem::MaybeUninit::<u32>::uninit();
        catch!{self.usrenv.err_ptr() =>
            OCIDateTimeGetTime(
                self.usrenv.as_ptr(), self.usrenv.err_ptr(), self.as_ptr(),
                hour.as_mut_ptr(), min.as_mut_ptr(), sec.as_mut_ptr(), fsec.as_mut_ptr()
            )
        }
        Ok( unsafe { (hour.assume_init(), min.assume_init(), sec.assume_init(), fsec.assume_init()) } )
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
        catch!{self.usrenv.err_ptr() =>
            OCIDateTimeGetTimeZoneName(
                self.usrenv.as_ptr(), self.usrenv.err_ptr(), self.as_ptr(),
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
        let mut hours = mem::MaybeUninit::<i8>::uninit();
        let mut min   = mem::MaybeUninit::<i8>::uninit();
        catch!{self.usrenv.err_ptr() =>
            OCIDateTimeGetTimeZoneOffset(
                self.usrenv.as_ptr(), self.usrenv.err_ptr(), self.as_ptr(),
                hours.as_mut_ptr(), min.as_mut_ptr()
            )
        }
        Ok( unsafe { (hours.assume_init(), min.assume_init()) } )
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
        to_string(fmt, fsprec, self.as_ptr(), self.usrenv)
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
    pub fn from_systimestamp(usrenv: &'e dyn UsrEnv) -> Result<Self> {
        let datetime = Descriptor::new(usrenv.env_ptr())?;
        catch!{usrenv.err_ptr() =>
            OCIDateTimeSysTimeStamp(usrenv.as_ptr(), usrenv.err_ptr(), datetime.get())
        }
        Ok( Self { usrenv, datetime } )
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
