//! The Oracle DATE which represents the year, month, day, hour, minute, and second of the date.

mod tosql;

use crate::{ Result, oci::*, env::Env };
use std::{ mem, ptr, cmp::Ordering };

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-E0890180-8714-4243-A585-0FD21EB05CA9
    fn OCIDateAddDays(
        env:        *mut OCIError,
        date:       *const OCIDate,
        num_days:   i32,
        result:     *mut OCIDate
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-CE37ECF1-622A-49A9-A9FD-40E1BD67C941
    fn OCIDateAddMonths(
        env:        *mut OCIError,
        date:       *const OCIDate,
        num_months: i32,
        result:     *mut OCIDate
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-2251373B-4F7B-4680-BB90-F9013216465A
    fn OCIDateAssign(
        env:        *mut OCIError,
        date:       *const OCIDate,
        result:     *mut OCIDate
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-067F7EB4-419B-4A5B-B1C4-B4C650B874A3
    // fn OCIDateCheck(
    //     env:        *mut OCIError,
    //     date:       *const OCIDate,
    //     result:     *mut u32
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-282C5B79-64AA-4B34-BFC6-292144B1AD16
    fn OCIDateCompare(
        env:        *mut OCIError,
        date1:      *const OCIDate,
        date2:      *const OCIDate,
        result:     *mut i32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-42422C47-805F-4EAA-BF44-E6DE6164082E
    fn OCIDateDaysBetween(
        env:        *mut OCIError,
        date1:      *const OCIDate,
        date2:      *const OCIDate,
        result:     *mut i32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-EA8FEB07-401C-477E-805B-CC9E89FB13F4
    fn OCIDateFromText(
        env:        *mut OCIError,
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
        env:        *mut OCIError,
        date:       *const OCIDate,
        result:     *mut OCIDate
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-A16AB88E-A3BF-4B50-8FEF-6427926198F4
    fn OCIDateNextDay(
        env:        *mut OCIError,
        date:       *const OCIDate,
        day:        *const u8,
        day_len:    u32,
        result:     *mut OCIDate
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-123DD789-48A2-4AD7-8B1E-5E454DFE3F1E
    fn OCIDateToText(
        env:        *mut OCIError,
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
        env:        *mut OCIError,
        result:     *mut OCIDate
    ) -> i32;
}

/// C mapping of the Oracle DATE type (SQLT_ODT)
#[derive(Debug)]
#[repr(C)]
pub struct OCIDate {
    year: i16, // gregorian year: range is -4712 <= year <= 9999
    month: u8, // month: range is 1 <= month <= 12
    day:   u8, // day: range is 1 <= day <= 31
    hour:  u8, // hours: range is 0 <= hours <= 23
    min:   u8, // minutes: range is 0 <= minutes <= 59
    sec:   u8  // seconds: range is 0 <= seconds <= 59
}

/// Returns unitinialized date to be used as a row's column buffer or an output variable
pub(crate) fn new() -> OCIDate {
    let date = mem::MaybeUninit::<OCIDate>::uninit();
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

pub(crate) fn from_date<'a>(from: *const OCIDate, env: &'a dyn Env) -> Result<Date<'a>> {
    let mut date = mem::MaybeUninit::<OCIDate>::uninit();
    catch!{env.err_ptr() =>
        OCIDateAssign(env.err_ptr(), from, date.as_mut_ptr())
    }
    let date = unsafe { date.assume_init() };
    Ok( Date { date, env } )
}

// fn check_date(date: &OCIDate, env: & dyn Env) -> Result<()> {
//     let mut res = 0;
//     catch!{env.err_ptr() =>
//         OCIDateCheck(env.err_ptr(), date, &mut res)
//     };
//     if res == 0 {
//         Ok(())
//     } else {
//         let mut msg = String::with_capacity(1024);
//         if (res & 0x0001) != 0 { msg.push_str("Bad day");               res &= !0x0001; }
//         if (res & 0x0002) != 0 { msg.push_str("Day below valid. ");     res &= !0x0002; }
//         if (res & 0x0004) != 0 { msg.push_str("Bad month. ");           res &= !0x0004; }
//         if (res & 0x0008) != 0 { msg.push_str("Month below valid. ");   res &= !0x0008; }
//         if (res & 0x0010) != 0 { msg.push_str("Bad year. ");            res &= !0x0010; }
//         if (res & 0x0020) != 0 { msg.push_str("Year below valid. ");    res &= !0x0020; }
//         if (res & 0x0040) != 0 { msg.push_str("Bad hour. ");            res &= !0x0040; }
//         if (res & 0x0080) != 0 { msg.push_str("Hour below valid. ");    res &= !0x0080; }
//         if (res & 0x0100) != 0 { msg.push_str("Bad min. ");             res &= !0x0100; }
//         if (res & 0x0200) != 0 { msg.push_str("Min below valid. ");     res &= !0x0200; }
//         if (res & 0x0400) != 0 { msg.push_str("Bad sec. ");             res &= !0x0400; }
//         if (res & 0x0800) != 0 { msg.push_str("Sec below valid. ");     res &= !0x0800; }
//         if (res & 0x1000) != 0 { msg.push_str("1582 missing day. ");    res &= !0x1000; }
//         if (res & 0x2000) != 0 { msg.push_str("Year zero. ");           res &= !0x2000; }
//         if (res & 0x8000) != 0 { msg.push_str("Bad format. ");          res &= !0x8000; }
//         if res != 0 { msg.push_str(&format!("And {:x}", res)); }
//         Err(Error::new(&msg))
//     }
// }

/// Represents Oracle DATE
pub struct Date<'a> {
    date: OCIDate,
    env: &'a dyn Env,
}

impl Date<'_> {
    pub(crate) fn as_ptr(&self) -> *const OCIDate {
        &self.date
    }

    pub(crate) fn as_mut_ptr(&mut self) -> *mut OCIDate {
        &mut self.date
    }
}

impl<'a> Date<'a> {

    /// Returns unitinialized (and invalid) date to be used as an output variable
    pub fn new(env: &'a dyn Env) -> Self {
        Self { date: new(), env }
    }

    /**
        Constructs a new date.

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let date = Date::with_date(1969, 7, 16, &env)?;

        assert_eq!(date.get_date(), (1969, 7, 16));
        assert_eq!(date.get_time(), (0, 0, 0));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn with_date(year: i16, month: u8, day: u8, env: &'a dyn Env) -> Result<Self> {
        let date = OCIDate { year, month, day, hour: 0, min: 0, sec: 0 };
        from_date(&date, env)
    }

    /**
        Constructs new date with time.

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let date = Date::with_datetime(1969, 7, 24, 16, 50, 35, &env)?;

        assert_eq!(date.get_date(), (1969, 7, 24));
        assert_eq!(date.get_time(), (16, 50, 35));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn with_datetime(year: i16, month: u8, day: u8, hour: u8, min: u8, sec: u8, env: &'a dyn Env) -> Result<Self> {
        let date = OCIDate { year, month, day, hour, min, sec };
        from_date(&date, env)
    }

    /**
        Converts a character string to a date type according to the specified format.

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let date = Date::from_string("July 4, 1776", "MONTH DD, YYYY", &env)?;

        assert_eq!(date.get_date(), (1776, 7, 4));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn from_string(txt: &str, fmt: &str, env: &'a dyn Env) -> Result<Self> {
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
        let date = unsafe { date.assume_init() };
        Ok( Self { date, env } )
    }

    /**
        Constructs new date from the client's system clock.

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let date = Date::from_sysdate(&env)?;
        let (year, _month, _day) = date.get_date();

        assert!(year >= 2021);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn from_sysdate(env: &'a dyn Env) -> Result<Self> {
        let mut date = mem::MaybeUninit::<OCIDate>::uninit();
        catch!{env.err_ptr() =>
            OCIDateSysDate(env.err_ptr(), date.as_mut_ptr())
        }
        let date = unsafe { date.assume_init() };
        Ok( Self { env, date } )
    }

    /**
        Performs a date assignment

        # Example
        ```
        use std::cmp::Ordering;
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let src = Date::from_string("July 4, 1776", "MONTH DD, YYYY", &env)?;
        let dst = Date::from_date(&src)?;

        assert_eq!(dst.get_date(), (1776, 7, 4));
        assert_eq!(dst.compare(&src)?, Ordering::Equal);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn from_date(from: &'a Date) -> Result<Self> {
        from_date(&from.date, from.env)
    }

    /**
        Gets the year, month, and day stored in an Oracle date.

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let date = Date::from_string("July 4, 1776", "MONTH DD, YYYY", &env)?;
        let (year, month, day) = date.get_date();

        assert_eq!((year, month, day), (1776, 7, 4));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn get_date(&self) -> (i16, u8, u8) {
        (self.date.year, self.date.month, self.date.day)
    }

    /**
        Changes the date leaving the time intact.

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let mut date = Date::from_string("July 4, 1776", "MONTH DD, YYYY", &env)?;
        assert_eq!(date.get_date(), (1776, 7, 4));

        date.set_date(1787, 9, 17)?;
        assert_eq!(date.get_date(), (1787, 9, 17));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn set_date(&mut self, year: i16, month: u8, day: u8) -> Result<()> {
        let src = Self::with_datetime(year, month, day, self.date.hour, self.date.min, self.date.sec, self.env)?;
        self.date = src.date;
        Ok(())
    }

    /**
        Gets the time stored in an Oracle date.

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let date = Date::with_datetime(1969, 7, 24, 16, 50, 35, &env)?;
        let (hour, min, sec) = date.get_time();

        assert_eq!((hour, min, sec), (16, 50, 35));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn get_time(&self) -> (u8, u8, u8) {
        (self.date.hour, self.date.min, self.date.sec)
    }

    /**
        Changes the time leaving the date intact.

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let mut date = Date::with_date(1969, 7, 16, &env)?;
        date.set_time(13, 32, 0)?;

        assert_eq!(date.get_date(), (1969, 7, 16));
        assert_eq!(date.get_time(), (13, 32, 0));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn set_time(&mut self, hour: u8, min: u8, sec: u8) -> Result<()>{
        let src = Self::with_datetime(self.date.year, self.date.month, self.date.day, hour, min, sec, self.env)?;
        self.date = src.date;
        Ok(())
    }

    /**
        Retrieves the year, month, day, hours, minutes and seconds from an Oracle date.

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let date = Date::with_datetime(1969, 7, 24, 16, 50, 35, &env)?;

        assert_eq!(date.get_date_and_time(), (1969, 7, 24, 16, 50, 35));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn get_date_and_time(&self) -> (i16, u8, u8, u8, u8, u8) {
        (self.date.year, self.date.month, self.date.day, self.date.hour, self.date.min, self.date.sec)
    }

    /**
        Changes the date and time.

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let mut date = Date::with_datetime(1969, 7, 16, 13, 32,  0, &env)?;
        date.set_date_and_time(1969, 7, 24, 16, 50, 35)?;

        assert_eq!(date.get_date_and_time(), (1969, 7, 24, 16, 50, 35));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn set_date_and_time(&mut self, year: i16, month: u8, day: u8, hour: u8, min: u8, sec: u8) -> Result<()> {
        let src = Self::with_datetime(year, month, day, hour, min, sec, self.env)?;
        self.date = src.date;
        Ok(())
    }

    /**
        Returns a string according to the specified format.

        Refer to Oracle [Format Models](https://docs.oracle.com/en/database/oracle/oracle-database/19/sqlrf/Format-Models.html)
        for the description of format.

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let date = Date::with_date(-1952, 2, 25, &env)?;
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

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let start = Date::with_date(1969, 7, 16, &env)?;
        let end = start.add_days(8)?;

        assert_eq!(end.get_date(), (1969, 7, 24));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn add_days(&self, num: i32) -> Result<Self> {
        let env = self.env;
        let mut date = mem::MaybeUninit::<OCIDate>::uninit();
        catch!{env.err_ptr() =>
            OCIDateAddDays(env.err_ptr(), self.as_ptr(), num, date.as_mut_ptr())
        }
        let date = unsafe { date.assume_init() };
        Ok( Self { env, date } )
    }

    /**
        Adds or subtracts months from this date.

        If the input date is the last day of a month, then the appropriate adjustments
        are made to ensure that the output date is also the last day of the month.
        For example, Feb. 28 + 1 month = March 31, and November 30 – 3 months = August 31.
        Otherwise the result date has the same day component as date.

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let date = Date::with_date(2019, 12, 31, &env)?;

        let date = date.add_months(2)?;
        assert_eq!(date.get_date(), (2020, 2, 29));

        let date = date.add_months(2)?;
        assert_eq!(date.get_date(), (2020, 4, 30));

        let date = date.add_months(-1)?;
        assert_eq!(date.get_date(), (2020, 3, 31));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn add_months(&self, num: i32) -> Result<Self> {
        let env = self.env;
        let mut date = mem::MaybeUninit::<OCIDate>::uninit();
        catch!{env.err_ptr() =>
            OCIDateAddMonths(env.err_ptr(), self.as_ptr(), num, date.as_mut_ptr())
        }
        let date = unsafe { date.assume_init() };
        Ok( Self { env, date } )
    }

    /**
        Compares this date with the `other` date.

        # Example
        ```
        use std::cmp::Ordering;
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let start = Date::with_datetime(1969, 7, 16, 13, 32, 0, &env)?;
        let end = Date::with_datetime(1969, 7, 24, 16, 50, 35, &env)?;

        assert_eq!(start.compare(&end)?, Ordering::Less);
        assert_eq!(end.compare(&start)?, Ordering::Greater);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn compare(&self, other: &Date) -> Result<Ordering> {
        let mut res = mem::MaybeUninit::<i32>::uninit();
        catch!{self.env.err_ptr() =>
            OCIDateCompare(self.env.err_ptr(), self.as_ptr(), other.as_ptr(), res.as_mut_ptr())
        }
        let res = unsafe { res.assume_init() };
        let ordering = if res == 0 { Ordering::Equal } else if res < 0 { Ordering::Less } else { Ordering::Greater };
        Ok( ordering )
    }

    /**
        Gets the number of days between two dates.

        When the number of days between date1 and date2 is computed, the time is ignored.

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let pearl_harbor = Date::with_date(1941, 12, 7, &env)?;
        let normandy_landings = Date::with_date(1944, 6, 6, &env)?;
        let days_between = normandy_landings.days_from(&pearl_harbor)?;

        assert_eq!(days_between, 912);
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

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let date = Date::with_date(2020, 2, 9, &env)?;
        let last_day_of_the_month = date.month_last_day()?;

        assert_eq!(last_day_of_the_month.get_date(), (2020, 2, 29));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn month_last_day(&self) -> Result<Self> {
        let env = self.env;
        let mut date = mem::MaybeUninit::<OCIDate>::uninit();
        catch!{env.err_ptr() =>
            OCIDateLastDay(env.err_ptr(), self.as_ptr(), date.as_mut_ptr())
        }
        let date = unsafe { date.assume_init() };
        Ok( Self { env, date } )
    }

    /**
        Gets the date of the next day of the week after a given date.

        # Example
        The following code example shows how to get the date of the next Monday after April 18, 1996 (a Thursday).
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let mar28_1996 = Date::from_string("28-MAR-1996", "DD-MON-YYYY", &env)?;
        let next_monday = mar28_1996.next_week_day("MONDAY")?;

        assert_eq!(next_monday.to_string("fmDD-Mon-YYYY")?, "1-Apr-1996");
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn next_week_day(&self, weekday: &str) -> Result<Self> {
        let env = self.env;
        let mut date = mem::MaybeUninit::<OCIDate>::uninit();
        catch!{env.err_ptr() =>
            OCIDateNextDay(
                env.err_ptr(), self.as_ptr(),
                weekday.as_ptr(), weekday.len() as u32,
                date.as_mut_ptr()
            )
        }
        let date = unsafe { date.assume_init() };
        Ok( Self { env, date } )
    }
}

impl std::fmt::Debug for Date<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.date.fmt(f)
    }
}
