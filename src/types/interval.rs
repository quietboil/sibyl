//! The Oracle time interval data types: INTERVAL YEAR TO MONTH and INTERVAL DAY TO SECOND

use crate::*;
use crate::desc::{ Descriptor, DescriptorType };
use super::*;
use super::number::OCINumber;
use libc::{ c_void, size_t };
use std::{ mem, cmp::Ordering };

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-0E4AF4DD-5EEB-434D-BA3A-F4EDE7038FF5
    fn OCIIntervalAdd(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        addend1:    *const OCIInterval,
        addend2:    *const OCIInterval,
        result:     *mut OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-A218E261-3D40-4B69-AD64-41B697A18C98
    fn OCIIntervalAssign(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        inpinter:   *const OCIInterval,
        outinter:   *mut OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-90BA159E-79AE-47C6-844C-41BB5ADFEBD3
    // fn OCIIntervalCheck(
    //     hndl:       *mut c_void,
    //     err:        *mut OCIError,
    //     interval:   *const OCIInterval,
    //     valid:      *mut u32,
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-CCE310E5-C75E-4EDD-9B52-9CED37BDFEFF
    fn OCIIntervalCompare(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        inter1:     *const OCIInterval,
        inter2:     *const OCIInterval,
        result:     *mut i32,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-16880D01-45BE-43A3-9CF2-AEAE07B64A6B
    fn OCIIntervalDivide(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        dividend:   *const OCIInterval,
        divisor:    *const OCINumber,
        result:     *mut OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-1F8A4B39-9EA5-4CEF-9468-079E4203B68D
    fn OCIIntervalFromNumber(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        interval:   *mut OCIInterval,
        number:     *const OCINumber,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-247BB9B8-307B-4132-A1ED-5CA658B0DAA6
    fn OCIIntervalFromText(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        inpstring:  *const u8,
        str_len:    size_t,
        result:     *mut OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-12B19818-0001-42F1-8B2C-FD96B7C3231C
    fn OCIIntervalFromTZ(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        inpstring:  *const u8,
        str_len:    size_t,
        result:     *mut OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-210C4C25-3E8D-4F6D-9502-20B258DACA60
    fn OCIIntervalGetDaySecond(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        dy:         *mut i32,
        hr:         *mut i32,
        mm:         *mut i32,
        ss:         *mut i32,
        fsec:       *mut i32,
        interval:   *const OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-665EFBF6-5032-4BD3-B7A3-1C35C2D5A6B7
    fn OCIIntervalGetYearMonth(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        yr:         *mut i32,
        mnth:       *mut i32,
        interval:   *const OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-4DBA1745-E675-4774-99AB-DEE2A1FC3788
    fn OCIIntervalMultiply(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        inter:      *const OCIInterval,
        nfactor:    *const OCINumber,
        result:     *mut OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-303A876B-E1EA-4AF8-8BD1-FC133C5F3F84
    fn OCIIntervalSetDaySecond(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        dy:         i32,
        hr:         i32,
        mm:         i32,
        ss:         i32,
        fsec:       i32,
        result:     *mut OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-07D8A23E-58E2-420B-B4CA-EF37420F7549
    fn OCIIntervalSetYearMonth(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        yr:         i32,
        mnth:       i32,
        result:     *mut OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-2D0465BC-B8EA-4F41-B200-587F49D0B2CB
    fn OCIIntervalSubtract(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        minuend:    *const OCIInterval,
        subtrahend: *const OCIInterval,
        result:     *mut OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-7B403C69-F618-42A6-94F3-41FB17F7F0AD
    fn OCIIntervalToNumber(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        interval:   *const OCIInterval,
        number:     *mut OCINumber,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-DC306081-C4C3-48F5-818D-4C02DD945192
    fn OCIIntervalToText(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        interval:   *const OCIInterval,
        lfprec:     u8,
        fsprec:     u8,
        buffer:     *mut u8,
        buflen:     size_t,
        resultlen:  *mut size_t,
    ) -> i32;
}

pub(crate) fn to_string(lfprec: u8, fsprec: u8, int: *const OCIInterval, usrenv: &dyn UsrEnv) -> Result<String> {
    let mut name: [u8;32];
    let mut size: usize;
    catch!{usrenv.err_ptr() =>
        name = mem::uninitialized();
        size = mem::uninitialized();
        OCIIntervalToText(
            usrenv.as_ptr(), usrenv.err_ptr(),
            int, lfprec, fsprec,
            name.as_mut_ptr(), name.len(), &mut size
        )
    }
    let txt = &name[0..size as usize];
    Ok( String::from_utf8_lossy(txt).to_string() )
}

pub(crate) fn to_number(int: *const OCIInterval, usrenv: &dyn UsrEnv) -> Result<OCINumber> {
    let mut num: OCINumber;
    catch!{usrenv.err_ptr() =>
        num = mem::uninitialized();
        OCIIntervalToNumber(usrenv.as_ptr(), usrenv.err_ptr(), int, &mut num as *mut OCINumber)
    }
    Ok( num )
}

pub(crate) fn from_interval<'a,T>(int: &Descriptor<T>, usrenv: &'a dyn UsrEnv) -> Result<Interval<'a,T>>
    where T: DescriptorType<OCIType=OCIInterval>
{
    let interval = Descriptor::new(usrenv.env_ptr())?;
    catch!{usrenv.err_ptr() =>
        OCIIntervalAssign(
            usrenv.as_ptr(), usrenv.err_ptr(),
            int.get(), interval.get()
        )
    }
    Ok( Interval { usrenv, interval } )
}


pub struct Interval<'e, T: DescriptorType<OCIType=OCIInterval>> {
    interval: Descriptor<T>,
    usrenv: &'e dyn UsrEnv,
}

impl<'e, T> Interval<'e, T>
    where T: DescriptorType<OCIType=OCIInterval>
{
    /// Returns new uninitialized interval.
    pub fn new(usrenv: &'e dyn UsrEnv) -> Result<Self> {
        let interval = Descriptor::new(usrenv.env_ptr())?;
        Ok( Self { usrenv, interval } )
    }

    /// When given an interval string, returns the interval represented by the string.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let int = oracle::IntervalDS::from_string("3 11:45:28.150000000", &env)?;
    /// let (d,h,m,s,n) = int.get_duration()?;
    ///
    /// assert_eq!((3,11,45,28,150000000), (d,h,m,s,n));
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn from_string(txt: &str, usrenv: &'e dyn UsrEnv) -> Result<Self> {
        let interval = Descriptor::new(usrenv.env_ptr())?;
        catch!{usrenv.err_ptr() =>
            OCIIntervalFromText(
                usrenv.as_ptr(), usrenv.err_ptr(),
                txt.as_ptr(), txt.len(),
                interval.get()
            )
        }
        Ok( Self { usrenv, interval } )
    }

    /// Converts an Oracle NUMBER to an interval.
    ///
    /// `num` is in years for YEAR TO MONTH intervals and in days for DAY TO SECOND intervals
    ///
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_real(5.5, &env)?;
    /// let int = oracle::IntervalDS::from_number(&num, &env)?;
    /// let (d,h,m,s,n) = int.get_duration()?;
    ///
    /// assert_eq!((5,12,0,0,0), (d,h,m,s,n));
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn from_number(num: &Number, usrenv: &'e dyn UsrEnv) -> Result<Self> {
        let interval = Descriptor::new(usrenv.env_ptr())?;
        catch!{usrenv.err_ptr() =>
            OCIIntervalFromNumber(
                usrenv.as_ptr(), usrenv.err_ptr(),
                interval.get(), num.as_ptr()
            )
        }
        Ok( Self { usrenv, interval } )
    }

    /// Changes an interval context.
    pub fn move_to(&mut self, usrenv: &'e dyn UsrEnv) {
        self.usrenv = usrenv;
    }

    pub(crate) fn as_ptr(&self) -> *const OCIInterval {
        self.interval.get() as *const OCIInterval
    }

    pub(crate) fn as_mut_ptr(&self) -> *mut OCIInterval {
        self.interval.get() as *mut OCIInterval
    }

    /// Copies one interval to another.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let int = oracle::IntervalDS::from_string("3 11:45:28.150000000", &env)?;
    /// let cpy = oracle::IntervalDS::from_interval(&int, &env)?;
    /// let (d,h,m,s,n) = cpy.get_duration()?;
    ///
    /// assert_eq!((3,11,45,28,150000000), (d,h,m,s,n));
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn from_interval(other: &Self, usrenv: &'e dyn UsrEnv) -> Result<Self> {
        from_interval(&other.interval, usrenv)
    }

    /// Returns number of years (for YEAR TO MONTH intervals) or days (for DAY TO SECOND intervals)
    ///
    /// Fractional portions of the interval are included in the Oracle NUMBER produced.
    /// Excess precision is truncated.
    ///
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let int = oracle::IntervalDS::from_string("3 12:00:00.000000000", &env)?;
    /// let num = int.to_number(&env)?;
    /// let val = num.to_real::<f64>()?;
    ///
    /// assert_eq!(3.5, val);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn to_number(&self, usrenv: &'e dyn UsrEnv) -> Result<Number> {
        let mut num = Number::new(usrenv);
        catch!{usrenv.err_ptr() =>
            OCIIntervalToNumber(
                usrenv.as_ptr(), usrenv.err_ptr(),
                self.as_ptr(), num.as_mut_ptr()
            )
        }
        Ok( num )
    }

    /// Returns a string representing the interval.
    ///
    /// - `lfprec` is a leading field precision: the number of digits used to represent the leading field.
    /// - `fsprec` is a fractional second precision of the interval: the number of digits used to represent the fractional seconds.
    ///
    /// The interval literal is output as 'year' or '[year-]month' for INTERVAL YEAR TO MONTH intervals
    /// and as 'seconds' or 'minutes[:seconds]' or 'hours[:minutes[:seconds]]' or 'days[ hours[:minutes[:seconds]]]'
    /// for INTERVAL DAY TO SECOND intervals (where optional fields are surrounded by brackets)
    ///
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let num = oracle::Number::from_real(3.1415927, &env)?;
    /// let int = oracle::IntervalDS::from_number(&num, &env)?;
    /// let txt = int.to_string(1, 3)?;
    ///
    /// assert_eq!("+3 03:23:53.609", txt);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn to_string(&self, lfprec: u8, fsprec: u8) -> Result<String> {
        to_string(lfprec, fsprec, self.as_ptr(), self.usrenv)
    }

    /// Compares two intervals.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let i1 = oracle::IntervalDS::from_string("3 12:00:00.000000001", &env)?;
    /// let i2 = oracle::IntervalDS::from_string("3 12:00:00.000000002", &env)?;
    /// let cmp = i1.compare(&i2)?;
    ///
    /// assert_eq!(std::cmp::Ordering::Less, cmp);
    ///
    /// let cmp = i2.compare(&i1)?;
    ///
    /// assert_eq!(std::cmp::Ordering::Greater, cmp);
    ///
    /// let i3 = oracle::IntervalDS::from_interval(&i2, &env)?;
    /// let cmp = i2.compare(&i3)?;
    ///
    /// assert_eq!(std::cmp::Ordering::Equal, cmp);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn compare(&self, other: &Self) -> Result<Ordering> {
        let mut res: i32;
        catch!{self.usrenv.err_ptr() =>
            res = mem::uninitialized();
            OCIIntervalCompare(
                self.usrenv.as_ptr(), self.usrenv.err_ptr(),
                self.as_ptr(), other.as_ptr(), &mut res
            )
        }
        let ordering = if res < 0 { Ordering::Less } else if res == 0 { Ordering::Equal } else { Ordering::Greater };
        Ok( ordering )
    }

    /// Adds two intervals to produce a resulting interval.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let i1 = oracle::IntervalDS::from_string("+0 02:13:40.000000000", &env)?;
    /// let i2 = oracle::IntervalDS::from_string("+0 00:46:20.000000000", &env)?;
    /// let res = i1.add(&i2)?;
    /// let (d,h,m,s,n) = res.get_duration()?;
    ///
    /// assert_eq!((0,3,0,0,0), (d,h,m,s,n));
    ///
    /// let i3 = oracle::IntervalDS::from_string("-0 00:13:40.000000000", &env)?;
    /// let res = i1.add(&i3)?;
    /// let (d,h,m,s,n) = res.get_duration()?;
    ///
    /// assert_eq!((0,2,0,0,0), (d,h,m,s,n));
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn add(&self, other: &Self) -> Result<Self> {
        let usrenv = self.usrenv;
        let interval = Descriptor::new(self.usrenv.env_ptr())?;
        catch!{usrenv.err_ptr() =>
            OCIIntervalAdd(
                usrenv.as_ptr(), usrenv.err_ptr(),
                self.as_ptr(), other.as_ptr(),
                interval.get()
            )
        }
        Ok( Self { usrenv, interval } )
    }

    /// Subtracts an interval from this interval and returns the difference as a new interval.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let i1 = oracle::IntervalDS::from_string("+0 02:13:40.000000000", &env)?;
    /// let i2 = oracle::IntervalDS::from_string("+0 01:13:40.000000000", &env)?;
    /// let res = i1.sub(&i2)?;
    /// let (d,h,m,s,n) = res.get_duration()?;
    ///
    /// assert_eq!((0,1,0,0,0), (d,h,m,s,n));
    ///
    /// let i3 = oracle::IntervalDS::from_string("-0 01:46:20.000000000", &env)?;
    /// let res = i1.sub(&i3)?;
    /// let (d,h,m,s,n) = res.get_duration()?;
    ///
    /// assert_eq!((0,4,0,0,0), (d,h,m,s,n));
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn sub(&self, other: &Self) -> Result<Self> {
        let usrenv = self.usrenv;
        let interval = Descriptor::new(self.usrenv.env_ptr())?;
        catch!{usrenv.err_ptr() =>
            OCIIntervalSubtract(
                usrenv.as_ptr(), usrenv.err_ptr(),
                self.as_ptr(), other.as_ptr(),
                interval.get()
            )
        }
        Ok( Self { usrenv, interval } )
    }

    /// Multiplies an interval by an Oracle NUMBER to produce an interval.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let int = oracle::IntervalDS::from_string("+0 00:10:15.000000000", &env)?;
    /// let num = oracle::Number::from_int(4, &env);
    /// let res = int.mul(&num)?;
    /// let (d,h,m,s,n) = res.get_duration()?;
    ///
    /// assert_eq!((0,0,41,0,0), (d,h,m,s,n));
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn mul(&self, num: &Number) -> Result<Self> {
        let usrenv = self.usrenv;
        let interval = Descriptor::new(usrenv.env_ptr())?;
        catch!{usrenv.err_ptr() =>
            OCIIntervalMultiply(
                usrenv.as_ptr(), usrenv.err_ptr(),
                self.as_ptr(), num.as_ptr(),
                interval.get()
            )
        }
        Ok( Self { usrenv, interval } )
    }

    /// Divides an interval by an Oracle NUMBER to produce an interval.
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let int = oracle::IntervalDS::from_string("+0 00:50:15.000000000", &env)?;
    /// let num = oracle::Number::from_int(5, &env);
    /// let res = int.div(&num)?;
    /// let (d,h,m,s,n) = res.get_duration()?;
    ///
    /// assert_eq!((0,0,10,3,0), (d,h,m,s,n));
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn div(&self, num: &Number) -> Result<Self> {
        let usrenv = self.usrenv;
        let interval = Descriptor::new(usrenv.env_ptr())?;
        catch!{usrenv.err_ptr() =>
            OCIIntervalDivide(
                usrenv.as_ptr(), usrenv.err_ptr(),
                self.as_ptr(), num.as_ptr(),
                interval.get()
            )
        }
        Ok( Self { usrenv, interval } )
    }
}

impl<'e> Interval<'e, OCIIntervalDayToSecond> {
    /// Returns interval with the region ID set (if the region is specified
    /// in the input string) and the current absolute offset, or an absolute
    /// offset with the region ID set to 0
    ///
    /// The input string must be of the form [+/-]TZH:TZM or 'TZR [TZD]'
    /// ## Example
    /// ```
    /// use sibyl as oracle;
    ///
    /// let env = oracle::env()?;
    /// let int = oracle::IntervalDS::from_tz("EST", &env)?;
    /// let txt = int.to_string(1, 1)?;
    ///
    /// assert_eq!("-0 05:00:00.0", txt);
    /// # Ok::<(),oracle::Error>(())
    /// ```
    pub fn from_tz(txt: &str, usrenv: &'e dyn UsrEnv) -> Result<Self> {
        let interval = Descriptor::new(usrenv.env_ptr())?;
        catch!{usrenv.err_ptr() =>
            OCIIntervalFromTZ(
                usrenv.as_ptr(), usrenv.err_ptr(),
                txt.as_ptr(), txt.len(),
                interval.get()
            )
        }
        Ok( Self { usrenv, interval } )
    }

    /// Returns new interval with a preset duration
    pub fn from_duration(dd: i32, hh: i32, mi: i32, ss: i32, ns: i32, usrenv: &'e dyn UsrEnv) -> Result<Self> {
        let interval = Descriptor::new(usrenv.env_ptr())?;
        catch!{usrenv.err_ptr() =>
            OCIIntervalSetDaySecond(
                usrenv.as_ptr(), usrenv.err_ptr(),
                dd, hh, mi, ss, ns,
                interval.get()
            )
        }
        Ok( Self { usrenv, interval } )
    }

    /// Gets values of day, hour, minute, second, and nano seconds from an interval.
    pub fn get_duration(&self) -> Result<(i32,i32,i32,i32,i32)> {
        let mut day:  i32;
        let mut hour: i32;
        let mut min:  i32;
        let mut sec:  i32;
        let mut fsec: i32;
        catch!{self.usrenv.err_ptr() =>
            day  = mem::uninitialized();
            hour = mem::uninitialized();
            min  = mem::uninitialized();
            sec  = mem::uninitialized();
            fsec = mem::uninitialized();
            OCIIntervalGetDaySecond(
                self.usrenv.as_ptr(), self.usrenv.err_ptr(),
                &mut day, &mut hour, &mut min, &mut sec, &mut fsec,
                self.as_ptr()
            )
        }
        Ok((day, hour, min, sec, fsec))
    }

    /// Sets day, hour, minute, second, and nanosecond in an interval.
    pub fn set_duration(&mut self, dd: i32, hh: i32, mi: i32, ss: i32, ns: i32) -> Result<()> {
        catch!{self.usrenv.err_ptr() =>
            OCIIntervalSetDaySecond(
                self.usrenv.as_ptr(), self.usrenv.err_ptr(),
                dd, hh, mi, ss, ns,
                self.as_mut_ptr()
            )
        }
        Ok(())
    }
}

impl<'e> Interval<'e, OCIIntervalYearToMonth> {
    /// Returns new interval with a preset duration
    pub fn from_duration(year: i32, month: i32, usrenv: &'e dyn UsrEnv) -> Result<Self> {
        let interval = Descriptor::new(usrenv.env_ptr())?;
        catch!{usrenv.err_ptr() =>
            OCIIntervalSetYearMonth(
                usrenv.as_ptr(), usrenv.err_ptr(),
                year, month,
                interval.get()
            )
        }
        Ok( Self { usrenv, interval } )
    }

    /// Gets values of day, hour, minute, second, and nano seconds from an interval.
    pub fn get_duration(&self) -> Result<(i32,i32)> {
        let mut year:  i32;
        let mut month: i32;
        catch!{self.usrenv.err_ptr() =>
            year  = mem::uninitialized();
            month = mem::uninitialized();
            OCIIntervalGetYearMonth(
                self.usrenv.as_ptr(), self.usrenv.err_ptr(),
                &mut year, &mut month,
                self.as_ptr()
            )
        }
        Ok((year, month))
    }

    /// Sets year and month in an interval.
    pub fn set_duration(&mut self, year: i32, month: i32) -> Result<()> {
        catch!{self.usrenv.err_ptr() =>
            OCIIntervalSetYearMonth(
                self.usrenv.as_ptr(), self.usrenv.err_ptr(),
                year, month,
                self.as_mut_ptr()
            )
        }
        Ok(())
    }
}

macro_rules! impl_int_to_sql {
    ($ts:ty => $sqlt:ident) => {
        impl ToSql for Interval<'_, $ts> {
            fn to_sql(&self) -> (u16, *const c_void, usize) {
                ( $sqlt, self.interval.as_ptr() as *const c_void, std::mem::size_of::<*mut OCIInterval>() )
            }
        }
    };
}

impl_int_to_sql!{ OCIIntervalYearToMonth => SQLT_INTERVAL_YM }
impl_int_to_sql!{ OCIIntervalDayToSecond => SQLT_INTERVAL_DS }

macro_rules! impl_int_to_sql_output {
    ($ts:ty => $sqlt:ident) => {
        impl ToSqlOut for Descriptor<$ts> {
            fn to_sql_output(&mut self, _col_size: usize) -> (u16, *mut c_void, usize) {
                ($sqlt, self.as_ptr() as *mut c_void, std::mem::size_of::<*mut OCIInterval>())
            }
        }
        impl ToSqlOut for Interval<'_, $ts> {
            fn to_sql_output(&mut self, col_size: usize) -> (u16, *mut c_void, usize) {
                self.interval.to_sql_output(col_size)
            }
        }
    };
}

impl_int_to_sql_output!{ OCIIntervalYearToMonth => SQLT_INTERVAL_YM }
impl_int_to_sql_output!{ OCIIntervalDayToSecond => SQLT_INTERVAL_DS }

