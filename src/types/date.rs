//! The Oracle DATE which represents the year, month, day, hour, minute, and second of the date.

mod tosql;

use crate::{ Result, oci::{self, *} };
use std::{ mem, cmp::Ordering };

/// Returns a stub date to be used as a row's column buffer or an output variable
pub(crate) fn new() -> OCIDate {
    OCIDate {
        year: 0, month: 1, day: 1, hour: 0, min: 0, sec: 0
    }
}

pub(crate) fn to_string(fmt: &str, date: &OCIDate, err: &OCIError) -> Result<String> {
    let txt = mem::MaybeUninit::<[u8;128]>::uninit();
    let mut txt = unsafe { txt.assume_init() };
    let mut txt_len = txt.len() as u32;
    oci::date_to_text(err, date, fmt.as_ptr(), fmt.len() as u8, &mut txt_len, txt.as_mut_ptr())?;
    let txt = &txt[0..txt_len as usize];
    Ok( String::from_utf8_lossy(txt).to_string() )
}

pub(crate) fn from_date<'a>(from: &OCIDate, err: &'a OCIError) -> Result<Date<'a>> {
    let date = *from;
    Ok( Date { date, err } )
}

// fn check_date(date: &OCIDate, err: &OCIError) -> Result<()> {
//     let mut res = 0;
//     oci::date_check(err, date, &mut res)?;
//     if res == 0 {
//         Ok(())
//     } else {
//         let mut msg = String::with_capacity(256);
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
    err:  &'a OCIError,
}

// impl Date<'_> {
//     pub(crate) fn as_ptr(&self) -> *const OCIDate {
//         &self.date
//     }

//     pub(crate) fn as_mut_ptr(&mut self) -> *mut OCIDate {
//         &mut self.date
//     }
// }

impl<'a> Date<'a> {

    /// Returns intentionally invalid (year zero) date to be used as an output variable
    pub fn new(err: &'a impl AsRef<OCIError>) -> Self {
        Self { date: new(), err: err.as_ref() }
    }

    /**
        Constructs a new date.

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let date = Date::with_date(1969, 7, 16, &env);

        assert_eq!(date.date(), (1969, 7, 16));
        assert_eq!(date.time(), (0, 0, 0));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn with_date(year: i16, month: u8, day: u8, err: &'a impl AsRef<OCIError>) -> Self {
        Self { 
            date: OCIDate { year, month, day, hour: 0, min: 0, sec: 0 }, 
            err: err.as_ref() 
        }
    }

    /**
        Constructs new date with time.

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let date = Date::with_date_and_time(1969, 7, 24, 16, 50, 35, &env);

        assert_eq!(date.date(), (1969, 7, 24));
        assert_eq!(date.time(), (16, 50, 35));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn with_date_and_time(year: i16, month: u8, day: u8, hour: u8, min: u8, sec: u8, err: &'a impl AsRef<OCIError>) -> Self {
        Self { 
            date: OCIDate { year, month, day, hour, min, sec }, 
            err: err.as_ref() 
        }
    }

    /**
        Converts a character string to a date type according to the specified format.

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let date = Date::from_string("July 4, 1776", "MONTH DD, YYYY", &env)?;

        assert_eq!(date.date(), (1776, 7, 4));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn from_string(txt: &str, fmt: &str, err: &'a impl AsRef<OCIError>) -> Result<Self> {
        let mut date = mem::MaybeUninit::<OCIDate>::uninit();
        oci::date_from_text(err.as_ref(), txt.as_ptr(), txt.len() as u32, fmt.as_ptr(), fmt.len() as u8, date.as_mut_ptr())?;
        let date = unsafe { date.assume_init() };
        Ok( Self { date, err: err.as_ref() } )
    }

    /**
        Constructs new date from the client's system clock.

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let date = Date::from_sysdate(&env)?;
        let (year, _month, _day) = date.date();

        assert!(year >= 2021);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn from_sysdate(err: &'a impl AsRef<OCIError>) -> Result<Self> {
        let mut date = mem::MaybeUninit::<OCIDate>::uninit();
        oci::date_sys_date(err.as_ref(), date.as_mut_ptr())?;
        let date = unsafe { date.assume_init() };
        Ok( Self { date, err: err.as_ref() } )
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

        assert_eq!(dst.date(), (1776, 7, 4));
        assert_eq!(dst.compare(&src)?, Ordering::Equal);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn from_date(from: &'a Date) -> Result<Self> {
        from_date(&from.date, from.err)
    }

    /**
        Gets the year, month, and day stored in an Oracle date.

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let date = Date::from_string("July 4, 1776", "MONTH DD, YYYY", &env)?;
        let (year, month, day) = date.date();

        assert_eq!(year,  1776);
        assert_eq!(month, 7);
        assert_eq!(day,   4);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn date(&self) -> (i16, u8, u8) {
        (self.date.year, self.date.month, self.date.day)
    }

    /**
        Changes the date leaving the time intact.

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let mut date = Date::from_string("July 4, 1776", "MONTH DD, YYYY", &env)?;
        assert_eq!(date.date(), (1776, 7, 4));

        date.set_date(1787, 9, 17);
        assert_eq!(date.date(), (1787, 9, 17));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn set_date(&mut self, year: i16, month: u8, day: u8) {
        self.date.year  = year;
        self.date.month = month;
        self.date.day   = day;
    }

    /**
        Gets the time stored in an Oracle date.

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let date = Date::with_date_and_time(1969, 7, 24, 16, 50, 35, &env);
        let (hour, min, sec) = date.time();

        assert_eq!((hour, min, sec), (16, 50, 35));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn time(&self) -> (u8, u8, u8) {
        (self.date.hour, self.date.min, self.date.sec)
    }

    /**
        Changes the time leaving the date intact.

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let mut date = Date::with_date(1969, 7, 16, &env);
        date.set_time(13, 32, 0);

        assert_eq!(date.date(), (1969, 7, 16));
        assert_eq!(date.time(), (13, 32, 0));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn set_time(&mut self, hour: u8, min: u8, sec: u8) {
        self.date.hour = hour;
        self.date.min  = min;
        self.date.sec  = sec;
    }

    /**
        Retrieves the year, month, day, hours, minutes and seconds from an Oracle date.

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let date = Date::with_date_and_time(1969, 7, 24, 16, 50, 35, &env);

        assert_eq!(date.date_and_time(), (1969, 7, 24, 16, 50, 35));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn date_and_time(&self) -> (i16, u8, u8, u8, u8, u8) {
        (self.date.year, self.date.month, self.date.day, self.date.hour, self.date.min, self.date.sec)
    }

    /**
        Changes the date and time.

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let mut date = Date::with_date_and_time(1969, 7, 16, 13, 32,  0, &env);
        date.set_date_and_time(1969, 7, 24, 16, 50, 35);

        assert_eq!(date.date_and_time(), (1969, 7, 24, 16, 50, 35));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn set_date_and_time(&mut self, year: i16, month: u8, day: u8, hour: u8, min: u8, sec: u8) {
        self.date.year  = year;
        self.date.month = month;
        self.date.day   = day;
        self.date.hour  = hour;
        self.date.min   = min;
        self.date.sec   = sec;
    }

    /**
        Returns a string according to the specified format.

        Refer to Oracle [Format Models](https://docs.oracle.com/en/database/oracle/oracle-database/19/sqlrf/Format-Models.html)
        for the description of format.

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let date = Date::with_date(-1952, 2, 25, &env);
        let res = date.to_string("FMMonth DD, YYYY BC")?;

        assert_eq!("February 25, 1952 BC", res);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn to_string(&self, fmt: &str) -> Result<String> {
        to_string(fmt, &self.date, self.err)
    }

    /**
        Adds or subtracts days from this date

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let start = Date::with_date(1969, 7, 16, &env);
        let end = start.add_days(8)?;

        assert_eq!(end.date(), (1969, 7, 24));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn add_days(&self, num: i32) -> Result<Self> {
        let mut date = mem::MaybeUninit::<OCIDate>::uninit();
        oci::date_add_days(self.err, &self.date, num, date.as_mut_ptr())?;
        let date = unsafe { date.assume_init() };
        Ok( Self { date, ..*self } )
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

        let date = Date::with_date(2019, 12, 31, &env);

        let date = date.add_months(2)?;
        assert_eq!(date.date(), (2020, 2, 29));

        let date = date.add_months(2)?;
        assert_eq!(date.date(), (2020, 4, 30));

        let date = date.add_months(-1)?;
        assert_eq!(date.date(), (2020, 3, 31));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn add_months(&self, num: i32) -> Result<Self> {
        let mut date = mem::MaybeUninit::<OCIDate>::uninit();
        oci::date_add_months(self.err, &self.date, num, date.as_mut_ptr())?;
        let date = unsafe { date.assume_init() };
        Ok( Self { date, ..*self } )
    }

    /**
        Compares this date with the `other` date.

        # Example
        ```
        use std::cmp::Ordering;
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let start = Date::with_date_and_time(1969, 7, 16, 13, 32, 0, &env);
        let end = Date::with_date_and_time(1969, 7, 24, 16, 50, 35, &env);

        assert_eq!(start.compare(&end)?, Ordering::Less);
        assert_eq!(end.compare(&start)?, Ordering::Greater);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn compare(&self, other: &Date) -> Result<Ordering> {
        let mut res = 0i32;
        oci::date_compare(self.err, &self.date, &other.date, &mut res)?;
        Ok(
            if res == 0 { Ordering::Equal } else if res < 0 { Ordering::Less } else { Ordering::Greater }
        )
    }

    /**
        Gets the number of days between two dates.

        When the number of days between date1 and date2 is computed, the time is ignored.

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let pearl_harbor = Date::with_date(1941, 12, 7, &env);
        let normandy_landings = Date::with_date(1944, 6, 6, &env);
        let days_between = normandy_landings.days_from(&pearl_harbor)?;

        assert_eq!(days_between, 912);
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn days_from(&self, other: &Date) -> Result<i32> {
        let mut res = 0i32;
        oci::date_days_between(self.err, &self.date, &other.date, &mut res)?;
        Ok( res )
    }

    /**
        Gets the date of the last day of the month in a specified date.

        # Example
        ```
        use sibyl::{ self as oracle, Date };
        let env = oracle::env()?;

        let date = Date::with_date(2020, 2, 9, &env);
        let last_day_of_the_month = date.month_last_day()?;

        assert_eq!(last_day_of_the_month.date(), (2020, 2, 29));
        # Ok::<(),oracle::Error>(())
        ```
    */
    pub fn month_last_day(&self) -> Result<Self> {
        let mut date = mem::MaybeUninit::<OCIDate>::uninit();
        oci::date_last_day(self.err, &self.date, date.as_mut_ptr())?;
        let date = unsafe { date.assume_init() };
        Ok( Self { date, ..*self } )
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
        let mut date = mem::MaybeUninit::<OCIDate>::uninit();
        oci::date_next_day(self.err, &self.date, weekday.as_ptr(), weekday.len() as u32, date.as_mut_ptr())?;
        let date = unsafe { date.assume_init() };
        Ok( Self { date, ..*self } )
    }
}

impl std::fmt::Debug for Date<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.date.fmt(f)
    }
}
