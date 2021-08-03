//! The Oracle DATE which represents the year, month, day, hour, minute, and second of the date.

use crate::*;
use super::*;
use libc::c_void;
use std::{ mem, ptr, cmp::Ordering };

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-E0890180-8714-4243-A585-0FD21EB05CA9
    fn OCIDateAddDays(
        err:        *mut OCIError,
        date:       *const OCIDate,
        num_days:   i32,
        result:     *mut OCIDate
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-CE37ECF1-622A-49A9-A9FD-40E1BD67C941
    fn OCIDateAddMonths(
        err:        *mut OCIError,
        date:       *const OCIDate,
        num_months: i32,
        result:     *mut OCIDate
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-2251373B-4F7B-4680-BB90-F9013216465A
    fn OCIDateAssign(
        err:        *mut OCIError,
        date:       *const OCIDate,
        result:     *mut OCIDate
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-067F7EB4-419B-4A5B-B1C4-B4C650B874A3
    // fn OCIDateCheck(
    //     err:        *mut OCIError,
    //     date:       *const OCIDate,
    //     result:     *mut u32
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-282C5B79-64AA-4B34-BFC6-292144B1AD16
    fn OCIDateCompare(
        err:        *mut OCIError,
        date1:      *const OCIDate,
        date2:      *const OCIDate,
        result:     *mut i32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-42422C47-805F-4EAA-BF44-E6DE6164082E
    fn OCIDateDaysBetween(
        err:        *mut OCIError,
        date1:      *const OCIDate,
        date2:      *const OCIDate,
        result:     *mut i32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-EA8FEB07-401C-477E-805B-CC9E89FB13F4
    fn OCIDateFromText(
        err:        *mut OCIError,
        txt:        *const u8,
        txt_len:    u32,
        fmt:        *const u8,
        fmt_len:    u8,
        lang:       *const u8,
        lang_len:   u32,
        result:     *mut OCIDate
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-14FB323E-BAEB-4FC7-81DA-6AF243C0D7D6
    fn OCIDateLastDay(
        err:        *mut OCIError,
        date:       *const OCIDate,
        result:     *mut OCIDate
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-A16AB88E-A3BF-4B50-8FEF-6427926198F4
    fn OCIDateNextDay(
        err:        *mut OCIError,
        date:       *const OCIDate,
        day:        *const u8,
        day_len:    u32,
        result:     *mut OCIDate
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-123DD789-48A2-4AD7-8B1E-5E454DFE3F1E
    fn OCIDateToText(
        err:        *mut OCIError,
        date:       *const OCIDate,
        fmt:        *const u8,
        fmt_len:    u8,
        lang:       *const u8,
        lang_len:   u32,
        buf_size:   *mut u32,
        buf:        *mut u8
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-751D4F33-E593-4845-9D5E-8761A19BD243
    fn OCIDateSysDate(
        err:        *mut OCIError,
        result:     *mut OCIDate
    ) -> i32;
}

/// C mapping of the Oracle DATE type (SQLT_ODT)
#[repr(C)]
pub struct OCIDate {
    year: i16, // gregorian year: range is -4712 <= year <= 9999
    month: u8, // month: range is 1 <= month <= 12
    day:   u8, // day: range is 1 <= day <= 31
    hour:  u8, // hours: range is 0 <= hours <= 23
    min:   u8, // minutes: range is 0 <= minutes <= 59
    sec:   u8  // seconds: range is 0 <= seconds <= 59
}

pub(crate) fn new() -> OCIDate {
    let date = mem::MaybeUninit::<OCIDate>::uninit();
    // Return unitinialized to be used as a row's column buffer
    unsafe { date.assume_init() }
}

pub(crate) fn to_string(fmt: &str, date: *const OCIDate, err: *mut OCIError) -> Result<String> {
    let mut txt : [u8;128] = unsafe { mem::MaybeUninit::uninit().assume_init() };
    let mut txt_len = txt.len() as u32;
    catch!{err =>
        OCIDateToText(
            err, date,
            fmt.as_ptr(), fmt.len() as u8,
            ptr::null(), 0,
            &mut txt_len, txt.as_mut_ptr()
        )
    }
    let txt = &txt[0..txt_len as usize];
    Ok( String::from_utf8_lossy(txt).to_string() )
}

pub(crate) fn from_date<'a>(from: &OCIDate, env: &'a dyn UsrEnv) -> Result<Date<'a>> {
    let mut date = mem::MaybeUninit::<OCIDate>::uninit();
    catch!{env.err_ptr() =>
        OCIDateAssign(env.err_ptr(), from as *const OCIDate, date.as_mut_ptr())
    }
    Ok( Date { env, date: unsafe { date.assume_init() } } )
}


/// Represents Oracle DATE
pub struct Date<'e> {
    env: &'e dyn UsrEnv,
    date: OCIDate,
}

impl<'e> Date<'e> {
    /// Constructs new date
    pub fn new(year: i16, month: u8, day: u8, env: &'e dyn UsrEnv) -> Self {
        Self { env, date: OCIDate { year, month, day, hour: 0, min: 0, sec: 0 } }
    }

    /// Constructs new date with time
    pub fn with_time(year: i16, month: u8, day: u8, hour: u8, min: u8, sec: u8, env: &'e dyn UsrEnv) -> Self {
        Self { env, date: OCIDate { year, month, day, hour, min, sec } }
    }

    /**
        Converts a character string to a date type according to the specified format.

        ## Example
        ```
        use sibyl as oracle;

        let env = oracle::env()?;
        let date = oracle::Date::from_string("July 4, 1776", "MONTH DD, YYYY", &env)?;
        let (y, m, d) = date.get_date();

        assert_eq!(1776, y);
        assert_eq!(   7, m);
        assert_eq!(   4, d);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn from_string(txt: &str, fmt: &str, env: &'e dyn UsrEnv) -> Result<Self> {
        let mut date = mem::MaybeUninit::<OCIDate>::uninit();
        catch!{env.err_ptr() =>
            OCIDateFromText(
                env.err_ptr(),
                txt.as_ptr(), txt.len() as u32,
                fmt.as_ptr(), fmt.len() as u8,
                ptr::null(), 0,
                date.as_mut_ptr()
            )
        }
        Ok( Self { env, date: unsafe { date.assume_init() } } )
    }

    /**
        Constructs new date from the client's system clock.

        ## Example
        ```
        use sibyl as oracle;

        let env = oracle::env()?;
        let date = oracle::Date::from_sysdate(&env)?;
        let (y, _m, _d) = date.get_date();

        assert!(2019 <= y);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn from_sysdate(env: &'e dyn UsrEnv) -> Result<Self> {
        let mut date = mem::MaybeUninit::<OCIDate>::uninit();
        catch!{env.err_ptr() =>
            OCIDateSysDate(env.err_ptr(), date.as_mut_ptr())
        }
        Ok( Self { env, date: unsafe { date.assume_init() } } )
    }

    /// Performs a date assignment
    pub fn from_date(from: &Date, env: &'e dyn UsrEnv) -> Result<Self> {
        from_date(&from.date, env)
    }

    pub(crate) fn as_ptr(&self) -> *const OCIDate {
        &self.date
    }

    pub(crate) fn as_mut_ptr(&mut self) -> *mut OCIDate {
        &mut self.date
    }

    /// Gets the year, month, and day stored in an Oracle date.
    pub fn get_date(&self) -> (i16, u8, u8) {
        (self.date.year, self.date.month, self.date.day)
    }

    /// Changes the date.
    pub fn set_date(&mut self, year: i16, month: u8, day: u8) {
        self.date.year  = year;
        self.date.month = month;
        self.date.day   = day;
    }

    /// Gets the time stored in an Oracle date
    pub fn get_time(&self) -> (u8, u8, u8) {
        (self.date.hour, self.date.min, self.date.sec)
    }

    /// Changes the time
    pub fn set_time(&mut self, hour: u8, min: u8, sec: u8) {
        self.date.hour = hour;
        self.date.min  = min;
        self.date.sec  = sec;
    }

    /**
        Returns a string according to the specified format.

        Refer to Oracle [Format Models](https://docs.oracle.com/en/database/oracle/oracle-database/19/sqlrf/Format-Models.html)
        for a description of format.

        ## Example
        ```
        use sibyl as oracle;

        let env = oracle::env()?;
        let date = oracle::Date::new(-1952, 2, 25, &env);
        let res = date.to_string("FMMonth DD, YYYY BC")?;

        assert_eq!("February 25, 1952 BC", res);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn to_string(&self, fmt: &str) -> Result<String> {
        to_string(fmt, self.as_ptr(), self.env.err_ptr())
    }

    /**
        Adds or subtracts days from this date

        ## Example
        ```
        use sibyl as oracle;

        let env = oracle::env()?;
        let start = oracle::Date::new(1969, 7, 16, &env);
        let end = start.add_days(8)?;
        let (y,m,d) = end.get_date();

        assert_eq!(1969, y);
        assert_eq!(   7, m);
        assert_eq!(  24, d);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn add_days(&self, num: i32) -> Result<Date> {
        let env = self.env;
        let mut date = mem::MaybeUninit::<OCIDate>::uninit();
        catch!{env.err_ptr() =>
            OCIDateAddDays(env.err_ptr(), self.as_ptr(), num, date.as_mut_ptr())
        }
        Ok( Self { env, date: unsafe { date.assume_init() } } )
    }

    /**
        Adds or subtracts months from this date.

        If the input date is the last day of a month, then the appropriate adjustments
        are made to ensure that the output date is also the last day of the month.
        For example, Feb. 28 + 1 month = March 31, and November 30 – 3 months = August 31.
        Otherwise the result date has the same day component as date.

        ## Example
        ```
        use sibyl as oracle;

        let env = oracle::env()?;
        let date = oracle::Date::new(2019, 12, 31, &env);
        let date = date.add_months(2)?;
        let (y,m,d) = date.get_date();

        assert_eq!(2020, y);
        assert_eq!(   2, m);
        assert_eq!(  29, d);

        let date = date.add_months(2)?;
        let (y,m,d) = date.get_date();

        assert_eq!(2020, y);
        assert_eq!(   4, m);
        assert_eq!(  30, d);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn add_months(&self, num: i32) -> Result<Date> {
        let env = self.env;
        let mut date = mem::MaybeUninit::<OCIDate>::uninit();
        catch!{env.err_ptr() =>
            OCIDateAddMonths(env.err_ptr(), self.as_ptr(), num, date.as_mut_ptr())
        }
        Ok( Self { env, date: unsafe { date.assume_init() } } )
    }

    /// Compares this date with the `other` date.
    pub fn compare(&self, other: &Date) -> Result<Ordering> {
        let mut res = mem::MaybeUninit::<i32>::uninit();
        catch!{self.env.err_ptr() =>
            OCIDateCompare(self.env.err_ptr(), self.as_ptr(), other.as_ptr(), res.as_mut_ptr())
        }
        let res = unsafe { res.assume_init() };
        let ordering = if res < 0 { Ordering::Less } else if res == 0 { Ordering::Equal } else { Ordering::Greater };
        Ok( ordering )
    }

    /**
        Gets the number of days between two dates.

        When the number of days between date1 and date2 is computed, the time is ignored.

        ## Example
        ```
        use sibyl as oracle;

        let env = oracle::env()?;
        let pearl_harbor = oracle::Date::new(1941, 12, 7, &env);
        let normandy_landings = oracle::Date::new(1944, 6, 6, &env);
        let days_between = normandy_landings.days_from(&pearl_harbor)?;

        assert_eq!(912, days_between);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn days_from(&self, other: &Date) -> Result<i32> {
        let mut res = mem::MaybeUninit::<i32>::uninit();
        catch!{self.env.err_ptr() =>
            OCIDateDaysBetween(self.env.err_ptr(), self.as_ptr(), other.as_ptr(), res.as_mut_ptr())
        }
        Ok( unsafe { res.assume_init() } )
    }

    /**
        Gets the date of the last day of the month in a specified date.

        ## Example
        ```
        use sibyl as oracle;

        let env = oracle::env()?;
        let date = oracle::Date::new(2020, 2, 9, &env);
        let date = date.month_last_day()?;
        let (y,m,d) = date.get_date();

        assert_eq!(2020, y);
        assert_eq!(   2, m);
        assert_eq!(  29, d);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn month_last_day(&self) -> Result<Date> {
        let env = self.env;
        let mut date = mem::MaybeUninit::<OCIDate>::uninit();
        catch!{env.err_ptr() =>
            OCIDateLastDay(env.err_ptr(), self.as_ptr(), date.as_mut_ptr())
        }
        Ok( Self { env, date: unsafe { date.assume_init() } } )
    }

    /**
        Gets the date of the next day of the week after a given date.

        ## Example
        The following code example shows how to get the date of the next Monday after April 18, 1996 (a Thursday).
        ```
        use sibyl as oracle;

        let env = oracle::env()?;
        let mar28_1996 = oracle::Date::from_string("28-MAR-1996", "DD-MON-YYYY", &env)?;
        let next_mon = mar28_1996.next_week_day("MONDAY")?;
        let next_mon = next_mon.to_string("fmDD-Mon-YYYY")?;

        assert_eq!("1-Apr-1996", next_mon);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn next_week_day(&self, weekday: &str) -> Result<Date> {
        let env = self.env;
        let mut date = mem::MaybeUninit::<OCIDate>::uninit();
        catch!{env.err_ptr() =>
            OCIDateNextDay(
                env.err_ptr(), self.as_ptr(),
                weekday.as_ptr(), weekday.len() as u32,
                date.as_mut_ptr()
            )
        }
        Ok( Self { env, date: unsafe { date.assume_init() } } )
    }
}

impl ToSql for Date<'_> {
    fn to_sql(&self) -> (u16, *const c_void, usize) {
        ( SQLT_ODT, self.as_ptr() as *const c_void, std::mem::size_of::<OCIDate>() )
    }
}

impl ToSqlOut for Date<'_> {
    fn to_sql_output(&mut self, _col_size: usize) -> (u16, *mut c_void, usize) {
        (SQLT_ODT, self.as_mut_ptr() as *mut c_void, std::mem::size_of::<OCIDate>())
    }
}

impl ToSqlOut for OCIDate {
    fn to_sql_output(&mut self, _col_size: usize) -> (u16, *mut c_void, usize) {
        (SQLT_ODT, self as *mut OCIDate as *mut c_void, std::mem::size_of::<OCIDate>())
    }
}
