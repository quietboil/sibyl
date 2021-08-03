use crate::*;
use crate::types::*;
use crate::stmt::Stmt;
use crate::column::ColumnValue;

/// A trait for types which instances can be created from the returned Oracle values.
pub trait FromSql<'a> : Sized {
    /**
        Converts, if possible, data stored in the column buffer into the requested
        type and returns the instance of it. Returns error if the conversion fails
        or not defined from the type of the column buffer into a requested type.
    */
    fn value(val: &ColumnValue, stmt: &'a dyn Stmt) -> Result<Self>;
}

impl<'a> FromSql<'a> for String {
    fn value(val: &ColumnValue, stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnValue::Text( oci_str )       => Ok( varchar::to_string(*oci_str, stmt.env_ptr()) ),
            ColumnValue::Number( oci_num_box ) => number::to_string("TM", oci_num_box.as_ref() as *const number::OCINumber, stmt.err_ptr()),
            ColumnValue::Date( oci_date )      => date::to_string("YYYY-MM-DD HH24::MI:SS", oci_date as *const date::OCIDate, stmt.err_ptr()),
            ColumnValue::Timestamp( ts )       => timestamp::to_string("YYYY-MM-DD HH24:MI:SSXFF", 3, ts.get(), stmt.usr_env()),
            ColumnValue::TimestampTZ( ts )     => timestamp::to_string("YYYY-MM-DD HH24:MI:SSXFF TZH:TZM", 3, ts.get(), stmt.usr_env()),
            ColumnValue::TimestampLTZ( ts )    => timestamp::to_string("YYYY-MM-DD HH24:MI:SSXFF TZH:TZM", 3, ts.get(), stmt.usr_env()),
            ColumnValue::IntervalYM( int )     => interval::to_string(4, 3, int.get(), stmt.usr_env()),
            ColumnValue::IntervalDS( int )     => interval::to_string(9, 3, int.get(), stmt.usr_env()),
            ColumnValue::Float( val )          => Ok( val.to_string() ),
            ColumnValue::Double( val )         => Ok( val.to_string() ),
            _                                  => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for &'a str {
    fn value(val: &ColumnValue, stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnValue::Text( oci_str ) => Ok( varchar::as_str(*oci_str, stmt.usr_env()) ),
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for &'a [u8] {
    fn value(val: &ColumnValue, stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnValue::Binary( oci_raw ) => Ok( raw::as_bytes(*oci_raw, stmt.usr_env()) ),
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a, T: number::Integer> FromSql<'a> for T {
    fn value(val: &ColumnValue, _stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnValue::Number( oci_num_box ) => <T>::from_number(oci_num_box),
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for f32 {
    fn value(val: &ColumnValue, stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnValue::Number( oci_num_box ) => number::to_real(oci_num_box, stmt.err_ptr()),
            ColumnValue::Float( val )          => Ok( *val ),
            ColumnValue::Double( val )         => Ok( *val as f32 ),
            ColumnValue::IntervalYM( int )     => {
                let num = interval::to_number(int.get(), stmt.usr_env())?;
                number::to_real(&num, stmt.err_ptr())
            }
            ColumnValue::IntervalDS( int )     => {
                let num = interval::to_number(int.get(), stmt.usr_env())?;
                number::to_real(&num, stmt.err_ptr())
            }
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for f64 {
    fn value(val: &ColumnValue, stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnValue::Number( oci_num_box ) => number::to_real(oci_num_box, stmt.err_ptr()),
            ColumnValue::Float( val )          => Ok( *val as f64 ),
            ColumnValue::Double( val )         => Ok( *val ),
            ColumnValue::IntervalYM( int )     => {
                let num = interval::to_number(int.get(), stmt.usr_env())?;
                number::to_real(&num, stmt.err_ptr())
            }
            ColumnValue::IntervalDS( int )     => {
                let num = interval::to_number(int.get(), stmt.usr_env())?;
                number::to_real(&num, stmt.err_ptr())
            }
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for number::Number<'a> {
    fn value(val: &ColumnValue, stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnValue::Number( oci_num_box ) => number::from_number(oci_num_box, stmt.usr_env()),
            ColumnValue::Float( val )          => number::Number::from_real(*val, stmt.usr_env()),
            ColumnValue::Double( val )         => number::Number::from_real(*val, stmt.usr_env()),
            ColumnValue::IntervalYM( int )     => {
                let num = interval::to_number(int.get(), stmt.usr_env())?;
                Ok( number::new_number(num, stmt.usr_env()) )
            }
            ColumnValue::IntervalDS( int )     => {
                let num = interval::to_number(int.get(), stmt.usr_env())?;
                Ok( number::new_number(num, stmt.usr_env()) )
            }
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for Date<'a> {
    fn value(val: &ColumnValue, stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnValue::Date( oci_date ) => date::from_date(oci_date, stmt.usr_env()),
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for Timestamp<'a> {
    fn value(val: &ColumnValue, stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnValue::Timestamp( ts )    => timestamp::from_timestamp(ts, stmt.usr_env()),
            ColumnValue::TimestampTZ( ts )  => timestamp::convert_into(ts, stmt.usr_env()),
            ColumnValue::TimestampLTZ( ts ) => timestamp::convert_into(ts, stmt.usr_env()),
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for TimestampTZ<'a> {
    fn value(val: &ColumnValue, stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnValue::Timestamp( ts )    => timestamp::convert_into(ts, stmt.usr_env()),
            ColumnValue::TimestampTZ( ts )  => timestamp::from_timestamp(ts, stmt.usr_env()),
            ColumnValue::TimestampLTZ( ts ) => timestamp::convert_into(ts, stmt.usr_env()),
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for TimestampLTZ<'a> {
    fn value(val: &ColumnValue, stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnValue::Timestamp( ts )    => timestamp::convert_into(ts, stmt.usr_env()),
            ColumnValue::TimestampTZ( ts )  => timestamp::convert_into(ts, stmt.usr_env()),
            ColumnValue::TimestampLTZ( ts ) => timestamp::from_timestamp(ts, stmt.usr_env()),
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for IntervalYM<'a> {
    fn value(val: &ColumnValue, stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnValue::IntervalYM( int )  => interval::from_interval(int, stmt.usr_env()),
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for IntervalDS<'a> {
    fn value(val: &ColumnValue, stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnValue::IntervalDS( int )  => interval::from_interval(int, stmt.usr_env()),
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for Cursor<'a> {
    fn value(val: &ColumnValue, stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnValue::Cursor( handle ) => {
                let ref_cursor = handle.take(stmt.env_ptr())?;
                Ok( Self::from_handle(ref_cursor, stmt) )
            }
            _ => Err( Error::new("cannot convert") )
        }
    }
}

macro_rules! impl_from_lob {
    ($var:path => $t:ident ) => {
        impl<'a> FromSql<'a> for $t<'a> {
            fn value(val: &ColumnValue, stmt: &'a dyn Stmt) -> Result<Self> {
                match val {
                    $var ( lob ) => {
                        let loc = lob.take(stmt.env_ptr())?;
                        Ok( $t::make(loc, stmt.conn()) )
                    }
                    _ => Err( Error::new("cannot convert") )
                }
            }
        }
    };
}

impl_from_lob!{ ColumnValue::CLOB  => CLOB  }
impl_from_lob!{ ColumnValue::BLOB  => BLOB  }
impl_from_lob!{ ColumnValue::BFile => BFile }
