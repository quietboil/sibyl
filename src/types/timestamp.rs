//! The Oracle time-stamp data types: TIMESTAMP, TIMESTAMP WITH TIME ZONE, TIMESTAMP WITH LOCAL TIME ZONE

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
    let mut name: [u8;128];
    let mut size: u32;
    catch!{usrenv.err_ptr() =>
        name = mem::uninitialized();
        size = name.len() as u32;
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

    /// Creates an uninitialized timestamp.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let _ts = oracle::Timestamp::new(&env)?;
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn new(usrenv: &'e dyn UsrEnv) -> Result<Self> {
        let datetime = Descriptor::new(usrenv.env_ptr())?;
        Ok( Self { usrenv, datetime } )
    }

    /// Changes a timestamp context.
    pub fn move_to(&mut self, usrenv: &'e dyn UsrEnv) {
        self.usrenv = usrenv;
    }

    /// Creates a timestamp and populates its fields.
    ///
    /// Time zone, a string, is represented in the format "[+|-][HH:MM]". If the time zone is not
    /// specified, then the session default time zone is assumed.
    ///
    /// Time zone is ignored for timestamps that do not have one.
    ///
    /// For timestamps with a time zone, the date and time fields are assumed to be in the local time
    /// of the specified time zone.
    ///
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let ts = oracle::Timestamp::from_datetime(1969,7,20,20,18,4,000000160,"", &env)?;
    /// let (y,m,d) = ts.get_date()?;
    /// assert_eq!((1969,7,20), (y,m,d));
    ///
    /// let (h,m,s,f) = ts.get_time()?;
    /// assert_eq!((20,18,4,160), (h,m,s,f));
    ///
    /// let res = ts.get_tz_offset();
    /// assert!(res.is_err());
    /// if let Err( oracle::Error::Oracle((errcode, _errmsg)) ) = res {
    ///     assert_eq!(1878, errcode); // field not found
    /// }
    ///
    /// let ts = oracle::TimestampTZ::from_datetime(1969,7,20,20,18,4,000000160,"+00:00", &env)?;
    /// let (y,m,d) = ts.get_date()?;
    /// assert_eq!((1969,7,20), (y,m,d));
    ///
    /// let (h,m,s,f) = ts.get_time()?;
    /// assert_eq!((20,18,4,160), (h,m,s,f));
    ///
    /// let (tzh,tzm) = ts.get_tz_offset()?;
    /// assert_eq!((0,0), (tzh,tzm));
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn from_datetime(year: i16, month: u8, day: u8, hour: u8, min: u8, sec: u8, fsec: u32, tz: &str, usrenv: &'e dyn UsrEnv) -> Result<Self> {
        let datetime = Descriptor::new(usrenv.env_ptr())?;
        catch!{usrenv.err_ptr() =>
            OCIDateTimeConstruct(
                usrenv.as_ptr(), usrenv.err_ptr(), datetime.get(),
                year, month, day, hour, min, sec, fsec, tz.as_ptr(), tz.len()
            )
        }
        Ok( Self { usrenv, datetime } )
    }

    /// Creates new timestamp from the given string according to the specified format.
    ///
    /// If the timestamp is in the user session, the conversion occurs in the session's NLS_LANGUAGE and
    /// the session's NLS_CALENDAR; otherwise, the default is used.
    ///
    /// See the description of the TO_DATE conversion function for a description of the format argument.
    ///
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let ts = oracle::TimestampTZ::from_string("July 20, 1969 8:18:04.16 pm UTC", "MONTH DD, YYYY HH:MI:SS.FF PM TZR", &env)?;
    /// let (y,m,d) = ts.get_date()?;
    /// assert_eq!((1969,7,20), (y,m,d));
    ///
    /// let (h,m,s,f) = ts.get_time()?;
    /// assert_eq!((20,18,4,160000000), (h,m,s,f));
    ///
    /// let (tzh,tzm) = ts.get_tz_offset()?;
    /// assert_eq!((0,0), (tzh,tzm));
    /// # Ok::<(),oracle::Error>(())
    /// ```
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

    /// This function converts this datetime type to another.
    /// The session default time zone (ORA_SDTZ) is used when converting a datetime
    /// without a time zone to one with a time zone.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let ts = oracle::TimestampTZ::from_datetime(1969,7,20,20,18,4,0,"+00:00", &env)?;
    /// let ts: oracle::Timestamp = ts.convert_into(&env)?;
    ///
    /// let (y,m,d) = ts.get_date()?;
    /// assert_eq!((1969,7), (y,m));
    ///
    /// let (h,m,s,f) = ts.get_time()?;
    /// assert_eq!((18,4,0), (m,s,f));
    ///
    /// let lts = oracle::TimestampTZ::from_systimestamp(&env)?;
    /// let (tzh,_tzm) = lts.get_tz_offset()?;
    ///
    /// let (ld, lh) = if tzh < 4 { (20,20+tzh) } else { (21,tzh-4) };
    /// assert_eq!(d, ld);
    /// assert_eq!(h, ld);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    ///
    pub fn convert_into<U>(&self, usrenv: &'e dyn UsrEnv) -> Result<Timestamp<'e, U>>
        where U: DescriptorType<OCIType=OCIDateTime>
    {
        convert_into(&self.datetime, usrenv)
    }

    pub(crate) fn as_ptr(&self) -> *const OCIDateTime {
        self.datetime.get() as *const OCIDateTime
    }

    /// Creates a copy of the other timestamp
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let ts1 = oracle::TimestampTZ::from_systimestamp(&env)?;
    /// let ts2 = oracle::TimestampTZ::from_timestamp(&ts1, &env)?;
    /// let cmp = ts2.compare(&ts1)?;
    /// assert_eq!(std::cmp::Ordering::Equal, cmp);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn from_timestamp(other: &Self, usrenv: &'e dyn UsrEnv) -> Result<Self> {
        from_timestamp(&other.datetime, usrenv)
    }

    /// Adds an interval to self and returns the result as a new timestamp.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let ts1 = oracle::TimestampTZ::from_datetime(1969,7,20,20,18,4,0,"UTC", &env)?;
    /// let int = oracle::IntervalDS::from_duration(0,21,35,56,0,&env)?;
    /// let ts2 = ts1.add(&int)?;
    /// let txt = ts2.to_string("YYYY-MM-DD HH24:MI:SS.FF TZR",1)?;
    ///
    /// assert_eq!("1969-07-21 17:54:00.0 UTC", txt);
    /// # Ok::<(),oracle::Error>(())
    /// ```
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

    /// Subtracts an interval from self and returns the result as a new timestamp.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let ts2 = oracle::TimestampTZ::from_datetime(1969,7,21,17,54,0,0,"UTC", &env)?;
    /// let int = oracle::IntervalDS::from_duration(0,21,35,56,0,&env)?;
    /// let ts1 = ts2.sub(&int)?;
    /// let txt = ts1.to_string("YYYY-MM-DD HH24:MI:SS.FF TZR",1)?;
    ///
    /// assert_eq!("1969-07-20 20:18:04.0 UTC", txt);
    /// # Ok::<(),oracle::Error>(())
    /// ```
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

    /// Returns the differnce between self and the `other` timestamp as an interval.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let ts1 = oracle::TimestampTZ::from_datetime(1969,7,20,20,18,4,0,"UTC", &env)?;
    /// let ts2 = oracle::TimestampTZ::from_datetime(1969,7,21,17,54,0,0,"UTC", &env)?;
    /// let int: oracle::IntervalDS = ts2.subtract(&ts1)?;
    /// let (d,h,m,s,n) = int.get_duration()?;
    ///
    /// assert_eq!((0,21,35,56,0), (d,h,m,s,n));
    /// # Ok::<(),oracle::Error>(())
    /// ```
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

    /// Compares this timestamp with the `other` one.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let ts1 = oracle::TimestampTZ::from_datetime(1969,7,20,20,18,4,0,"+00:00", &env)?;
    /// let ts2 = oracle::TimestampTZ::from_datetime(1969,7,20,16,18,4,0,"-04:00", &env)?;
    /// let cmp = ts2.compare(&ts1)?;
    ///
    /// assert_eq!(std::cmp::Ordering::Equal, cmp);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn compare<U>(&self, other: &Timestamp<U>) -> Result<Ordering>
        where U: DescriptorType<OCIType=OCIDateTime>
    {
        let mut res: i32;
        catch!{self.usrenv.err_ptr() =>
            res = mem::uninitialized();
            OCIDateTimeCompare(
                self.usrenv.as_ptr(), self.usrenv.err_ptr(),
                self.as_ptr(), other.as_ptr(),
                &mut res
            )
        }
        let ordering = if res < 0 { Ordering::Less } else if res == 0 { Ordering::Equal } else { Ordering::Greater };
        Ok( ordering )
    }

    /// Returns the date (year, month, day) portion of a timestamp
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let ts = oracle::Timestamp::from_datetime(1969,7,20,20,18,4,0,"", &env)?;
    /// let (y,m,d) = ts.get_date()?;
    ///
    /// assert_eq!((1969,7,20), (y,m,d));
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn get_date(&self) -> Result<(i16, u8, u8)> {
        let mut year: i16;
        let mut month: u8;
        let mut day:   u8;
        catch!{self.usrenv.err_ptr() =>
            year  = mem::uninitialized();
            month = mem::uninitialized();
            day   = mem::uninitialized();
            OCIDateTimeGetDate(
                self.usrenv.as_ptr(), self.usrenv.err_ptr(), self.as_ptr(),
                &mut year, &mut month, &mut day
            )
        }
        Ok((year, month, day))
    }

    /// Returns the time (hour, min, second, fractional second) of a timestamp
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let ts = oracle::Timestamp::from_datetime(1969,7,20,20,18,4,0,"", &env)?;
    /// let (h,m,s,f) = ts.get_time()?;
    ///
    /// assert_eq!((20,18,4,0), (h,m,s,f));
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn get_time(&self) -> Result<(u8, u8, u8, u32)> {
        let mut hour: u8;
        let mut min:  u8;
        let mut sec:  u8;
        let mut fsec: u32;
        catch!{self.usrenv.err_ptr() =>
            hour = mem::uninitialized();
            min  = mem::uninitialized();
            sec  = mem::uninitialized();
            fsec = mem::uninitialized();
            OCIDateTimeGetTime(
                self.usrenv.as_ptr(), self.usrenv.err_ptr(), self.as_ptr(),
                &mut hour, &mut min, &mut sec, &mut fsec
            )
        }
        Ok((hour, min, sec, fsec))
    }

    /// Returns the time zone name portion of a timestamp
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let ts = oracle::TimestampTZ::from_string("July 20, 1969 8:18:04.16 pm UTC", "MONTH DD, YYYY HH:MI:SS.FF PM TZR", &env)?;
    /// let tz = ts.get_tz_name()?;
    /// assert_eq!("UTC", tz);
    ///
    /// let ts = oracle::TimestampTZ::from_datetime(1969,7,20,20,18,4,0,"+00:00", &env)?;
    /// let tz = ts.get_tz_name()?;
    /// assert_eq!("+00:00", tz);
    ///
    /// let ts = oracle::TimestampTZ::from_datetime(1969,7,20,20,18,4,0,"EST", &env)?;
    /// let tz = ts.get_tz_name()?;
    /// assert_eq!("EST", tz);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn get_tz_name(&self) -> Result<String> {
        let mut name: [u8;64];
        let mut size: u32;
        catch!{self.usrenv.err_ptr() =>
            name = mem::uninitialized();
            size = name.len() as u32;
            OCIDateTimeGetTimeZoneName(
                self.usrenv.as_ptr(), self.usrenv.err_ptr(), self.as_ptr(),
                name.as_mut_ptr(), &mut size
            )
        }
        let txt = &name[0..size as usize];
        Ok( String::from_utf8_lossy(txt).to_string() )
    }

    /// Returns the time zone hour and the time zone minute portion from a timestamp.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let ts = oracle::TimestampTZ::from_string("July 20, 1969 8:18:04.16 pm UTC", "MONTH DD, YYYY HH:MI:SS.FF PM TZR", &env)?;
    /// let (tzh,tzm) = ts.get_tz_offset()?;
    ///
    /// assert_eq!((0,0), (tzh,tzm));
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn get_tz_offset(&self) -> Result<(i8, i8)> {
        let mut hours: i8;
        let mut min:   i8;
        catch!{self.usrenv.err_ptr() =>
            hours = mem::uninitialized();
            min   = mem::uninitialized();
            OCIDateTimeGetTimeZoneOffset(
                self.usrenv.as_ptr(), self.usrenv.err_ptr(), self.as_ptr(),
                &mut hours, &mut min
            )
        }
        Ok((hours, min))
    }

    /// Converts the given date to a string according to the specified format.
    ///
    /// If timestamp is at the user session, the conversion occurs in the session's NLS_LANGUAGE
    /// and the session's NLS_CALENDAR; otherwise, the default is used.
    ///
    /// If the conversion format is an empty (zero-length) string, then the date is converted to
    /// a character string in the default format for that type.
    ///
    /// See the description of the TO_DATE conversion function for a description of the format argument.
    ///
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let ts = oracle::TimestampTZ::from_datetime(1969,7,20,20,18,4,0,"UTC", &env)?;
    /// let txt = ts.to_string("Dy, Mon DD, YYYY HH:MI:SS.FF PM TZR",3)?;
    ///
    /// assert_eq!("Sun, Jul 20, 1969 08:18:04.000 PM UTC", txt);
    /// # Ok::<(),oracle::Error>(())
    /// ```
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

    /// Creates new timestamp from the system current date and time.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let ts = oracle::TimestampTZ::from_systimestamp(&env)?;
    /// let (y,_m,_d) = ts.get_date()?;
    ///
    /// assert!(2019 <= y);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn from_systimestamp(usrenv: &'e dyn UsrEnv) -> Result<Self> {
        let datetime = Descriptor::new(usrenv.env_ptr())?;
        catch!{usrenv.err_ptr() =>
            OCIDateTimeSysTimeStamp(usrenv.as_ptr(), usrenv.err_ptr(), datetime.get())
        }
        Ok( Self { usrenv, datetime } )
    }
}

macro_rules! impl_ts_to_sql {
    ($ts:ty => $sqlt:ident) => {
        impl ToSql for Timestamp<'_, $ts> {
            fn to_sql(&self) -> (u16, *const c_void, usize) {
                ( $sqlt, self.datetime.as_ptr() as *const c_void, std::mem::size_of::<*mut OCIDateTime>() )
            }
        }
    };
}

impl_ts_to_sql!{ OCITimestamp    => SQLT_TIMESTAMP     }
impl_ts_to_sql!{ OCITimestampTZ  => SQLT_TIMESTAMP_TZ  }
impl_ts_to_sql!{ OCITimestampLTZ => SQLT_TIMESTAMP_LTZ }

macro_rules! impl_ts_to_sql_output {
    ($ts:ty => $sqlt:ident) => {
        impl ToSqlOut for Descriptor<$ts> {
            fn to_sql_output(&mut self, _col_size: usize) -> (u16, *mut c_void, usize) {
                ($sqlt, self.as_ptr() as *mut c_void, std::mem::size_of::<*mut OCIDateTime>())
            }
        }
        impl ToSqlOut for Timestamp<'_, $ts> {
            fn to_sql_output(&mut self, col_size: usize) -> (u16, *mut c_void, usize) {
                self.datetime.to_sql_output(col_size)
            }
        }
    };
}

impl_ts_to_sql_output!{ OCITimestamp    => SQLT_TIMESTAMP     }
impl_ts_to_sql_output!{ OCITimestampTZ  => SQLT_TIMESTAMP_TZ  }
impl_ts_to_sql_output!{ OCITimestampLTZ => SQLT_TIMESTAMP_LTZ }
