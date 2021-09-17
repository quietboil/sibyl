use crate::*;
use crate::types::*;
use crate::stmt::Stmt;
use crate::desc::Descriptor;
use libc::c_void;
use std::ptr;

const OCI_ATTR_DATA_SIZE : u32 =  1; // maximum size of the data
const OCI_ATTR_DATA_TYPE : u32 =  2; // the SQL type of the column/argument

const DEFAULT_LONG_BUFFER_SIZE : usize = 32768;

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/bind-define-describe-functions.html#GUID-74939FB5-919E-4D24-B327-AFB532435061
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

/// Internal representation of a column from SELECT projection
pub struct Column {
    col: Descriptor<OCIParam>,
    def: Handle<OCIDefine>,
    buf: ColumnBuffer,
    /// Length of data fetched
    len: u32,
    /// Output "indicator":
    /// -2 : The length of the item is greater than the length of the output variable; the item has been truncated.
    ///      Unline the case of indicators that are > 0, the original length is longer than the maximum data length
    ///      that can be returned in the i16 indicator variable.
    /// -1 : The selected value is null, and the value of the output variable is unchanged.
    ///  0 : Oracle Database assigned an intact value to the host variable
    /// >0 : The length of the item is greater than the length of the output variable; the item has been truncated.
    ///      The positive value returned in the indicator variable is the actual length before truncation.
    ind: i16,
    pos: u16,
}

impl Column {
    pub(crate) fn new(pos: usize, stmt: *mut OCIStmt, err: *mut OCIError) -> Result<Self> {
        let col = param::get::<OCIParam>(pos as u32, OCI_HTYPE_STMT, stmt as *const c_void, err)?;
        let def = Handle::from(ptr::null_mut::<OCIDefine>());
        Ok( Self {
            col, def,
            buf: ColumnBuffer::Undefined,
            len: 0,
            ind: OCI_IND_NULL,
            pos: pos as u16
        } )
    }

    pub(crate) fn as_ptr(&self) -> *mut OCIParam {
        self.col.get()
    }

    /// Creates `ColumnValue` that will be used to fetch column values
    pub(crate) fn setup_output_buffer(&mut self, stmt: *mut OCIStmt, env: *mut OCIEnv, err: *mut OCIError) -> Result<()> {
        let data_type = self.col.get_attr::<u16>(OCI_ATTR_DATA_TYPE, err)?;
        let data_size = match data_type {
            SQLT_LNG | SQLT_LBI => DEFAULT_LONG_BUFFER_SIZE,
            _ => self.col.get_attr::<u16>(OCI_ATTR_DATA_SIZE, err)? as usize
        };
        self.buf = ColumnBuffer::new(data_type, data_size, env, err)?;
        let (output_type, output_buff_ptr, output_buff_size) = self.buf.get_output_buffer_def(data_size);
        catch!{err =>
            OCIDefineByPos2(
                stmt, self.def.as_ptr(), err,
                self.pos as u32, output_buff_ptr, output_buff_size as i64, output_type,
                &mut self.ind as *mut i16 as *mut c_void, &mut self.len, ptr::null_mut::<u16>(),
                OCI_DEFAULT
            )
        }
        Ok(())
    }

    pub(crate) fn change_buffer_size(&mut self, size: usize, stmt: &dyn Stmt) -> Result<()> {
        let data_type = self.col.get_attr::<u16>(OCI_ATTR_DATA_TYPE, stmt.err_ptr())?;
        if (data_type == SQLT_LNG || data_type == SQLT_LBI) && size > DEFAULT_LONG_BUFFER_SIZE {
            self.buf.resize(size, stmt.env_ptr(), stmt.err_ptr())?;
            let (output_type, output_buff_ptr, output_buff_size) = self.buf.get_output_buffer_def(size);
            catch!{stmt.err_ptr() =>
                OCIDefineByPos2(
                    stmt.stmt_ptr(), self.def.as_ptr(), stmt.err_ptr(),
                    self.pos as u32, output_buff_ptr, output_buff_size as i64, output_type,
                    &mut self.ind as *mut i16 as *mut c_void, &mut self.len, ptr::null_mut::<u16>(),
                    OCI_DEFAULT
                )
            }
        }
        Ok(())
    }

    pub(crate) fn position(&self) -> usize {
        self.pos as usize
    }

    pub(crate) fn get_column_buffer(&self) -> &ColumnBuffer {
        &self.buf
    }

    pub(crate) fn drop_output_buffer(&mut self, env: *mut OCIEnv, err: *mut OCIError) {
        self.buf.drop(env, err);
    }

    /// Returns `true` if the last value fetched was NULL.
    pub(crate) fn is_null(&self) -> bool {
        self.ind == OCI_IND_NULL
    }
}

/// Column output buffer
pub enum ColumnBuffer {
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

impl ColumnBuffer {
    fn new(data_type: u16, data_size: usize, env: *mut OCIEnv, err: *mut OCIError) -> Result<Self> {
        let val = match data_type {
            SQLT_DAT            => ColumnBuffer::Date( date::new() ),
            SQLT_TIMESTAMP      => ColumnBuffer::Timestamp( Descriptor::<OCITimestamp>::new(env)? ),
            SQLT_TIMESTAMP_TZ   => ColumnBuffer::TimestampTZ( Descriptor::<OCITimestampTZ>::new(env)? ),
            SQLT_TIMESTAMP_LTZ  => ColumnBuffer::TimestampLTZ( Descriptor::<OCITimestampLTZ>::new(env)? ),
            SQLT_INTERVAL_YM    => ColumnBuffer::IntervalYM( Descriptor::<OCIIntervalYearToMonth>::new(env)? ),
            SQLT_INTERVAL_DS    => ColumnBuffer::IntervalDS( Descriptor::<OCIIntervalDayToSecond>::new(env)? ),
            SQLT_NUM            => ColumnBuffer::Number( Box::new(number::new()) ),
            SQLT_IBFLOAT        => ColumnBuffer::Float( 0f32 ),
            SQLT_IBDOUBLE       => ColumnBuffer::Double( 0f64 ),
            SQLT_BIN | SQLT_LBI => ColumnBuffer::Binary( raw::new(data_size, env, err)? ),
            SQLT_CLOB           => ColumnBuffer::CLOB( Descriptor::<OCICLobLocator>::new(env)? ),
            SQLT_BLOB           => ColumnBuffer::BLOB( Descriptor::<OCIBLobLocator>::new(env)? ),
            SQLT_BFILE          => ColumnBuffer::BFile( Descriptor::<OCIBFileLocator>::new(env)? ),
            SQLT_RDD            => ColumnBuffer::Rowid( RowID::new(env)? ),
            SQLT_RSET           => ColumnBuffer::Cursor( Handle::<OCIStmt>::new(env)? ),
            _ => ColumnBuffer::Text( varchar::new(data_size, env, err)? )
        };
        Ok( val )
    }

    fn drop(&mut self, env: *mut OCIEnv, err: *mut OCIError) {
        match self {
            ColumnBuffer::Text( mut oci_str ) => {
                varchar::free(&mut oci_str, env, err);
            }
            ColumnBuffer::Binary( mut oci_raw ) => {
                raw::free(&mut oci_raw, env, err);
            }
            _ => { }
        }
    }

    // Returns (output type, pointer to the output buffer, buffer size)
    fn get_output_buffer_def(&mut self, col_size: usize) -> (u16, *mut c_void, usize) {
        use crate::types::{
            number::OCINumber,
            date::OCIDate,
        };
        use std::mem::size_of;
        match self {
            ColumnBuffer::Text( oci_str )       => (SQLT_LVC,            (*oci_str) as *mut c_void,                              col_size + size_of::<u32>()),
            ColumnBuffer::Binary( oci_raw )     => (SQLT_LVB,            (*oci_raw) as *mut c_void,                              col_size + size_of::<u32>()),
            ColumnBuffer::Number( oci_num_box ) => (SQLT_VNU,            oci_num_box.as_mut() as *mut OCINumber as *mut c_void,  size_of::<OCINumber>()),
            ColumnBuffer::Date( oci_date )      => (SQLT_ODT,            oci_date as *mut OCIDate as *mut c_void,                size_of::<OCIDate>()),
            ColumnBuffer::Timestamp( ts )       => (SQLT_TIMESTAMP,      ts.as_ptr() as *mut c_void,                             size_of::<*mut OCIDateTime>()),
            ColumnBuffer::TimestampTZ( ts )     => (SQLT_TIMESTAMP_TZ,   ts.as_ptr() as *mut c_void,                             size_of::<*mut OCIDateTime>()),
            ColumnBuffer::TimestampLTZ( ts )    => (SQLT_TIMESTAMP_LTZ,  ts.as_ptr() as *mut c_void,                             size_of::<*mut OCIDateTime>()),
            ColumnBuffer::IntervalYM( int )     => (SQLT_INTERVAL_YM,    int.as_ptr() as *mut c_void,                            size_of::<*mut OCIInterval>()),
            ColumnBuffer::IntervalDS( int )     => (SQLT_INTERVAL_DS,    int.as_ptr() as *mut c_void,                            size_of::<*mut OCIInterval>()),
            ColumnBuffer::Float( val )          => (SQLT_BFLOAT,         val as *mut f32 as *mut c_void,                         size_of::<f32>()),
            ColumnBuffer::Double( val )         => (SQLT_BDOUBLE,        val as *mut f64 as *mut c_void,                         size_of::<f64>()),
            ColumnBuffer::CLOB( lob )           => (SQLT_CLOB,           lob.as_ptr() as *mut c_void,                            size_of::<*mut OCILobLocator>()),
            ColumnBuffer::BLOB( lob )           => (SQLT_BLOB,           lob.as_ptr() as *mut c_void,                            size_of::<*mut OCILobLocator>()),
            ColumnBuffer::BFile( lob )          => (SQLT_BFILE,          lob.as_ptr() as *mut c_void,                            size_of::<*mut OCILobLocator>()),
            ColumnBuffer::Rowid( rowid )        => (SQLT_RDD,            rowid.as_ptr() as *mut c_void,                          size_of::<*mut OCIRowid>()),
            ColumnBuffer::Cursor( handle )      => (SQLT_RSET,           handle.as_ptr() as *mut c_void,                         0),
            ColumnBuffer::Undefined             => (0,                   ptr::null_mut::<c_void>(),                              0)
        }
    }

    fn resize(&mut self, size: usize, env: *mut OCIEnv, err: *mut OCIError) -> Result<()> {
        match self {
            ColumnBuffer::Text( txt )   => varchar::resize(txt, size, env, err),
            ColumnBuffer::Binary( bin ) => raw::resize(bin, size, env, err),
            _ => Ok(())
        }
    }
}
