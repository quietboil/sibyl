use super::{cursor::Cursor, cols::{ColumnBuffer, Column}, rows::Row};
use crate::{
    Error,
    IntervalDS, IntervalYM, Result, RowID, Timestamp, TimestampLTZ, TimestampTZ,
    oci::*,
    types::{
        date, interval, number, raw, timestamp, varchar,
        Date, Varchar, rowid
    },
    lob::{ self, LOB },
};

/// A trait for types which values can be created from the returned Oracle data.
pub trait FromSql<'a> : Sized {
    /**
        Converts, if possible, data stored in the column buffer into the requested
        type and returns the created value.
        
        Returns error if the conversion fails or conversion from the type of the
        column buffer into a requested type is not defined.
    */
    fn value(row: &'a Row<'a>, col: &mut Column) -> Result<Self>;
}

fn assert_not_null(row: &Row, col: &Column) -> Result<()> {
    if col.is_null() {
        let col_name = col.name(row.as_ref())?;
        Err(Error::msg(format!("Column {} is null", col_name)))
    } else {
        Ok(())
    }
}

impl<'a> FromSql<'a> for String {
    fn value(row: &'a Row<'a>, col: &mut Column) -> Result<Self> {
        assert_not_null(row, col)?;
        match col.data() {
            ColumnBuffer::Text( oci_str_ptr )   => Ok( varchar::to_string(oci_str_ptr, row.as_ref()) ),
            ColumnBuffer::Number( oci_num_box ) => number::to_string("TM", oci_num_box.as_ref(), row.as_ref()),
            ColumnBuffer::Date( oci_date )      => date::to_string("YYYY-MM-DD HH24::MI:SS", oci_date, row.as_ref()),
            ColumnBuffer::Timestamp( ts )       => timestamp::to_string("YYYY-MM-DD HH24:MI:SSXFF", 3, ts.as_ref(), row),
            ColumnBuffer::TimestampTZ( ts )     => timestamp::to_string("YYYY-MM-DD HH24:MI:SSXFF TZH:TZM", 3, ts.as_ref(), row),
            ColumnBuffer::TimestampLTZ( ts )    => timestamp::to_string("YYYY-MM-DD HH24:MI:SSXFF TZH:TZM", 3, ts.as_ref(), row),
            ColumnBuffer::IntervalYM( int )     => interval::to_string(int.as_ref(), 4, 3, row),
            ColumnBuffer::IntervalDS( int )     => interval::to_string(int.as_ref(), 9, 5, row),
            ColumnBuffer::Float( val )          => Ok( val.to_string() ),
            ColumnBuffer::Double( val )         => Ok( val.to_string() ),
            ColumnBuffer::Rowid( rowid )        => rowid::to_string(rowid, row.as_ref()),
            _                                   => Err( Error::new("cannot return as a String") )
        }
    }
}

impl<'a> FromSql<'a> for Varchar<'a> {
    fn value(row: &'a Row<'a>, col: &mut Column) -> Result<Self> {
        assert_not_null(row, col)?;
        if let ColumnBuffer::Text( oci_str_ptr ) = col.data() {
            Varchar::from_ocistring(oci_str_ptr, row)
        } else {
            let text : String = FromSql::value(row, col)?;
            Varchar::from(&text, row)
        }
    }
}

impl<'a> FromSql<'a> for &'a str {
    fn value(row: &'a Row<'a>, col: &mut Column) -> Result<Self> {
        assert_not_null(row, col)?;
        match col.data() {
            ColumnBuffer::Text( oci_str_ptr ) => Ok( varchar::as_str(&oci_str_ptr, row.as_ref()) ),
            _ => Err( Error::new("cannot borrow as &str") )
        }
    }
}

impl<'a> FromSql<'a> for &'a [u8] {
    fn value(row: &'a Row<'a>, col: &mut Column) -> Result<Self> {
        assert_not_null(row, col)?;
        match col.data() {
            ColumnBuffer::Binary( oci_raw_ptr ) => Ok( {
                // inlined Raw::as_bytes to deal with the buffer lifetime issue
                let ptr = raw::as_ptr(&oci_raw_ptr, row.as_ref());
                let len = raw::len(&oci_raw_ptr, row.as_ref());
                unsafe {
                    std::slice::from_raw_parts(ptr, len)
                }
            }),
            _ => Err( Error::new("cannot borrow as &[u8]") )
        }
    }
}

impl<'a, T: number::Integer> FromSql<'a> for T {
    fn value(row: &'a Row<'a>, col: &mut Column) -> Result<Self> {
        assert_not_null(row, col)?;
        match col.data() {
            ColumnBuffer::Number( oci_num_box ) => <T>::from_number(oci_num_box, row.as_ref()),
            _ => Err( Error::new("cannot return as an integer") )
        }
    }
}

impl<'a> FromSql<'a> for f32 {
    fn value(row: &'a Row<'a>, col: &mut Column) -> Result<Self> {
        assert_not_null(row, col)?;
        match col.data() {
            ColumnBuffer::Number( oci_num_box ) => number::to_real(oci_num_box, row.as_ref()),
            ColumnBuffer::Float( val )          => Ok( *val ),
            ColumnBuffer::Double( val )         => Ok( *val as f32 ),
            ColumnBuffer::IntervalYM( int )     => {
                let num = interval::to_number(int, row)?;
                number::to_real(&num, row.as_ref())
            }
            ColumnBuffer::IntervalDS( int )     => {
                let num = interval::to_number(int, row)?;
                number::to_real(&num, row.as_ref())
            }
            _ => Err( Error::new("cannot return as f32") )
        }
    }
}

impl<'a> FromSql<'a> for f64 {
    fn value(row: &'a Row<'a>, col: &mut Column) -> Result<Self> {
        assert_not_null(row, col)?;
        match col.data() {
            ColumnBuffer::Number( oci_num_box ) => number::to_real(oci_num_box, row.as_ref()),
            ColumnBuffer::Float( val )          => Ok( *val as f64 ),
            ColumnBuffer::Double( val )         => Ok( *val ),
            ColumnBuffer::IntervalYM( int )     => {
                let num = interval::to_number(int, row)?;
                number::to_real(&num, row.as_ref())
            }
            ColumnBuffer::IntervalDS( int )     => {
                let num = interval::to_number(int, row)?;
                number::to_real(&num, row.as_ref())
            }
            _ => Err( Error::new("cannot return as f64") )
        }
    }
}

impl<'a> FromSql<'a> for number::Number<'a> {
    fn value(row: &'a Row<'a>, col: &mut Column) -> Result<Self> {
        assert_not_null(row, col)?;
        match col.data() {
            ColumnBuffer::Number( oci_num_box ) => number::Number::from(oci_num_box, row),
            ColumnBuffer::Float( val )          => number::Number::from_real(*val, row),
            ColumnBuffer::Double( val )         => number::Number::from_real(*val, row),
            ColumnBuffer::IntervalYM( int )     => {
                let num = interval::to_number(int, row)?;
                Ok( number::Number::make(num, row) )
            }
            ColumnBuffer::IntervalDS( int )     => {
                let num = interval::to_number(int, row)?;
                Ok( number::Number::make(num, row) )
            }
            _ => Err( Error::new("cannot return as a Number") )
        }
    }
}

impl<'a> FromSql<'a> for Date<'a> {
    fn value(row: &'a Row<'a>, col: &mut Column) -> Result<Self> {
        assert_not_null(row, col)?;
        match col.data()  {
            ColumnBuffer::Date( oci_date ) => date::from_date(oci_date, row.as_ref()),
            _ => Err( Error::new("cannot return as a Date") )
        }
    }
}

impl<'a> FromSql<'a> for Timestamp<'a> {
    fn value(row: &'a Row<'a>, col: &mut Column) -> Result<Self> {
        assert_not_null(row, col)?;
        match col.data() {
            ColumnBuffer::Timestamp( ts )    => timestamp::from_timestamp(ts, row),
            ColumnBuffer::TimestampTZ( ts )  => timestamp::convert_into(ts, row),
            ColumnBuffer::TimestampLTZ( ts ) => timestamp::convert_into(ts, row),
            _ => Err( Error::new("cannot return as a Timestamp") )
        }
    }
}

impl<'a> FromSql<'a> for TimestampTZ<'a> {
    fn value(row: &'a Row<'a>, col: &mut Column) -> Result<Self> {
        assert_not_null(row, col)?;
        match col.data() {
            ColumnBuffer::Timestamp( ts )    => timestamp::convert_into(ts, row),
            ColumnBuffer::TimestampTZ( ts )  => timestamp::from_timestamp(ts, row),
            ColumnBuffer::TimestampLTZ( ts ) => timestamp::convert_into(ts, row),
            _ => Err( Error::new("cannot return as a Timestamp with time zone") )
        }
    }
}

impl<'a> FromSql<'a> for TimestampLTZ<'a> {
    fn value(row: &'a Row<'a>, col: &mut Column) -> Result<Self> {
        assert_not_null(row, col)?;
        match col.data() {
            ColumnBuffer::Timestamp( ts )    => timestamp::convert_into(ts, row),
            ColumnBuffer::TimestampTZ( ts )  => timestamp::convert_into(ts, row),
            ColumnBuffer::TimestampLTZ( ts ) => timestamp::from_timestamp(ts, row),
            _ => Err( Error::new("cannot return as a Timestamp with local time zone") )
        }
    }
}

impl<'a> FromSql<'a> for IntervalYM<'a> {
    fn value(row: &'a Row<'a>, col: &mut Column) -> Result<Self> {
        assert_not_null(row, col)?;
        match col.data() {
            ColumnBuffer::IntervalYM( int )  => interval::from_interval(int, row),
            _ => Err( Error::new("cannot return as Inteval year to month") )
        }
    }
}

impl<'a> FromSql<'a> for IntervalDS<'a> {
    fn value(row: &'a Row<'a>, col: &mut Column) -> Result<Self> {
        assert_not_null(row, col)?;
        match col.data() {
            ColumnBuffer::IntervalDS( int )  => interval::from_interval(int, row),
            _ => Err( Error::new("cannot return as Interval day to second") )
        }
    }
}

impl<'a> FromSql<'a> for Cursor<'a> {
    fn value(row: &'a Row<'a>, col: &mut Column) -> Result<Self> {
        assert_not_null(row, col)?;
        match col.data() {
            ColumnBuffer::Cursor( handle ) => {
                let mut ref_cursor : Handle<OCIStmt> = Handle::new(row)?;
                ref_cursor.swap(handle);
                Ok( Cursor::explicit(ref_cursor, row) )
            }
            _ => Err( Error::new("cannot return as Cursor") )
        }
    }
}

macro_rules! impl_from_lob {
    ($var:path => $t:ident ) => {
        impl<'a> FromSql<'a> for LOB<'a,$t> {
            fn value(row: &'a Row<'a>, col: &mut Column) -> Result<Self> {
                assert_not_null(row, col)?;
                match col.data() {
                    $var ( row_loc ) => {
                        if lob::is_initialized(row_loc, row.as_ref(), row.as_ref())? {
                            let mut loc : Descriptor<$t> = Descriptor::new(row)?;
                            loc.swap(row_loc);
                            Ok( LOB::<$t>::make(loc, row.session()) )
                        } else {
                            Err(Error::new("already consumed"))
                        }
                    },
                    _ => Err( Error::new("cannot return as a LOB locator") )
                }
            }
        }
    };
}

impl_from_lob!{ ColumnBuffer::CLOB  => OCICLobLocator  }
impl_from_lob!{ ColumnBuffer::BLOB  => OCIBLobLocator  }
impl_from_lob!{ ColumnBuffer::BFile => OCIBFileLocator }

impl<'a> FromSql<'a> for RowID {
    fn value(row: &'a Row<'a>, col: &mut Column) -> Result<Self> {
        assert_not_null(row, col)?;
        match col.data() {
            ColumnBuffer::Rowid( rowid )  => {
                if rowid::is_initialized(rowid) {
                    let mut res = Descriptor::<OCIRowid>::new(row)?;
                    res.swap(rowid);
                    Ok(RowID::from(res))
                } else {
                    Err(Error::new("already consumed"))
                }
            },
            _ => Err( Error::new("cannot return as row id") )
        }
    }
}

impl<'a, T: FromSql<'a>> FromSql<'a> for Option<T> {
    fn value(row: &'a Row<'a>, col: &mut Column) -> Result<Self> {
        if col.is_null() {
            Ok(None)
        } else {
            let val : T = FromSql::value(row, col)?;
            Ok(Some(val))
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
        let dbuser = std::env::var("DBUSER").expect("user name");
        let dbpass = std::env::var("DBPASS").expect("password");
        let oracle = env()?;
        let session = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = session.prepare("
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
              WHEN name_already_used THEN NULL;
            END;
        ")?;
        stmt.execute(())?;

        let stmt = session.prepare("
            INSERT INTO test_large_object_data (fbin) VALUES (BFileName('MEDIA_DIR',:NAME))
            RETURNING id INTO :ID
        ")?;
        let mut hw_id = 0usize;
        let count = stmt.execute(((":NAME", "hello_world.txt"), (":ID", &mut hw_id)))?;
        assert_eq!(count, 1);
        let mut hs_id = 0usize;
        let count = stmt.execute(((":NAME", "hello_supplemental.txt"), (":ID", &mut hs_id)))?;
        assert_eq!(count, 1);

        let stmt = session.prepare("SELECT fbin FROM test_large_object_data WHERE id IN (:ID1, :ID2) ORDER BY id")?;
        let rows = stmt.query(((":ID1", &hw_id), (":ID2", &hs_id)))?;

        if let Some(row) = rows.next()? {
            let lob : BFile = row.get(0)?;
            assert!(lob.file_exists()?);
            let (dir, name) = lob.file_name()?;
            assert_eq!(dir, "MEDIA_DIR");
            assert_eq!(name, "hello_world.txt");
            assert_eq!(lob.len()?, 28);
        }

        if let Some(row) = rows.next()? {
            let lob : BFile = row.get(0)?;
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
        let lob : BFile = row.get(0)?;
        Ok(lob)
    }

    #[test]
    fn from_rowid() -> Result<()> {
        let dbname = std::env::var("DBNAME").expect("database name");
        let dbuser = std::env::var("DBUSER").expect("user name");
        let dbpass = std::env::var("DBPASS").expect("password");
        let oracle = env()?;
        let session = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = session.prepare("
            SELECT ROWID, manager_id
              FROM hr.employees
             WHERE employee_id = :ID
        ")?;
        if let Some(row) = stmt.query_single((":ID", 107))? {
            let strid : String = row.get(0)?;
            let rowid : RowID = row.get(0)?;
            assert_eq!(rowid.to_string(&session)?, strid);
            let manager_id: u32 = row.get(1)?;
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
        let rowid : RowID = row.get(0)?;
        Ok(rowid)
    }
}
