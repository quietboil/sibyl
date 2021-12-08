use super::{cursor::Cursor, cols::ColumnBuffer, rows::Row};
use crate::{
    Error,
    IntervalDS, IntervalYM, Result, RowID, Timestamp, TimestampLTZ, TimestampTZ,
    oci::*,
    types::{
        date, interval, number, raw, timestamp, varchar,
        Date, Varchar
    },
    lob::{ self, LOB },
};

/// A trait for types which instances can be created from the returned Oracle values.
pub trait FromSql<'a> : Sized {
    /**
        Converts, if possible, data stored in the column buffer into the requested
        type and returns the instance of it. Returns error if the conversion fails
        or conversion from the type of the column buffer into a requested type is
        not defined.
    */
    fn value(row: &'a Row<'a>, col: &mut ColumnBuffer) -> Result<Self>;
}

impl<'a> FromSql<'a> for String {
    fn value(row: &'a Row<'a>, col: &mut ColumnBuffer) -> Result<Self> {
        match col {
            ColumnBuffer::Text( oci_str_ptr )   => Ok( varchar::to_string(oci_str_ptr.get(), row.env_ptr()) ),
            ColumnBuffer::Number( oci_num_box ) => number::to_string("TM", oci_num_box.as_ref() as *const OCINumber, row.err_ptr()),
            ColumnBuffer::Date( oci_date )      => date::to_string("YYYY-MM-DD HH24::MI:SS", oci_date as *const OCIDate, row.err_ptr()),
            ColumnBuffer::Timestamp( ts )       => timestamp::to_string("YYYY-MM-DD HH24:MI:SSXFF", 3, ts.get(), row.get_ctx()),
            ColumnBuffer::TimestampTZ( ts )     => timestamp::to_string("YYYY-MM-DD HH24:MI:SSXFF TZH:TZM", 3, ts.get(), row.get_ctx()),
            ColumnBuffer::TimestampLTZ( ts )    => timestamp::to_string("YYYY-MM-DD HH24:MI:SSXFF TZH:TZM", 3, ts.get(), row.get_ctx()),
            ColumnBuffer::IntervalYM( int )     => interval::to_string(4, 3, int.get(), row.get_ctx()),
            ColumnBuffer::IntervalDS( int )     => interval::to_string(9, 5, int.get(), row.get_ctx()),
            ColumnBuffer::Float( val )          => Ok( val.to_string() ),
            ColumnBuffer::Double( val )         => Ok( val.to_string() ),
            ColumnBuffer::Rowid( rowid )        => rowid.to_string(row.get_env()),
            _                                   => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for Varchar<'a> {
    fn value(row: &'a Row<'a>, col: &mut ColumnBuffer) -> Result<Self> {
        if let ColumnBuffer::Text( oci_str_ptr ) = col {
            Varchar::from_ocistring(oci_str_ptr.get(), row.get_env())
        } else {
            let text : String = FromSql::value(row, col)?;
            Varchar::from(&text, row.get_env())
        }
    }
}

impl<'a> FromSql<'a> for &'a str {
    fn value(row: &'a Row<'a>, col: &mut ColumnBuffer) -> Result<Self> {
        match col {
            ColumnBuffer::Text( oci_str_ptr ) => Ok( varchar::as_str(oci_str_ptr.get(), row.get_ctx().env_ptr()) ),
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for &'a [u8] {
    fn value(row: &'a Row<'a>, col: &mut ColumnBuffer) -> Result<Self> {
        match col {
            ColumnBuffer::Binary( oci_raw_ptr ) => Ok( raw::as_bytes(oci_raw_ptr.get(), row.get_ctx().env_ptr()) ),
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a, T: number::Integer> FromSql<'a> for T {
    fn value(row: &'a Row<'a>, col: &mut ColumnBuffer) -> Result<Self> {
        match col {
            ColumnBuffer::Number( oci_num_box ) => <T>::from_number(oci_num_box, row.err_ptr()),
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for f32 {
    fn value(row: &'a Row<'a>, col: &mut ColumnBuffer) -> Result<Self> {
        match col {
            ColumnBuffer::Number( oci_num_box ) => number::to_real(oci_num_box, row.err_ptr()),
            ColumnBuffer::Float( val )          => Ok( *val ),
            ColumnBuffer::Double( val )         => Ok( *val as f32 ),
            ColumnBuffer::IntervalYM( int )     => {
                let num = interval::to_number(int.get(), row.get_ctx())?;
                number::to_real(&num, row.err_ptr())
            }
            ColumnBuffer::IntervalDS( int )     => {
                let num = interval::to_number(int.get(), row.get_ctx())?;
                number::to_real(&num, row.err_ptr())
            }
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for f64 {
    fn value(row: &'a Row<'a>, col: &mut ColumnBuffer) -> Result<Self> {
        match col {
            ColumnBuffer::Number( oci_num_box ) => number::to_real(oci_num_box, row.err_ptr()),
            ColumnBuffer::Float( val )          => Ok( *val as f64 ),
            ColumnBuffer::Double( val )         => Ok( *val ),
            ColumnBuffer::IntervalYM( int )     => {
                let num = interval::to_number(int.get(), row.get_ctx())?;
                number::to_real(&num, row.err_ptr())
            }
            ColumnBuffer::IntervalDS( int )     => {
                let num = interval::to_number(int.get(), row.get_ctx())?;
                number::to_real(&num, row.err_ptr())
            }
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for number::Number<'a> {
    fn value(row: &'a Row<'a>, col: &mut ColumnBuffer) -> Result<Self> {
        match col {
            ColumnBuffer::Number( oci_num_box ) => number::from_number(oci_num_box, row.get_ctx()),
            ColumnBuffer::Float( val )          => number::Number::from_real(*val, row.get_ctx()),
            ColumnBuffer::Double( val )         => number::Number::from_real(*val, row.get_ctx()),
            ColumnBuffer::IntervalYM( int )     => {
                let num = interval::to_number(int.get(), row.get_ctx())?;
                Ok( number::new_number(num, row.get_ctx()) )
            }
            ColumnBuffer::IntervalDS( int )     => {
                let num = interval::to_number(int.get(), row.get_ctx())?;
                Ok( number::new_number(num, row.get_ctx()) )
            }
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for Date<'a> {
    fn value(row: &'a Row<'a>, col: &mut ColumnBuffer) -> Result<Self> {
        match col {
            ColumnBuffer::Date( oci_date ) => date::from_date(oci_date, row.get_env()),
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for Timestamp<'a> {
    fn value(row: &'a Row<'a>, col: &mut ColumnBuffer) -> Result<Self> {
        match col {
            ColumnBuffer::Timestamp( ts )    => timestamp::from_timestamp(ts, row.get_ctx()),
            ColumnBuffer::TimestampTZ( ts )  => timestamp::convert_into(ts, row.get_ctx()),
            ColumnBuffer::TimestampLTZ( ts ) => timestamp::convert_into(ts, row.get_ctx()),
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for TimestampTZ<'a> {
    fn value(row: &'a Row<'a>, col: &mut ColumnBuffer) -> Result<Self> {
        match col {
            ColumnBuffer::Timestamp( ts )    => timestamp::convert_into(ts, row.get_ctx()),
            ColumnBuffer::TimestampTZ( ts )  => timestamp::from_timestamp(ts, row.get_ctx()),
            ColumnBuffer::TimestampLTZ( ts ) => timestamp::convert_into(ts, row.get_ctx()),
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for TimestampLTZ<'a> {
    fn value(row: &'a Row<'a>, col: &mut ColumnBuffer) -> Result<Self> {
        match col {
            ColumnBuffer::Timestamp( ts )    => timestamp::convert_into(ts, row.get_ctx()),
            ColumnBuffer::TimestampTZ( ts )  => timestamp::convert_into(ts, row.get_ctx()),
            ColumnBuffer::TimestampLTZ( ts ) => timestamp::from_timestamp(ts, row.get_ctx()),
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for IntervalYM<'a> {
    fn value(row: &'a Row<'a>, col: &mut ColumnBuffer) -> Result<Self> {
        match col {
            ColumnBuffer::IntervalYM( int )  => interval::from_interval(int, row.get_ctx()),
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for IntervalDS<'a> {
    fn value(row: &'a Row<'a>, col: &mut ColumnBuffer) -> Result<Self> {
        match col {
            ColumnBuffer::IntervalDS( int )  => interval::from_interval(int, row.get_ctx()),
            _ => Err( Error::new("cannot convert") )
        }
    }
}

impl<'a> FromSql<'a> for Cursor<'a> {
    fn value(row: &'a Row<'a>, col: &mut ColumnBuffer) -> Result<Self> {
        match col {
            ColumnBuffer::Cursor( handle ) => {
                let mut ref_cursor : Handle<OCIStmt> = Handle::new(row.env_ptr())?;
                ref_cursor.swap(handle);
                Ok( Cursor::explicit(ref_cursor, row) )
            }
            _ => Err( Error::new("cannot convert") )
        }
    }
}

macro_rules! impl_from_lob {
    ($var:path => $t:ident ) => {
        impl<'a> FromSql<'a> for LOB<'a,$t> {
            fn value(row: &'a Row<'a>, col: &mut ColumnBuffer) -> Result<Self> {
                match col {
                    $var ( lob ) => {
                        if lob::is_initialized(lob, row.env_ptr(), row.err_ptr())? {
                            let mut loc : Descriptor<$t> = Descriptor::new(row.env_ptr())?;
                            loc.swap(lob);
                            Ok( LOB::<$t>::make(loc, row.conn()) )
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

impl_from_lob!{ ColumnBuffer::CLOB  => OCICLobLocator  }
impl_from_lob!{ ColumnBuffer::BLOB  => OCIBLobLocator  }
impl_from_lob!{ ColumnBuffer::BFile => OCIBFileLocator }

impl<'a> FromSql<'a> for RowID {
    fn value(row: &'a Row<'a>, col: &mut ColumnBuffer) -> Result<Self> {
        match col {
            ColumnBuffer::Rowid( rowid )  => {
                if rowid.is_initialized() {
                    let mut res = RowID::new(row.env_ptr())?;
                    res.swap(rowid);
                    Ok(res)
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

#[cfg(all(test,feature="blocking"))]
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

        if let Some(row) = rows.next()? {
            let lob : BFile = row.get(0)?.expect("first row BFILE locator");
            assert!(lob.file_exists()?);
            let (dir, name) = lob.file_name()?;
            assert_eq!(dir, "MEDIA_DIR");
            assert_eq!(name, "hello_world.txt");
            assert_eq!(lob.len()?, 28);
        }

        if let Some(row) = rows.next()? {
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
        }

        Ok(())
    }

    fn get_bfile<'a>(row: &'a Row<'a>) -> Result<BFile<'a>> {
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

        if let Some(row) = rows.next()? {
            let strid : String = row.get(0)?.expect("ROWID as text");
            let rowid : RowID = row.get(0)?.expect("ROWID");
            assert_eq!(rowid.to_string(&conn)?, strid);
            let manager_id: u32 = row.get(1)?.expect("manager ID");
            assert_eq!(manager_id, 103, "employee ID of Alexander Hunold");

            match get_rowid(&row) {
                Ok(_) => panic!("unexpected duplicate ROWID descriptor"),
                Err(Error::Interface(msg)) => assert_eq!(msg, "already consumed"),
                Err(err) => panic!("unexpected error: {:?}", err)
            }
        }

        Ok(())
    }

    fn get_rowid<'a>(row: &'a Row<'a>) -> Result<RowID> {
        let rowid : RowID = row.get(0)?.expect("ROWID pseudo-column");
        Ok(rowid)
    }
}
