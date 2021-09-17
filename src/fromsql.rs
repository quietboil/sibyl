use crate::*;
use crate::types::*;
use crate::stmt::Stmt;
use crate::column::ColumnBuffer;

/// A trait for types which instances can be created from the returned Oracle values.
pub trait FromSql<'a> : Sized {
    /**
        Converts, if possible, data stored in the column buffer into the requested
        type and returns the instance of it. Returns error if the conversion fails
        or not defined from the type of the column buffer into a requested type.
    */
    fn value(val: &ColumnBuffer, stmt: &'a dyn Stmt) -> Result<Self>;
}

impl<'a> FromSql<'a> for String {
    fn value(val: &ColumnBuffer, stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnBuffer::Text( oci_str )       => Ok( varchar::to_string(*oci_str, stmt.env_ptr()) ),
            ColumnBuffer::Number( oci_num_box ) => number::to_string("TM", oci_num_box.as_ref() as *const number::OCINumber, stmt.err_ptr()),
            ColumnBuffer::Date( oci_date )      => date::to_string("YYYY-MM-DD HH24::MI:SS", oci_date as *const date::OCIDate, stmt.err_ptr()),
            ColumnBuffer::Timestamp( ts )       => timestamp::to_string("YYYY-MM-DD HH24:MI:SSXFF", 3, ts.get(), stmt.usr_env()),
            ColumnBuffer::TimestampTZ( ts )     => timestamp::to_string("YYYY-MM-DD HH24:MI:SSXFF TZH:TZM", 3, ts.get(), stmt.usr_env()),
            ColumnBuffer::TimestampLTZ( ts )    => timestamp::to_string("YYYY-MM-DD HH24:MI:SSXFF TZH:TZM", 3, ts.get(), stmt.usr_env()),
            ColumnBuffer::IntervalYM( int )     => interval::to_string(4, 3, int.get(), stmt.usr_env()),
            ColumnBuffer::IntervalDS( int )     => interval::to_string(9, 5, int.get(), stmt.usr_env()),
            ColumnBuffer::Float( val )          => Ok( val.to_string() ),
            ColumnBuffer::Double( val )         => Ok( val.to_string() ),
            &ColumnBuffer::Rowid( ref rowid )   => rowid.to_string(stmt.err_ptr()),
            _                                   => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for Varchar<'a> {
    fn value(val: &ColumnBuffer, stmt: &'a dyn Stmt) -> Result<Self> {
        if let ColumnBuffer::Text( oci_str ) = val {
            Varchar::from_ocistring(*oci_str, stmt.env())
        } else {
            let text : String = FromSql::value(val, stmt)?;
            Varchar::from(&text, stmt.env())
        }
    }
}

impl<'a> FromSql<'a> for &'a str {
    fn value(val: &ColumnBuffer, stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnBuffer::Text( oci_str ) => Ok( varchar::as_str(*oci_str, stmt.usr_env().env_ptr()) ),
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for &'a [u8] {
    fn value(val: &ColumnBuffer, stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnBuffer::Binary( oci_raw ) => Ok( raw::as_bytes(*oci_raw, stmt.usr_env().env_ptr()) ),
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a, T: number::Integer> FromSql<'a> for T {
    fn value(val: &ColumnBuffer, stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnBuffer::Number( oci_num_box ) => <T>::from_number(oci_num_box, stmt.err_ptr()),
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for f32 {
    fn value(val: &ColumnBuffer, stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnBuffer::Number( oci_num_box ) => number::to_real(oci_num_box, stmt.err_ptr()),
            ColumnBuffer::Float( val )          => Ok( *val ),
            ColumnBuffer::Double( val )         => Ok( *val as f32 ),
            ColumnBuffer::IntervalYM( int )     => {
                let num = interval::to_number(int.get(), stmt.usr_env())?;
                number::to_real(&num, stmt.err_ptr())
            }
            ColumnBuffer::IntervalDS( int )     => {
                let num = interval::to_number(int.get(), stmt.usr_env())?;
                number::to_real(&num, stmt.err_ptr())
            }
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for f64 {
    fn value(val: &ColumnBuffer, stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnBuffer::Number( oci_num_box ) => number::to_real(oci_num_box, stmt.err_ptr()),
            ColumnBuffer::Float( val )          => Ok( *val as f64 ),
            ColumnBuffer::Double( val )         => Ok( *val ),
            ColumnBuffer::IntervalYM( int )     => {
                let num = interval::to_number(int.get(), stmt.usr_env())?;
                number::to_real(&num, stmt.err_ptr())
            }
            ColumnBuffer::IntervalDS( int )     => {
                let num = interval::to_number(int.get(), stmt.usr_env())?;
                number::to_real(&num, stmt.err_ptr())
            }
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for number::Number<'a> {
    fn value(val: &ColumnBuffer, stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnBuffer::Number( oci_num_box ) => number::from_number(oci_num_box, stmt.usr_env()),
            ColumnBuffer::Float( val )          => number::Number::from_real(*val, stmt.usr_env()),
            ColumnBuffer::Double( val )         => number::Number::from_real(*val, stmt.usr_env()),
            ColumnBuffer::IntervalYM( int )     => {
                let num = interval::to_number(int.get(), stmt.usr_env())?;
                Ok( number::new_number(num, stmt.usr_env()) )
            }
            ColumnBuffer::IntervalDS( int )     => {
                let num = interval::to_number(int.get(), stmt.usr_env())?;
                Ok( number::new_number(num, stmt.usr_env()) )
            }
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for Date<'a> {
    fn value(val: &ColumnBuffer, stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnBuffer::Date( oci_date ) => date::from_date(oci_date, stmt.usr_env()),
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for Timestamp<'a> {
    fn value(val: &ColumnBuffer, stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnBuffer::Timestamp( ts )    => timestamp::from_timestamp(ts, stmt.usr_env()),
            ColumnBuffer::TimestampTZ( ts )  => timestamp::convert_into(ts, stmt.usr_env()),
            ColumnBuffer::TimestampLTZ( ts ) => timestamp::convert_into(ts, stmt.usr_env()),
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for TimestampTZ<'a> {
    fn value(val: &ColumnBuffer, stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnBuffer::Timestamp( ts )    => timestamp::convert_into(ts, stmt.usr_env()),
            ColumnBuffer::TimestampTZ( ts )  => timestamp::from_timestamp(ts, stmt.usr_env()),
            ColumnBuffer::TimestampLTZ( ts ) => timestamp::convert_into(ts, stmt.usr_env()),
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for TimestampLTZ<'a> {
    fn value(val: &ColumnBuffer, stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnBuffer::Timestamp( ts )    => timestamp::convert_into(ts, stmt.usr_env()),
            ColumnBuffer::TimestampTZ( ts )  => timestamp::convert_into(ts, stmt.usr_env()),
            ColumnBuffer::TimestampLTZ( ts ) => timestamp::from_timestamp(ts, stmt.usr_env()),
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for IntervalYM<'a> {
    fn value(val: &ColumnBuffer, stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnBuffer::IntervalYM( int )  => interval::from_interval(int, stmt.usr_env()),
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for IntervalDS<'a> {
    fn value(val: &ColumnBuffer, stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnBuffer::IntervalDS( int )  => interval::from_interval(int, stmt.usr_env()),
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for Cursor<'a> {
    fn value(val: &ColumnBuffer, stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnBuffer::Cursor( handle ) => {
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
            fn value(val: &ColumnBuffer, stmt: &'a dyn Stmt) -> Result<Self> {
                match val {
                    $var ( lob ) => {
                        if lob::is_initialized(lob, stmt.env_ptr(), stmt.err_ptr())? {
                            let loc = lob.take(stmt.env_ptr())?;
                            Ok( $t::make(loc, stmt.conn()) )
                        } else {
                            Err(Error::new("already consumed"))
                        }
                    },
                    _ => Err( Error::new("cannot convert") )
                }
            }
        }
    };
}

impl_from_lob!{ ColumnBuffer::CLOB  => CLOB  }
impl_from_lob!{ ColumnBuffer::BLOB  => BLOB  }
impl_from_lob!{ ColumnBuffer::BFile => BFile }

impl<'a> FromSql<'a> for RowID {
    fn value(val: &ColumnBuffer, stmt: &'a dyn Stmt) -> Result<Self> {
        match val {
            ColumnBuffer::Rowid( rowid )  => {
                if rowid.is_initialized() {
                    rowid.take(stmt.env_ptr())
                } else {
                    Err(Error::new("already consumed"))
                }
            },
            _ => Err( Error::new("cannot convert") )
        }
    }
}

// fn dump<T: desc::DescriptorType>(desc: &desc::Descriptor<T>, pfx: &str) {
//     let ptr = desc.get() as *const libc::c_void as *const u8;
//     let mem = std::ptr::slice_from_raw_parts(ptr, 32);
//     let mem = unsafe { &*mem };
//     println!("{}: {:?}", pfx, mem);
// }

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn from_lob() -> Result<()> {
        let dbname = std::env::var("DBNAME").expect("database name");
        let dbuser = std::env::var("DBUSER").expect("schema name");
        let dbpass = std::env::var("DBPASS").expect("password");
        let oracle = env()?;
        let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            DECLARE
                name_already_used EXCEPTION; PRAGMA EXCEPTION_INIT(name_already_used, -955);
            BEGIN
                EXECUTE IMMEDIATE '
                    CREATE TABLE test_large_object_data (
                        id      NUMBER GENERATED ALWAYS AS IDENTITY,
                        bin     BLOB,
                        text    CLOB,
                        ntxt    NCLOB,
                        fbin    BFILE
                    )
                ';
            EXCEPTION
              WHEN name_already_used THEN
                EXECUTE IMMEDIATE '
                    TRUNCATE TABLE test_large_object_data
                ';
            END;
        ")?;
        stmt.execute(&[])?;

        let stmt = conn.prepare("
            INSERT INTO test_large_object_data (fbin) VALUES (BFileName('MEDIA_DIR',:NAME))
        ")?;
        let count = stmt.execute(&[ &(":NAME", "hello_world.txt") ])?;
        assert_eq!(count, 1);
        let count = stmt.execute(&[ &(":NAME", "hello_supplemental.txt") ])?;
        assert_eq!(count, 1);

        let stmt = conn.prepare("SELECT fbin FROM test_large_object_data ORDER BY id")?;
        let rows = stmt.query(&[])?;

        let row  = rows.next()?.expect("first row from the result set");
        let lob : BFile = row.get(0)?.expect("first row BFILE locator");
        assert!(lob.file_exists()?);
        let (dir, name) = lob.file_name()?;
        assert_eq!(dir, "MEDIA_DIR");
        assert_eq!(name, "hello_world.txt");
        assert_eq!(lob.len()?, 28);

        let row  = rows.next()?.expect("second row from the result set");
        let lob : BFile = row.get(0)?.expect("second row BFILE locator");
        assert!(lob.file_exists()?);
        let (dir, name) = lob.file_name()?;
        assert_eq!(dir, "MEDIA_DIR");
        assert_eq!(name, "hello_supplemental.txt");
        assert_eq!(lob.len()?, 18);

        match get_bfile(&row) {
            Ok(_) => panic!("unexpected duplicate LOB locator"),
            Err(Error::Interface(msg)) => assert_eq!(msg, "already consumed"),
            Err(err) => panic!("unexpected error: {:?}", err)
        }

        Ok(())
    }

    fn get_bfile<'a>(row: &'a Row) -> Result<BFile<'a>> {
        let lob : BFile = row.get(0)?.unwrap();
        Ok(lob)
    }

    #[test]
    fn from_rowid() -> Result<()> {
        let dbname = std::env::var("DBNAME").expect("database name");
        let dbuser = std::env::var("DBUSER").expect("schema name");
        let dbpass = std::env::var("DBPASS").expect("password");
        let oracle = env()?;
        let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            SELECT ROWID, manager_id
              FROM hr.employees
             WHERE employee_id = :ID
        ")?;
        let rows = stmt.query(&[ &(":ID", 107) ])?;
        let row = rows.next()?.expect("selected row");
        let strid : String = row.get(0)?.expect("ROWID as text");
        let rowid : RowID = row.get(0)?.expect("ROWID");
        assert_eq!(rowid.to_string(conn.err_ptr())?, strid);
        let manager_id: u32 = row.get(1)?.expect("menager ID");
        assert_eq!(manager_id, 102);

        match get_rowid(&row) {
            Ok(_) => panic!("unexpected duplicate ROWID descriptor"),
            Err(Error::Interface(msg)) => assert_eq!(msg, "already consumed"),
            Err(err) => panic!("unexpected error: {:?}", err)
        }

        Ok(())
    }

    fn get_rowid(row: &Row) -> Result<RowID> {
        let rowid : RowID = row.get(0)?.expect("ROWID pseudo-column");
        Ok(rowid)
    }
}
