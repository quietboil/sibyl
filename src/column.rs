use crate::*;
use crate::types::*;
use crate::stmt::Stmt;
use crate::desc::Descriptor;
use libc::c_void;
use std::{
    ptr,
    cell::{ RefCell, Ref }
};

const OCI_ATTR_DATA_SIZE : u32 =  1; // maximum size of the data
const OCI_ATTR_DATA_TYPE : u32 =  2; // the SQL type of the column/argument

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/bind-define-describe-functions.html#GUID-CFE5AA54-DEBC-42D3-8A27-AFF1E7815691
    fn OCIDefineByPos2(
        stmtp:      *mut OCIStmt,
        defnpp:     *mut *mut OCIDefine,
        errhp:      *mut OCIError,
        position:   u32,
        valuep:     *mut c_void,
        value_sz:   i64,
        dty:        u16,
        indp:       *mut c_void,
        rlenp:      *mut u32,
        rcodep:     *mut u16,
        mode:       u32
    ) -> i32;
}

/// Internal representation of a column from the SELECT projection
pub(crate) struct Column {
    col: Descriptor<OCIParam>,
    def: Handle<OCIDefine>,
    val: RefCell<ColumnValue>,
    len: u32,
    ind: i16,
    pos: u16,
}

impl Column {
    pub(crate) fn new(pos: usize, col: Descriptor<OCIParam>) -> Result<Self> {
        let def = Handle::from(ptr::null_mut::<OCIDefine>());
        Ok( Self {
            col, def,
            val: RefCell::new(ColumnValue::Undefined),
            len: 0,
            ind: OCI_IND_NOTNULL,
            pos: pos as u16
        } )
    }

    pub(crate) fn as_ptr(&self) -> *mut OCIParam {
        self.col.get()
    }

    pub(crate) fn borrow_buffer(&self) -> Ref<ColumnValue> {
        self.val.borrow()
    }

    /// The maximum size of the column in bytes.
    /// For example, it returns 22 for NUMBERs.
    fn size(&self, err: *mut OCIError) -> Result<usize> {
        let size = self.col.get_attr::<u16>(OCI_ATTR_DATA_SIZE, err)? as usize;
        Ok( size )
    }

    /// The OCI data type (SQLT) of the column.
    fn data_type(&self, err: *mut OCIError) -> Result<u16> {
        self.col.get_attr::<u16>(OCI_ATTR_DATA_TYPE, err)
    }

    /// Creates `ColumnValue` that will be used to fetch column values
    pub(crate) fn define_output_buffer(&mut self, stmt: &dyn Stmt) -> Result<()> {
        let col_data_type = self.data_type(stmt.err_ptr())?;
        let data_size = match col_data_type {
            SQLT_LNG | SQLT_LBI => stmt.get_max_col_size(),
            _ => self.size(stmt.err_ptr())?
        };
        // SQLT_CLOB | SQLT_BLOB
        let mut val = self.val.get_mut();
        if let ColumnValue::Undefined = val {
            self.val.replace(
                ColumnValue::new(col_data_type, data_size, stmt)?
            );
            val = self.val.get_mut();
        }
        let (output_type, output_buff_ptr, output_buff_size) = val.to_sql_output(data_size);
        let mut def_ptr = self.def.get();
        catch!{stmt.err_ptr() =>
            OCIDefineByPos2(
                stmt.stmt_ptr(), &mut def_ptr, stmt.err_ptr(),
                self.pos as u32, output_buff_ptr, output_buff_size as i64, output_type,
                &mut self.ind as *mut i16 as *mut c_void, &mut self.len, ptr::null_mut::<u16>(),
                OCI_DEFAULT
            )
        }
        self.def.replace(def_ptr);
        Ok(())
    }

    pub(crate) fn drop_output(&mut self, env: *mut OCIEnv, err: *mut OCIError) {
        self.val.get_mut().drop(env, err);
    }

    /// Returns `true` if the last value fetched was NULL.
    pub(crate) fn is_null(&self) -> bool {
        self.ind == OCI_IND_NULL
    }
}

/// Column output buffer
pub enum ColumnValue {
    Undefined,
    Text( *mut varchar::OCIString ),
    CLOB( Descriptor<OCICLobLocator> ),
    Binary( *mut raw::OCIRaw ),
    BLOB( Descriptor<OCIBLobLocator> ),
    BFile( Descriptor<OCIBFileLocator> ),
    Number( Box<number::OCINumber> ),
    Date( date::OCIDate ),
    Timestamp( Descriptor<OCITimestamp> ),
    TimestampTZ( Descriptor<OCITimestampTZ> ),
    TimestampLTZ( Descriptor<OCITimestampLTZ> ),
    IntervalYM( Descriptor<OCIIntervalYearToMonth> ),
    IntervalDS( Descriptor<OCIIntervalDayToSecond> ),
    Float( f32 ),
    Double( f64 ),
    Rowid( RowID ),
    Cursor( Handle<OCIStmt> )
}

impl ColumnValue {
    fn new(data_type: u16, data_size: usize, stmt: &dyn Stmt) -> Result<Self> {
        let val = match data_type {
            SQLT_DAT            => ColumnValue::Date( date::new() ),
            SQLT_TIMESTAMP      => ColumnValue::Timestamp( Descriptor::<OCITimestamp>::new(stmt.env_ptr())? ),
            SQLT_TIMESTAMP_TZ   => ColumnValue::TimestampTZ( Descriptor::<OCITimestampTZ>::new(stmt.env_ptr())? ),
            SQLT_TIMESTAMP_LTZ  => ColumnValue::TimestampLTZ( Descriptor::<OCITimestampLTZ>::new(stmt.env_ptr())? ),
            SQLT_INTERVAL_YM    => ColumnValue::IntervalYM( Descriptor::<OCIIntervalYearToMonth>::new(stmt.env_ptr())? ),
            SQLT_INTERVAL_DS    => ColumnValue::IntervalDS( Descriptor::<OCIIntervalDayToSecond>::new(stmt.env_ptr())? ),
            SQLT_NUM            => ColumnValue::Number( Box::new(number::new()) ),
            SQLT_IBFLOAT        => ColumnValue::Float( 0f32 ),
            SQLT_IBDOUBLE       => ColumnValue::Double( 0f64 ),
            SQLT_BIN | SQLT_LBI => ColumnValue::Binary( raw::new(data_size, stmt.env_ptr(), stmt.err_ptr())? ),
            SQLT_CLOB           => ColumnValue::CLOB( Descriptor::<OCICLobLocator>::new(stmt.env_ptr())? ),
            SQLT_BLOB           => ColumnValue::BLOB( Descriptor::<OCIBLobLocator>::new(stmt.env_ptr())? ),
            SQLT_BFILE          => ColumnValue::BFile( Descriptor::<OCIBFileLocator>::new(stmt.env_ptr())? ),
            SQLT_RDD            => ColumnValue::Rowid( RowID::new(stmt.env_ptr())? ),
            SQLT_RSET           => ColumnValue::Cursor( Handle::<OCIStmt>::new(stmt.env_ptr())? ),
            _ => ColumnValue::Text( varchar::new(data_size, stmt.env_ptr(), stmt.err_ptr())? )
        };
        Ok( val )
    }

    fn drop(&mut self, env: *mut OCIEnv, err: *mut OCIError) {
        match self {
            ColumnValue::Text( mut oci_str ) => {
                varchar::free(&mut oci_str, env, err);
            }
            ColumnValue::Binary( mut oci_raw ) => {
                raw::free(&mut oci_raw, env, err);
            }
            _ => { }
        }
    }
}

impl ToSqlOut for ColumnValue {
    fn to_sql_output(&mut self, col_size: usize) -> (u16, *mut c_void, usize) {
        match self {
            ColumnValue::Text( oci_str )       => (*oci_str).to_sql_output(col_size),
            ColumnValue::Binary( oci_raw )     => (*oci_raw).to_sql_output(col_size),
            ColumnValue::Number( oci_num_box ) => oci_num_box.to_sql_output(col_size),
            ColumnValue::Date( oci_date )      => oci_date.to_sql_output(col_size),
            ColumnValue::Timestamp( ts )       => ts.to_sql_output(col_size),
            ColumnValue::TimestampTZ( ts )     => ts.to_sql_output(col_size),
            ColumnValue::TimestampLTZ( ts )    => ts.to_sql_output(col_size),
            ColumnValue::IntervalYM( int )     => int.to_sql_output(col_size),
            ColumnValue::IntervalDS( int )     => int.to_sql_output(col_size),
            ColumnValue::Float( val )          => val.to_sql_output(col_size),
            ColumnValue::Double( val )         => val.to_sql_output(col_size),
            ColumnValue::CLOB( lob )           => lob.to_sql_output(col_size),
            ColumnValue::BLOB( lob )           => lob.to_sql_output(col_size),
            ColumnValue::BFile( lob )          => lob.to_sql_output(col_size),
            ColumnValue::Rowid( rowid )        => rowid.to_sql_output(col_size),
            ColumnValue::Cursor( handle )      => handle.to_sql_output(col_size),
            ColumnValue::Undefined             => (0, ptr::null_mut::<c_void>(), 0)
        }
    }
}
