//! Oracle OCI FFI

#![allow(dead_code)]

pub(crate) mod ptr;
pub(crate) mod attr;
pub(crate) mod handle;
pub(crate) mod desc;
pub(crate) mod param;
#[cfg(feature="nonblocking")]
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
pub mod futures;

pub(crate) use ptr::Ptr;
pub(crate) use handle::Handle;
pub(crate) use desc::Descriptor;

use libc::{size_t, c_void};

use crate::{Error, Result};

#[repr(align(4))]
pub(crate) struct Aligned<T>(T);

impl<T> Aligned<T> {
    pub(crate) fn new(val: T) -> Self {
        Self(val)
    }
    pub(crate) fn as_mut_ptr(&mut self) -> *mut T {
        &mut self.0 as _
    }
}

macro_rules! impl_from_aligned {
    ($($t:ty),+) => {
        $(
            impl From<Aligned<$t>> for $t {
                fn from(aligned: Aligned<$t>) -> Self {
                    aligned.0
                }
            }
        )+
    };
}

impl_from_aligned![i8, u8, i16, u16];

pub(crate) const OCI_DEFAULT                : u32 = 0;

// OCI Error Codes
pub(crate) const OCI_SUCCESS                : i32 = 0;
pub(crate) const OCI_SUCCESS_WITH_INFO      : i32 = 1;
pub(crate) const OCI_NEED_DATA              : i32 = 99;
pub(crate) const OCI_NO_DATA                : i32 = 100;
pub(crate) const OCI_ERROR                  : i32 = -1;
pub(crate) const OCI_INVALID_HANDLE         : i32 = -2;
pub(crate) const OCI_STILL_EXECUTING        : i32 = -3123;
pub(crate) const OCI_CONTINUE               : i32 = -24200;

// Oracle error codes
pub(crate) const NO_DATA_FOUND              : i32 = 1403;

// Attribute Constants
pub(crate) const OCI_ATTR_ROW_COUNT         : u32 = 9;
pub(crate) const OCI_ATTR_PREFETCH_ROWS     : u32 = 11;
pub(crate) const OCI_ATTR_PARAM_COUNT       : u32 = 18;     // number of columns in the select list
pub(crate) const OCI_ATTR_STMT_TYPE         : u32 = 24;
pub(crate) const OCI_ATTR_STMTCACHESIZE     : u32 = 176;    // size of the stm cache
pub(crate) const OCI_ATTR_BIND_COUNT        : u32 = 190;
pub(crate) const OCI_ATTR_ROWS_FETCHED      : u32 = 197;
pub(crate) const OCI_ATTR_STMT_IS_RETURNING : u32 = 218;
pub(crate) const OCI_ATTR_UB8_ROW_COUNT     : u32 = 457;
pub(crate) const OCI_ATTR_INVISIBLE_COL     : u32 = 461;
pub(crate) const OCI_ATTR_CALL_TIMEOUT      : u32 = 531;

// Handle Types
pub(crate) const OCI_HTYPE_ENV              : u32 = 1;
pub(crate) const OCI_HTYPE_ERROR            : u32 = 2;
pub(crate) const OCI_HTYPE_SVCCTX           : u32 = 3;
pub(crate) const OCI_HTYPE_STMT             : u32 = 4;
pub(crate) const OCI_HTYPE_BIND             : u32 = 5;
pub(crate) const OCI_HTYPE_DEFINE           : u32 = 6;
pub(crate) const OCI_HTYPE_DESCRIBE         : u32 = 7;
pub(crate) const OCI_HTYPE_SERVER           : u32 = 8;
pub(crate) const OCI_HTYPE_SESSION          : u32 = 9;
pub(crate) const OCI_HTYPE_AUTHINFO         : u32 = OCI_HTYPE_SESSION;
pub(crate) const OCI_HTYPE_CPOOL            : u32 = 26;
pub(crate) const OCI_HTYPE_SPOOL            : u32 = 27;

// Handle Definitions
#[repr(C)] pub        struct OCIEnv         { _private: [u8; 0] }
#[repr(C)] pub        struct OCIError       { _private: [u8; 0] }
#[repr(C)] pub(crate) struct OCISvcCtx      { _private: [u8; 0] }
#[repr(C)] pub        struct OCIStmt        { _private: [u8; 0] }
#[repr(C)] pub(crate) struct OCIBind        { _private: [u8; 0] }
#[repr(C)] pub(crate) struct OCIDefine      { _private: [u8; 0] }
#[repr(C)] pub(crate) struct OCIDescribe    { _private: [u8; 0] }
#[repr(C)] pub(crate) struct OCIServer      { _private: [u8; 0] }
#[repr(C)] pub        struct OCISession     { _private: [u8; 0] }
#[repr(C)] pub(crate) struct OCIRaw         { _private: [u8; 0] }

#[repr(C)] pub(crate) struct OCIAuthInfo    { _private: [u8; 0] }
#[repr(C)] pub(crate) struct OCISPool       { _private: [u8; 0] }
#[repr(C)] pub(crate) struct OCICPool       { _private: [u8; 0] }

/// Trait of handles to have their own type
pub(crate) trait HandleType : OCIStruct {
    fn get_type() -> u32;
}

macro_rules! impl_handle_type {
    ($($oci_handle:ty => $id:ident),+) => {
        $(
            impl HandleType for $oci_handle {
                fn get_type() -> u32 { $id }
            }
        )+
    };
}

impl_handle_type!{
    OCIEnv      => OCI_HTYPE_ENV,
    OCIError    => OCI_HTYPE_ERROR,
    OCISvcCtx   => OCI_HTYPE_SVCCTX,
    OCIStmt     => OCI_HTYPE_STMT,
    OCIBind     => OCI_HTYPE_BIND,
    OCIDefine   => OCI_HTYPE_DEFINE,
    OCIDescribe => OCI_HTYPE_DESCRIBE,
    OCIServer   => OCI_HTYPE_SERVER,
    OCISession  => OCI_HTYPE_SESSION,
    OCIAuthInfo => OCI_HTYPE_AUTHINFO,
    OCICPool    => OCI_HTYPE_CPOOL,
    OCISPool    => OCI_HTYPE_SPOOL
}

// Descriptor Types
pub(crate) const OCI_DTYPE_LOB              : u32 = 50;  // lob locator
pub(crate) const OCI_DTYPE_RSET             : u32 = 52;  // result set descriptor
pub(crate) const OCI_DTYPE_PARAM            : u32 = 53;  // a parameter descriptor obtained from ocigparm
pub(crate) const OCI_DTYPE_ROWID            : u32 = 54;  // rowid descriptor
pub(crate) const OCI_DTYPE_FILE             : u32 = 56;  // File Lob locator
pub(crate) const OCI_DTYPE_LOCATOR          : u32 = 61;  // LOB locator
pub(crate) const OCI_DTYPE_INTERVAL_YM      : u32 = 62;  // Interval year month
pub(crate) const OCI_DTYPE_INTERVAL_DS      : u32 = 63;  // Interval day second
pub(crate) const OCI_DTYPE_TIMESTAMP        : u32 = 68;  // Timestamp
pub(crate) const OCI_DTYPE_TIMESTAMP_TZ     : u32 = 69;  // Timestamp with timezone
pub(crate) const OCI_DTYPE_TIMESTAMP_LTZ    : u32 = 70;  // Timestamp with local tz

// Descriptor Definitions
#[repr(C)] pub(crate) struct OCIResult      { _private: [u8; 0] }
#[repr(C)] pub        struct OCILobLocator  { _private: [u8; 0] }
#[repr(C)] pub(crate) struct OCILobRegion   { _private: [u8; 0] }
#[repr(C)] pub(crate) struct OCIParam       { _private: [u8; 0] }
#[repr(C)] pub(crate) struct OCIRowid       { _private: [u8; 0] }
#[repr(C)] pub        struct OCIDateTime    { _private: [u8; 0] }
#[repr(C)] pub        struct OCIInterval    { _private: [u8; 0] }
#[repr(C)] pub(crate) struct OCIString      { _private: [u8; 0] }

// Virtual descriptors
pub struct OCICLobLocator           {}
pub struct OCIBLobLocator           {}
pub struct OCIBFileLocator          {}
pub struct OCITimestamp             {}
pub struct OCITimestampTZ           {}
pub struct OCITimestampLTZ          {}
pub struct OCIIntervalYearToMonth   {}
pub struct OCIIntervalDayToSecond   {}

/// Trait of (bindable) values to have a type
pub trait SqlType {
    /// Returns SQLT type code
    fn sql_type() -> u16;
    /// Returns SQLT code for the NULL of the type
    /// Some types - `RAW` - are picky about SQLT of the NULL.
    fn sql_null_type() -> u16 {
        Self::sql_type()
    }
}

macro_rules! impl_sql_type {
    ($($t:ty),+ => $sqlt:ident) => {
        $(
            impl SqlType for $t {
                fn sql_type() -> u16 {
                    $sqlt
                }
            }
        )+
    };
}
pub(crate) use impl_sql_type;

/// Trait of descriptors to have a type
pub trait DescriptorType : OCIStruct + SqlType {
    type OCIType;
    fn get_type() -> u32;
}

macro_rules! impl_descr_type {
    ($($oci_desc:ident => $id:ident, $sqlt:ident, $ret:ident),+) => {
        $(
            impl SqlType for $oci_desc {
                fn sql_type() -> u16 { $sqlt }
            }
            impl DescriptorType for $oci_desc {
                type OCIType = $ret;
                fn get_type() -> u32 { $id }
            }
        )+
    };
}

impl_descr_type!{
    OCICLobLocator          => OCI_DTYPE_LOB,           SQLT_CLOB,          OCILobLocator,
    OCIBLobLocator          => OCI_DTYPE_LOB,           SQLT_BLOB,          OCILobLocator,
    OCIBFileLocator         => OCI_DTYPE_FILE,          SQLT_BFILE,         OCILobLocator,
    OCIParam                => OCI_DTYPE_PARAM,         SQLT_NON,           OCIParam,
    OCIRowid                => OCI_DTYPE_ROWID,         SQLT_RDD,           OCIRowid,
    OCITimestamp            => OCI_DTYPE_TIMESTAMP,     SQLT_TIMESTAMP,     OCIDateTime,
    OCITimestampTZ          => OCI_DTYPE_TIMESTAMP_TZ,  SQLT_TIMESTAMP_TZ,  OCIDateTime,
    OCITimestampLTZ         => OCI_DTYPE_TIMESTAMP_LTZ, SQLT_TIMESTAMP_LTZ, OCIDateTime,
    OCIIntervalYearToMonth  => OCI_DTYPE_INTERVAL_YM,   SQLT_INTERVAL_YM,   OCIInterval,
    OCIIntervalDayToSecond  => OCI_DTYPE_INTERVAL_DS,   SQLT_INTERVAL_DS,   OCIInterval
}

/// Marker trait for OCI handles and descriptors
pub trait OCIStruct {}

macro_rules! mark_as_oci {
    ($($t:ty),+) => {
        $(
            impl OCIStruct for $t {}
        )+
    };
}

mark_as_oci!(OCIEnv, OCIError, OCISvcCtx, OCIStmt, OCIBind, OCIDefine, OCIDescribe, OCIServer, OCISession, OCIAuthInfo, OCISPool, OCICPool);
mark_as_oci!(OCIResult, OCILobLocator, OCILobRegion, OCIParam, OCIRowid, OCIDateTime, OCIInterval, OCIString, OCIRaw);
mark_as_oci!(OCICLobLocator, OCIBLobLocator, OCIBFileLocator, OCITimestamp, OCITimestampTZ, OCITimestampLTZ, OCIIntervalYearToMonth, OCIIntervalDayToSecond);

/// C mapping of the Oracle NUMBER
#[repr(C)] pub struct OCINumber {
    pub(crate) bytes: [u8; 22]
}

/// C mapping of the Oracle DATE type (SQLT_ODT)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct OCIDate {
    pub(crate) year: i16, // gregorian year: range is -4712 <= year <= 9999
    pub(crate) month: u8, // month: range is 1 <= month <= 12
    pub(crate) day:   u8, // day: range is 1 <= day <= 31
    pub(crate) hour:  u8, // hours: range is 0 <= hours <= 23
    pub(crate) min:   u8, // minutes: range is 0 <= minutes <= 59
    pub(crate) sec:   u8, // seconds: range is 0 <= seconds <= 59
}

mark_as_oci!(OCINumber, OCIDate);

// Data types
pub(crate) const SQLT_CHR               : u16 = 1;   // (ORANET TYPE) character string
pub(crate) const SQLT_NUM               : u16 = 2;   // (ORANET TYPE) oracle numeric
pub(crate) const SQLT_INT               : u16 = 3;   // (ORANET TYPE) integer
pub(crate) const SQLT_FLT               : u16 = 4;   // (ORANET TYPE) Floating point number
pub(crate) const SQLT_STR               : u16 = 5;   // zero terminated string
pub(crate) const SQLT_VNU               : u16 = 6;   // NUM with preceding length byte
pub(crate) const SQLT_PDN               : u16 = 7;   // (ORANET TYPE) Packed Decimal Numeric
pub(crate) const SQLT_LNG               : u16 = 8;   // long
pub(crate) const SQLT_VCS               : u16 = 9;   // Variable character string
pub(crate) const SQLT_NON               : u16 = 10;  // Null/empty PCC Descriptor entry
pub(crate) const SQLT_RID               : u16 = 11;  // rowid
pub(crate) const SQLT_DAT               : u16 = 12;  // date in oracle format
pub(crate) const SQLT_VBI               : u16 = 15;  // binary in VCS format
pub(crate) const SQLT_BFLOAT            : u16 = 21;  // Native Binary float
pub(crate) const SQLT_BDOUBLE           : u16 = 22;  // NAtive binary double
pub(crate) const SQLT_BIN               : u16 = 23;  // binary data(DTYBIN)
pub(crate) const SQLT_LBI               : u16 = 24;  // long binary
pub(crate) const SQLT_UIN               : u16 = 68;  // unsigned integer
pub(crate) const SQLT_SLS               : u16 = 91;  // Display sign leading separate
pub(crate) const SQLT_LVC               : u16 = 94;  // Longer longs (char)
pub(crate) const SQLT_LVB               : u16 = 95;  // Longer long binary
pub(crate) const SQLT_AFC               : u16 = 96;  // Ansi fixed char
pub(crate) const SQLT_AVC               : u16 = 97;  // Ansi Var char
pub(crate) const SQLT_IBFLOAT           : u16 = 100; // binary float canonical
pub(crate) const SQLT_IBDOUBLE          : u16 = 101; // binary double canonical
pub(crate) const SQLT_CUR               : u16 = 102; // cursor  type
pub(crate) const SQLT_RDD               : u16 = 104; // rowid descriptor
pub(crate) const SQLT_LAB               : u16 = 105; // label type
pub(crate) const SQLT_OSL               : u16 = 106; // oslabel type

pub(crate) const SQLT_NTY               : u16 = 108; // named object type, a.k.a. user-defined type
pub(crate) const SQLT_REF               : u16 = 110; // ref type
pub(crate) const SQLT_CLOB              : u16 = 112; // character lob
pub(crate) const SQLT_BLOB              : u16 = 113; // binary lob
pub(crate) const SQLT_BFILE             : u16 = 114; // binary file lob
pub(crate) const SQLT_CFILE             : u16 = 115; // character file lob
pub(crate) const SQLT_RSET              : u16 = 116; // result set type
pub(crate) const SQLT_NCO               : u16 = 122; // named collection type (varray or nested table)
pub(crate) const SQLT_VST               : u16 = 155; // OCIString type
pub(crate) const SQLT_ODT               : u16 = 156; // OCIDate type

// datetimes and intervals
pub(crate) const SQLT_DATE              : u16 = 184; // ANSI Date
pub(crate) const SQLT_TIME              : u16 = 185; // TIME
pub(crate) const SQLT_TIME_TZ           : u16 = 186; // TIME WITH TIME ZONE
pub(crate) const SQLT_TIMESTAMP         : u16 = 187; // TIMESTAMP
pub(crate) const SQLT_TIMESTAMP_TZ      : u16 = 188; // TIMESTAMP WITH TIME ZONE
pub(crate) const SQLT_INTERVAL_YM       : u16 = 189; // INTERVAL YEAR TO MONTH
pub(crate) const SQLT_INTERVAL_DS       : u16 = 190; // INTERVAL DAY TO SECOND
pub(crate) const SQLT_TIMESTAMP_LTZ     : u16 = 232; // TIMESTAMP WITH LOCAL TZ

pub(crate) const SQLT_PNTY              : u16 = 241; // pl/sql representation of named types

// some pl/sql specific types
pub(crate) const SQLT_REC               : u16 = 250; // pl/sql 'record' (or %rowtype)
pub(crate) const SQLT_TAB               : u16 = 251; // pl/sql 'indexed table'
pub(crate) const SQLT_BOL               : u16 = 252; // pl/sql 'boolean'

// Null indicator information
pub(crate) const OCI_IND_NOTNULL        : i16 = 0;
pub(crate) const OCI_IND_NULL           : i16 = -1;

// char set "form" information
pub(crate) const SQLCS_IMPLICIT         : u8 = 1;
pub(crate) const SQLCS_NCHAR            : u8 = 2;

// OBJECT Duration
pub(crate) const OCI_DURATION_SESSION   : u16 = 10;
pub(crate) const OCI_DURATION_STATEMENT : u16 = 13;

// Character Sets
pub(crate) const AL32UTF8               : u16 = 873;
pub(crate) const UTF8                   : u16 = 871;

// Initialization Modes
pub(crate) const OCI_THREADED : u32 = 1;
pub(crate) const OCI_OBJECT   : u32 = 2;

pub(crate) const OCI_ATTR_CACHE_OPT_SIZE    : u32 = 34;
pub(crate) const OCI_ATTR_CACHE_MAX_SIZE    : u32 = 35;
pub(crate) const OCI_ATTR_ENV_NLS_LANGUAGE  : u32 = 424;
pub(crate) const OCI_ATTR_ENV_NLS_TERRITORY : u32 = 425;

// Credential Types
pub(crate) const OCI_CRED_RDBMS    : u32 = 1;
pub(crate) const OCI_CRED_EXT      : u32 = 2;

// OCISessionPoolCreate Modes
pub(crate) const OCI_SPC_REINITIALIZE   : u32 = 0x0001; // Reinitialize the session pool
pub(crate) const OCI_SPC_HOMOGENEOUS    : u32 = 0x0002; // Session pool is homogeneneous
pub(crate) const OCI_SPC_STMTCACHE      : u32 = 0x0004; // Session pool has stmt cache
pub(crate) const OCI_SPC_NO_RLB         : u32 = 0x0008; // Do not enable Runtime load balancing

// OCISessionPoolDestroy Modes
pub(crate) const OCI_SPD_FORCE          : u32 = 0x0001; // Force the sessions to terminate. Even if there are some busy sessions close them.

// ATTR Values for Session Pool
pub(crate) const OCI_SPOOL_ATTRVAL_WAIT     : u8 = 0; // block till you get a session
pub(crate) const OCI_SPOOL_ATTRVAL_NOWAIT   : u8 = 1; // error out if no session avaliable
pub(crate) const OCI_SPOOL_ATTRVAL_FORCEGET : u8 = 2; // get session even if max is exceeded
pub(crate) const OCI_SPOOL_ATTRVAL_TIMEDWAIT: u8 = 3; // wait for specified timeout if pool is maxed out

// OCISessionGet Modes
pub(crate) const OCI_SESSGET_SPOOL          : u32 = 0x0001;
pub(crate) const OCI_SESSGET_STMTCACHE      : u32 = 0x0004;
pub(crate) const OCI_SESSGET_SPOOL_MATCHANY : u32 = 0x0020;
pub(crate) const OCI_SESSGET_PURITY_NEW     : u32 = 0x0040;
pub(crate) const OCI_SESSGET_PURITY_SELF    : u32 = 0x0080;
pub(crate) const OCI_SESSGET_CPOOL          : u32 = 0x0200;

// Server Handle Attribute Values
// const OCI_SERVER_NOT_CONNECTED  : u32 = 0;
pub(crate) const OCI_SERVER_NORMAL : u32 = 1;
pub(crate) const OCI_ATTR_NONBLOCKING_MODE  : u32 = 3;
pub(crate) const OCI_ATTR_SERVER            : u32 = 6;
pub(crate) const OCI_ATTR_SESSION           : u32 = 7;
pub(crate) const OCI_ATTR_ROWID             : u32 = 19;
pub(crate) const OCI_ATTR_USERNAME          : u32 = 22;
pub(crate) const OCI_ATTR_PASSWORD          : u32 = 23;
pub(crate) const OCI_ATTR_LOBEMPTY          : u32 = 45;
pub(crate) const OCI_ATTR_SERVER_STATUS     : u32 = 143;
pub(crate) const OCI_ATTR_CURRENT_SCHEMA    : u32 = 224;
pub(crate) const OCI_ATTR_CLIENT_IDENTIFIER : u32 = 278;
pub(crate) const OCI_ATTR_MODULE            : u32 = 366;
pub(crate) const OCI_ATTR_ACTION            : u32 = 367;
pub(crate) const OCI_ATTR_CLIENT_INFO       : u32 = 368;
pub(crate) const OCI_ATTR_COLLECT_CALL_TIME : u32 = 369;
pub(crate) const OCI_ATTR_CALL_TIME         : u32 = 370;
pub(crate) const OCI_ATTR_DRIVER_NAME       : u32 = 424;
pub(crate) const OCI_ATTR_DEFAULT_LOBPREFETCH_SIZE : u32 = 438;
pub(crate) const OCI_ATTR_LOB_REMOTE        : u32 = 520;
pub(crate) const OCI_ATTR_LOB_TYPE          : u32 = 591;

pub(crate) const OCI_ATTR_SPOOL_STMTCACHESIZE           : u32 = 208; // Stmt cache size of pool
pub(crate) const OCI_ATTR_SPOOL_TIMEOUT                 : u32 = 308; // session timeout
pub(crate) const OCI_ATTR_SPOOL_GETMODE                 : u32 = 309; // session get mode
pub(crate) const OCI_ATTR_SPOOL_BUSY_COUNT              : u32 = 310; // busy session count
pub(crate) const OCI_ATTR_SPOOL_OPEN_COUNT              : u32 = 311; // open session count
pub(crate) const OCI_ATTR_SPOOL_AUTH                    : u32 = 460; // Auth handle on pool handle
pub(crate) const OCI_ATTR_SPOOL_MAX_LIFETIME_SESSION    : u32 = 490; // Max Lifetime for session
pub(crate) const OCI_ATTR_SPOOL_WAIT_TIMEOUT            : u32 = 506;
pub(crate) const OCI_ATTR_SPOOL_MAX_USE_SESSION         : u32 = 580;

// Connection Pool Attributes
pub(crate) const OCI_ATTR_CONN_NOWAIT       : u32 = 178;
pub(crate) const OCI_ATTR_CONN_BUSY_COUNT   : u32 = 179;
pub(crate) const OCI_ATTR_CONN_OPEN_COUNT   : u32 = 180;
pub(crate) const OCI_ATTR_CONN_TIMEOUT      : u32 = 181;
pub(crate) const OCI_ATTR_STMT_STATE        : u32 = 182;
pub(crate) const OCI_ATTR_CONN_MIN          : u32 = 183;
pub(crate) const OCI_ATTR_CONN_MAX          : u32 = 184;
pub(crate) const OCI_ATTR_CONN_INCR         : u32 = 185;


pub(crate) const OCI_ERROR_MAXMSG_SIZE      : usize = 3072;

pub(crate) const OCI_FETCH_NEXT             : u16 = 2;

pub(crate) const OCI_TEMP_BLOB              : u8 = 1;
pub(crate) const OCI_TEMP_CLOB              : u8 = 2;

pub(crate) const OCI_FILE_READONLY          : u8 = 1;
pub(crate) const OCI_LOB_READONLY           : u8 = 1;
pub(crate) const OCI_LOB_READWRITE          : u8 = 2;

pub(crate) const OCI_ONE_PIECE              : u8 = 0;
pub(crate) const OCI_FIRST_PIECE            : u8 = 1;
pub(crate) const OCI_NEXT_PIECE             : u8 = 2;
pub(crate) const OCI_LAST_PIECE             : u8 = 3;

pub(crate) const OCI_LOB_CONTENTTYPE_MAXSIZE    : usize = 128;

// Parsing Syntax Types
pub(crate) const OCI_NTV_SYNTAX   : u32 = 1;

// Statement Types
// pub(crate) const OCI_STMT_UNKNOWN : u16 = 0;
pub(crate) const OCI_STMT_SELECT  : u16 = 1;
// pub(crate) const OCI_STMT_UPDATE  : u16 = 2;
// pub(crate) const OCI_STMT_DELETE  : u16 = 3;
// pub(crate) const OCI_STMT_INSERT  : u16 = 4;
// pub(crate) const OCI_STMT_CREATE  : u16 = 5;
// pub(crate) const OCI_STMT_DROP    : u16 = 6;
// pub(crate) const OCI_STMT_ALTER   : u16 = 7;
// pub(crate) const OCI_STMT_BEGIN   : u16 = 8;
// pub(crate) const OCI_STMT_DECLARE : u16 = 9;
// pub(crate) const OCI_STMT_CALL    : u16 = 10;
// pub(crate) const OCI_STMT_MERGE   : u16 = 16;

// Attributes common to Columns and Stored Procs
pub(crate) const OCI_ATTR_DATA_SIZE         : u32 =  1; // maximum size of the data
pub(crate) const OCI_ATTR_DATA_TYPE         : u32 =  2; // the SQL type of the column/argument
// pub(crate) const OCI_ATTR_DISP_SIZE         : u32 =  3; // the display size
pub(crate) const OCI_ATTR_NAME              : u32 =  4; // the name of the column/argument
pub(crate) const OCI_ATTR_PRECISION         : u32 =  5; // precision if number type
pub(crate) const OCI_ATTR_SCALE             : u32 =  6; // scale if number type
pub(crate) const OCI_ATTR_IS_NULL           : u32 =  7; // is it null ?
pub(crate) const OCI_ATTR_TYPE_NAME         : u32 =  8; // name of the named data type or a package name for package private types
pub(crate) const OCI_ATTR_SCHEMA_NAME       : u32 =  9; // the schema name
// pub(crate) const OCI_ATTR_SUB_NAME          : u32 = 10; // type name if package private type
// pub(crate) const OCI_ATTR_POSITION          : u32 = 11; // relative position of col/arg in the list of cols/args
// pub(crate) const OCI_ATTR_PACKAGE_NAME      : u32 = 12; // package name of package type
pub(crate) const OCI_ATTR_CHARSET_FORM      : u32 = 32;
pub(crate) const OCI_ATTR_COL_PROPERTIES    : u32 = 104;
pub(crate) const OCI_ATTR_CHAR_SIZE         : u32 = 286;

// Flags coresponding to the column properties
pub(crate) const OCI_ATTR_COL_PROPERTY_IS_IDENTITY             : u8 = 0x01;
pub(crate) const OCI_ATTR_COL_PROPERTY_IS_GEN_ALWAYS           : u8 = 0x02;
pub(crate) const OCI_ATTR_COL_PROPERTY_IS_GEN_BY_DEF_ON_NULL   : u8 = 0x04;

/// Character set form
#[derive(Debug)]
pub enum CharSetForm {
    Undefined = 0,
    Implicit = 1,
    NChar = 2
}

/// LOB cache control flags
pub enum Cache {
    No  = 0,
    Yes = 1,
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/handle-and-descriptor-functions.html#GUID-C5BF55F7-A110-4CB5-9663-5056590F12B5
    fn OCIHandleAlloc(
        parenth:    *const OCIEnv,
        hndlpp:     *mut *mut c_void,
        hndl_type:  u32,
        xtramem_sz: size_t,
        usrmempp:   *const c_void
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/handle-and-descriptor-functions.html#GUID-E87E9F91-D3DC-4F35-BE7C-F1EFBFEEBA0A
    fn OCIHandleFree(
        hndlp:      *const c_void,
        hnd_type:   u32
    ) -> i32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/handle-and-descriptor-functions.html#GUID-E9EF2766-E078-49A7-B1D1-738E4BA4814F
    fn OCIDescriptorAlloc(
        parenth:    *const OCIEnv,
        descpp:     *mut *mut c_void,
        desc_type:  u32,
        xtramem_sz: size_t,
        usrmempp:   *const c_void
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/handle-and-descriptor-functions.html#GUID-A32BF051-3DC1-491C-AAFD-A46034DD1629
    fn OCIDescriptorFree(
        descp:      *mut c_void,
        desc_type:  u32
    ) -> i32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/handle-and-descriptor-functions.html#GUID-FA199A99-4D7A-42C2-BB0A-C20047B95DF9
    fn OCIAttrGet(
        trgthndlp:  *const c_void,
        trghndltyp: u32,
        attributep: *mut c_void,
        sizep:      *const u32,
        attrtype:   u32,
        errhp:      *const OCIError
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/handle-and-descriptor-functions.html#GUID-3741D7BD-7652-4D7A-8813-AC2AEA8D3B03
    fn OCIAttrSet(
        trgthndlp:  *const c_void,
        trghndltyp: u32,
        attributep: *const c_void,
        size:       u32,
        attrtype:   u32,
        errhp:      *const OCIError
    ) -> i32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/handle-and-descriptor-functions.html#GUID-35D2FF91-139B-4A5C-97C8-8BC29866CCA4
    fn OCIParamGet(
        hndlp:      *const c_void,
        htype:      u32,
        errhp:      *const OCIError,
        descr:      *mut *mut c_void,
        pos:        u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/handle-and-descriptor-functions.html#GUID-280CF9E5-3537-4785-9AFA-4E63DE29A266
    // fn OCIParamSet(
    //     hndlp:      *const c_void,
    //     htype:      u32,
    //     errhp:      *const OCIError,
    //     descr:      *const c_void,
    //     dtype:      u32,
    //     pos:        u32
    // ) -> i32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/miscellaneous-functions.html#GUID-83879DA9-768D-4ED6-AFFE-F9D216E4D9B8
    fn OCIClientVersion(
        feature_release:         *mut i32,
        release_update:          *mut i32,
        release_update_revision: *mut i32,
        increment:               *mut i32,
        ext:                     *mut i32,
    );

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/miscellaneous-functions.html#GUID-4B99087C-74F6-498A-8310-D6645172390A
    pub(crate) fn OCIErrorGet(
        hndlp:      *const c_void,
        recordno:   u32,
        sqlstate:   *const c_void,
        errcodep:   *const i32,
        bufp:       *mut u8,
        bufsiz:     u32,
        hnd_type:   u32,
    ) -> i32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/connect-authorize-and-initialize-functions.html#GUID-0B6911A9-4B46-476C-BC5E-B87581666CD9
    pub(crate) fn OCIEnvNlsCreate(
        envhpp:     *mut *mut  OCIEnv,
        mode:       u32,
        ctxp:       *const c_void,
        malocfp:    *const c_void,
        ralocfp:    *const c_void,
        mfreefp:    *const c_void,
        xtramemsz:  size_t,
        usrmempp:   *const c_void,
        charset:    u16,
        ncharset:   u16
    ) -> i32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/connect-authorize-and-initialize-functions.html#GUID-B6291228-DA2F-4CE9-870A-F94243141757
    fn OCIServerAttach(
        srvhp:      *const OCIServer,
        errhp:      *const OCIError,
        dblink:     *const u8,
        dblink_len: u32,
        mode:       u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/connect-authorize-and-initialize-functions.html#GUID-402B540A-05FF-464B-B9C8-B2E7B4ABD564
    fn OCIServerDetach(
        srvhp:      *const OCIServer,
        errhp:      *const OCIError,
        mode:       u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/connect-authorize-and-initialize-functions.html#GUID-31B1FDB3-056E-4AF9-9B89-8DA6AA156947
    fn OCISessionBegin(
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        userhp:     *const OCISession,
        credt:      u32,
        mode:       u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/connect-authorize-and-initialize-functions.html#GUID-2AE88BDC-2C44-4958-B26A-434B0407F06F
    fn OCISessionEnd(
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        userhp:     *const OCISession,
        mode:       u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/connect-authorize-and-initialize-functions.html#GUID-7E5A69F2-0268-4655-845D-A7662902FAA2
    fn OCIConnectionPoolCreate (
        envhp:          *const OCIEnv,
        errhp:          *const OCIError,
        cpoolhp:        *const OCICPool,
        pool_name:      *mut *const u8,
        pool_name_len:  *const u32,
        dblink:         *const u8,
        dblink_len:     u32,
        conn_min:       u32,
        conn_max:       u32,
        conn_incr:      u32,
        username:       *const u8,
        username_len:   u32,
        password:       *const u8,
        password_len:   u32,
        mode:           u32,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/connect-authorize-and-initialize-functions.html#GUID-46720C8F-0A9F-4300-B6C4-4E47875A95C2
    fn OCIConnectionPoolDestroy  (
        spoolhp:        *const OCICPool,
        errhp:          *const OCIError,
        mode:           u32,
    ) -> i32;


    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/connect-authorize-and-initialize-functions.html#GUID-1E929CFB-9D96-4E8E-9F24-904AD539E555
    fn OCISessionPoolCreate (
        envhp:          *const OCIEnv,
        errhp:          *const OCIError,
        spoolhp:        *const OCISPool,
        pool_name:      *mut *const u8,
        pool_name_len:  *const u32,
        conn_str:       *const u8,
        conn_str_len:   u32,
        sess_min:       u32,
        sess_max:       u32,
        sess_incr:      u32,
        userid:         *const u8,
        userid_len:     u32,
        password:       *const u8,
        password_len:   u32,
        mode:           u32,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/connect-authorize-and-initialize-functions.html#GUID-2797C90C-C7AC-47FB-B1C2-CE41B743FB5C
    fn OCISessionPoolDestroy  (
        spoolhp:        *const OCISPool,
        errhp:          *const OCIError,
        mode:           u32,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/connect-authorize-and-initialize-functions.html#GUID-890DFBC4-718B-4339-A0EA-6226A25B8241
    fn OCISessionGet(
        envhp:      *const OCIEnv,
        errhp:      *const OCIError,
        svchp:      *mut *mut OCISvcCtx,
        authinfop:  *const OCIAuthInfo,
        dbname:     *const u8,
        dbname_len: u32,
        taginfo:    *const u8,
        taginfolen: u32,
        rettags:    *mut *const u8,
        rettagslen: *const u32,
        found:      *const u8,
        mode:       u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/connect-authorize-and-initialize-functions.html#GUID-DAAECC99-A432-48B5-AC33-0868C2FE762D
    fn OCISessionRelease(
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        tag:        *const u8,
        taglen:     u32,
        mode:       u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/transaction-functions.html#GUID-DDAE3122-8769-4A30-8D78-EB2A3CCF77D4
    fn OCITransCommit(
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        flags:      u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/transaction-functions.html#GUID-06EF9A0A-01A3-40CE-A0B7-DF0504A93366
    fn OCITransRollback(
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        flags:      u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/miscellaneous-functions.html#GUID-033BF96D-D88D-4F18-909A-3AB7C2F6C70F
    fn OCIPing(
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        mode:       u32
    ) -> i32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/statement-functions.html#GUID-E6C1DC67-D464-4D2A-9F19-737423D31779
    fn OCIStmtPrepare2(
        svchp:      *const OCISvcCtx,
        stmthp:     *mut *mut OCIStmt,
        errhp:      *const OCIError,
        stmttext:   *const u8,
        stmt_len:   u32,
        key:        *const u8,
        keylen:     u32,
        language:   u32,
        mode:       u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/statement-functions.html#GUID-256034CE-2ADB-4BE5-BC8D-748307F2EA8E
    fn OCIStmtRelease(
        stmtp:      *const OCIStmt,
        errhp:      *const OCIError,
        key:        *const u8,
        keylen:     u32,
        mode:       u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/bind-define-describe-functions.html#GUID-87D50C09-F18D-45BB-A8AF-1E6AFEC6FE2E
    fn OCIStmtGetBindInfo(
        stmtp:      *const OCIStmt,
        errhp:      *const OCIError,
        size:       u32,
        startloc:   u32,
        found:      *const i32,
        bvnp:       *mut *mut u8,
        bvnl:       *const u8,
        invp:       *mut *mut u8,
        invl:       *const u8,
        dupl:       *const u8,
        hndl:       *mut *mut OCIBind
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/bind-define-describe-functions.html#GUID-CD63DF78-2178-4727-A896-B9673C4A37F0
    // fn OCIBindByName2(
    //     stmtp:      *const OCIStmt,
    //     bindpp:     *mut *mut OCIBind,
    //     errhp:      *const OCIError,
    //     namep:      *const u8,
    //     name_len:   i32,
    //     valuep:     *const c_void,
    //     value_sz:   i64,
    //     dty:        u16,
    //     indp:       *const c_void,
    //     alenp:      *const u32,
    //     rcodep:     *const u16,
    //     maxarr_len: u32,
    //     curelep:    *const u32,
    //     mode:       u32
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/bind-define-describe-functions.html#GUID-5C505821-323D-473D-825B-448C8D9A6702
    fn OCIBindByPos2(
        stmtp:      *const OCIStmt,
        bindpp:     *mut *mut OCIBind,
        errhp:      *const OCIError,
        position:   u32,
        valuep:     *const c_void,
        value_sz:   i64,
        dty:        u16,
        indp:       *const i16,
        alenp:      *const u32,
        rcodep:     *const u16,
        maxarr_len: u32,
        curelep:    *const u32,
        mode:       u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/bind-define-describe-functions.html#GUID-030270CB-346A-412E-B3B3-556DD6947BE2
    // fn OCIBindDynamic(
    //     bindp:      *const OCIBind,
    //     errhp:      *const OCIError,
    //     ictxp:      *const c_void,
    //     icbfp:      OCICallbackInBind,
    //     octxp:      *const c_void,
    //     ocbfp:      OCICallbackOutBind
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/statement-functions.html#GUID-98B26708-3E02-45C0-8258-5D5544F32BE9
    fn OCIStmtExecute(
        svchp:      *const OCISvcCtx,
        stmtp:      *const OCIStmt,
        errhp:      *const OCIError,
        iters:      u32,
        rowoff:     u32,
        snap_in:    *const c_void,  // *const OCISnapshot
        snap_out:   *const c_void,    // *const OCISnapshot
        mode:       u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/statement-functions.html#GUID-60B998F9-F213-43BA-AB84-76F1EC6A6687
    pub(crate) fn OCIStmtGetNextResult(
        stmtp:      *const OCIStmt,
        errhp:      *const OCIError,
        result:     *mut *mut OCIStmt,
        rtype:      *mut u32,
        mode:       u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/statement-functions.html#GUID-DF585B90-58BA-45FC-B7CE-6F7F987C03B9
    pub(crate) fn OCIStmtFetch2(
        stmtp:      *const OCIStmt,
        errhp:      *const OCIError,
        nrows:      u32,
        orient:     u16,
        offset:     i16,
        mode:       u32
    ) -> i32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/bind-define-describe-functions.html#GUID-74939FB5-919E-4D24-B327-AFB532435061
    fn OCIDefineByPos2(
        stmtp:      *const OCIStmt,
        defnpp:     *mut *mut OCIDefine,
        errhp:      *const OCIError,
        position:   u32,
        valuep:     *const c_void,
        value_sz:   i64,
        dty:        u16,
        indp:       *const i16,
        rlenp:      *const u32,
        rcodep:     *const u16,
        mode:       u32
    ) -> i32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/miscellaneous-functions.html#GUID-064F2680-453A-40D1-9C36-518F1E2B31DF
    fn OCIRowidToChar(
        desc:   *const OCIRowid,
        text:   *const u8,
        size:   *const u16,
        err:    *const OCIError,
    ) -> i32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-84EA4A66-27BF-470C-8464-3DE31937702A
    // fn OCIDurationBegin(
    //     envhp:      *const OCIEnv,
    //     errhp:      *const OCIError,
    //     svchp:      *const OCISvcCtx,
    //     parent:     u16,
    //     duration:   *const u16
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-AABC3F29-C91B-45A7-AF1E-D486C12E4962
    // fn OCIDurationEnd(
    //     envhp:      *const OCIEnv,
    //     errhp:      *const OCIError,
    //     svchp:      *const OCISvcCtx,
    //     duration:   u16
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-5B43FC88-A649-4764-8C1E-6D792F05F7CE
    fn OCILobAppend(
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        dst:        *const OCILobLocator,
        src:        *const OCILobLocator,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-9B25760D-649E-4B83-A0AA-8C4F3C479BC8
    fn OCILobCharSetForm(
        envhp:      *const OCIEnv,
        errhp:      *const OCIError,
        src:        *const OCILobLocator,
        csform:     *const u8
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-A243691D-8180-4AF6-AA6E-DF9333F8258B
    fn OCILobCharSetId(
        envhp:      *const OCIEnv,
        errhp:      *const OCIError,
        src:        *const OCILobLocator,
        csid:       *const u16
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-CBEB9238-6B47-4A08-8C8D-FC2E5ED56557
    pub(crate) fn OCILobClose(
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        loc:        *const OCILobLocator,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-404C8A50-516F-4DFD-939D-646A232AF7DF
    fn OCILobCopy2(
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        dst:        *const OCILobLocator,
        src:        *const OCILobLocator,
        amount:     u64,
        dst_off:    u64,
        src_off:    u64,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-63F75EC5-EB14-4E25-B593-270FF814615A
    fn OCILobCreateTemporary(
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        loc:        *const OCILobLocator,
        csid:       u16,
        csfrm:      u8,
        lob_type:   u8,
        cache:      u8,
        duration:   u16,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-264797B2-B3EA-4F6D-9A0E-BF8A4DDA13FA
    fn OCILobErase2(
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        loc:        *const OCILobLocator,
        amount:     *const u64,
        offset:     u64,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-40AFA7A3-3A24-4DF7-A719-AECA7C1F522A
    pub(crate) fn OCILobFileClose(
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        filep:      *const OCILobLocator,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-977F905D-DAFB-4D88-8FE0-7A345837B147
    fn OCILobFileExists(
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        filep:      *const OCILobLocator,
        flag:       *const u8
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-BF637A34-B18A-47EE-A060-93C4E79D1813
    fn OCILobFileGetName(
        envhp:      *const OCIEnv,
        errhp:      *const OCIError,
        loc:        *const OCILobLocator,
        dir:        *const u8,
        dir_len:    *const u16,
        filename:   *const u8,
        name_len:   *const u16,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-A662166C-DC74-40B4-9BFA-8D3ED216FDE7
    fn OCILobFileIsOpen(
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        filep:      *const OCILobLocator,
        flag:       *const u8
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-2E933BBA-BCE3-41F2-B8A2-4F9485F0BCB0
    fn OCILobFileOpen(
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        filep:      *const OCILobLocator,
        mode:       u8
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-507AC0EF-4CAB-437E-BB94-1FD77EDC1B5C
    fn OCILobFileSetName(
        envhp:      *const OCIEnv,
        errhp:      *const OCIError,
        filepp:     *mut *mut OCILobLocator,
        dir:        *const u8,
        dir_len:    u16,
        filename:   *const u8,
        name_len:   u16,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-E0FBF017-1B08-410C-9E53-F6E14008813A
    pub(crate) fn OCILobFreeTemporary(
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        loc:        *const OCILobLocator,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-ABB71585-172E-4F3E-A0CF-F70D709F2072
    fn OCILobGetChunkSize(
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        loc:        *const OCILobLocator,
        size:       *const u32,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-D62200EF-FA60-4788-950F-0C0686D807FD
    fn OCILobGetContentType(
        envhp:      *const OCIEnv,
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        loc:        *const OCILobLocator,
        ctx_type:   *mut u8,
        len:        *mut u32,
        mode:       u32
    )-> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-9BC0A78A-37CB-432F-AE2B-22C905608C4C
    fn OCILobGetLength2(
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        loc:        *const OCILobLocator,
        len:        *const u64,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-5142710F-03AD-43D5-BBAB-6732B874E52E
    fn OCILobIsEqual(
        envhp:      *const OCIEnv,
        loc1:       *const OCILobLocator,
        loc2:       *const OCILobLocator,
        flag:       *const u8,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-FFF883CE-3B99-4319-A81C-A11F8740209E
    pub(crate) fn OCILobIsOpen(
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        loc:        *const OCILobLocator,
        flag:       *const u8,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-071D8134-F9E7-4C5A-8E63-E90831FA7AC3
    pub(crate) fn OCILobIsTemporary(
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        loc:        *const OCILobLocator,
        flag:       *const u8,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-DA1CD18B-7044-4E40-B1F4-4FCC1FCAB6C4
    fn OCILobLoadFromFile2(
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        dst:        *const OCILobLocator,
        src:        *const OCILobLocator,
        amount:     u64,
        dst_off:    u64,
        src_off:    u64,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-F7887376-4B3C-430C-94A3-11FE96E26627
    fn OCILobLocatorAssign(
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        src:        *const OCILobLocator,
        dst:        *mut *mut OCILobLocator,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-4CA17A83-795F-43B2-8B76-611B13E4C8DE
    fn OCILobLocatorIsInit(
        envhp:      *const OCIEnv,
        errhp:      *const OCIError,
        src:        *const OCILobLocator,
        flag:       *const u8,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-B007A3C7-999B-4AD7-8BF7-C6D14572F470
    fn OCILobOpen(
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        loc:        *const OCILobLocator,
        mode:       u8,
    ) -> i32;

    // https://docs.oracle.com/cd/B19306_01/appdev.102/b14250/oci16msc002.htm#sthref3010
    fn OCILobRead(
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        loc:        *const OCILobLocator,
        amtp:       *mut u32,
        offset:     u32,
        buf:        *mut u8,
        buf_len:    u32,
        ctx:        *mut c_void,
        read_cb:    *const c_void,
        csid:       u16,
        csfrm:      u8,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-6AC6E6DA-236B-4BF9-942F-9FCC4178FEDA
    fn OCILobRead2(
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        loc:        *const OCILobLocator,
        byte_cnt:   *mut u64,
        char_cnt:   *mut u64,
        offset:     u64,
        buf:        *mut u8,
        buf_len:    u64,
        piece:      u8,
        ctx:        *mut c_void,
        read_cb:    *const c_void,
        csid:       u16,
        csfrm:      u8,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-789C0971-76D5-4439-9379-E3DCE7885528
    fn OCILobSetContentType(
        envhp:      *const OCIEnv,
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        loc:        *const OCILobLocator,
        ctype:      *const u8,
        len:        u32,
        mode:       u32,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-ABDB1543-1782-4216-AD80-55FA82CFF733
    fn OCILobTrim2(
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        loc:        *const OCILobLocator,
        len:        u64,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-77F056CA-9EEE-4550-8A8E-0155DF994DBE
    fn OCILobWrite2(
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        loc:        *const OCILobLocator,
        byte_cnt:   *mut u64,
        char_cnt:   *mut u64,
        offset:     u64,
        buf:        *const u8,
        buf_len:    u64,
        piece:      u8,
        ctx:        *const c_void,
        write_cb:   *const c_void,
        csid:       u16,
        csfrm:      u8,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-87D3275A-B042-4991-B261-AB531BB83CA2
    fn OCILobWriteAppend2(
        svchp:      *const OCISvcCtx,
        errhp:      *const OCIError,
        loc:        *const OCILobLocator,
        byte_cnt:   *mut u64,
        char_cnt:   *mut u64,
        buf:        *const u8,
        buf_len:    u64,
        piece:      u8,
        ctx:        *const c_void,
        write_cb:   *const c_void,
        csid:       u16,
        csfrm:      u8,
    ) -> i32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-E0890180-8714-4243-A585-0FD21EB05CA9
    fn OCIDateAddDays(
        err:        *const OCIError,
        date:       *const OCIDate,
        num_days:   i32,
        result:     *const OCIDate
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-CE37ECF1-622A-49A9-A9FD-40E1BD67C941
    fn OCIDateAddMonths(
        err:        *const OCIError,
        date:       *const OCIDate,
        num_months: i32,
        result:     *const OCIDate
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-2251373B-4F7B-4680-BB90-F9013216465A
    fn OCIDateAssign(
        err:        *const OCIError,
        date:       *const OCIDate,
        result:     *const OCIDate
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-067F7EB4-419B-4A5B-B1C4-B4C650B874A3
    // fn OCIDateCheck(
    //     err:        *const OCIError,
    //     date:       *const OCIDate,
    //     result:     *const u32
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-282C5B79-64AA-4B34-BFC6-292144B1AD16
    fn OCIDateCompare(
        err:        *const OCIError,
        date1:      *const OCIDate,
        date2:      *const OCIDate,
        result:     *const i32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-42422C47-805F-4EAA-BF44-E6DE6164082E
    fn OCIDateDaysBetween(
        err:        *const OCIError,
        date1:      *const OCIDate,
        date2:      *const OCIDate,
        result:     *const i32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-EA8FEB07-401C-477E-805B-CC9E89FB13F4
    fn OCIDateFromText(
        err:        *const OCIError,
        txt:        *const u8,
        txt_len:    u32,
        fmt:        *const u8,
        fmt_len:    u8,
        lang:       *const u8,
        lang_len:   u32,
        result:     *const OCIDate
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-14FB323E-BAEB-4FC7-81DA-6AF243C0D7D6
    fn OCIDateLastDay(
        err:        *const OCIError,
        date:       *const OCIDate,
        result:     *const OCIDate
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-A16AB88E-A3BF-4B50-8FEF-6427926198F4
    fn OCIDateNextDay(
        err:        *const OCIError,
        date:       *const OCIDate,
        day:        *const u8,
        day_len:    u32,
        result:     *const OCIDate
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-123DD789-48A2-4AD7-8B1E-5E454DFE3F1E
    fn OCIDateToText(
        err:        *const OCIError,
        date:       *const OCIDate,
        fmt:        *const u8,
        fmt_len:    u8,
        lang:       *const u8,
        lang_len:   u32,
        buf_size:   *const u32,
        buf:        *const u8
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-751D4F33-E593-4845-9D5E-8761A19BD243
    fn OCIDateSysDate(
        err:        *const OCIError,
        result:     *const OCIDate
    ) -> i32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-0E4AF4DD-5EEB-434D-BA3A-F4EDE7038FF5
    fn OCIIntervalAdd(
        hndl:       *const c_void,
        err:        *const OCIError,
        addend1:    *const OCIInterval,
        addend2:    *const OCIInterval,
        result:     *mut OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-A218E261-3D40-4B69-AD64-41B697A18C98
    fn OCIIntervalAssign(
        hndl:       *const c_void,
        err:        *const OCIError,
        inpinter:   *const OCIInterval,
        outinter:   *mut OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-90BA159E-79AE-47C6-844C-41BB5ADFEBD3
    // fn OCIIntervalCheck(
    //     hndl:       *const c_void,
    //     err:        *const OCIError,
    //     interval:   *const OCIInterval,
    //     valid:      *const u32,
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-CCE310E5-C75E-4EDD-9B52-9CED37BDFEFF
    fn OCIIntervalCompare(
        hndl:       *const c_void,
        err:        *const OCIError,
        inter1:     *const OCIInterval,
        inter2:     *const OCIInterval,
        result:     *const i32,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-16880D01-45BE-43A3-9CF2-AEAE07B64A6B
    fn OCIIntervalDivide(
        hndl:       *const c_void,
        err:        *const OCIError,
        dividend:   *const OCIInterval,
        divisor:    *const OCINumber,
        result:     *mut OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-1F8A4B39-9EA5-4CEF-9468-079E4203B68D
    fn OCIIntervalFromNumber(
        hndl:       *const c_void,
        err:        *const OCIError,
        interval:   *mut OCIInterval,
        number:     *const OCINumber,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-247BB9B8-307B-4132-A1ED-5CA658B0DAA6
    fn OCIIntervalFromText(
        hndl:       *const c_void,
        err:        *const OCIError,
        inpstring:  *const u8,
        str_len:    size_t,
        result:     *mut OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-12B19818-0001-42F1-8B2C-FD96B7C3231C
    fn OCIIntervalFromTZ(
        hndl:       *const c_void,
        err:        *const OCIError,
        inpstring:  *const u8,
        str_len:    size_t,
        result:     *mut OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-210C4C25-3E8D-4F6D-9502-20B258DACA60
    fn OCIIntervalGetDaySecond(
        hndl:       *const c_void,
        err:        *const OCIError,
        dy:         *const i32,
        hr:         *const i32,
        mm:         *const i32,
        ss:         *const i32,
        fsec:       *const i32,
        interval:   *const OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-665EFBF6-5032-4BD3-B7A3-1C35C2D5A6B7
    fn OCIIntervalGetYearMonth(
        hndl:       *const c_void,
        err:        *const OCIError,
        yr:         *const i32,
        mnth:       *const i32,
        interval:   *const OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-4DBA1745-E675-4774-99AB-DEE2A1FC3788
    fn OCIIntervalMultiply(
        hndl:       *const c_void,
        err:        *const OCIError,
        inter:      *const OCIInterval,
        nfactor:    *const OCINumber,
        result:     *mut OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-303A876B-E1EA-4AF8-8BD1-FC133C5F3F84
    fn OCIIntervalSetDaySecond(
        hndl:       *const c_void,
        err:        *const OCIError,
        dy:         i32,
        hr:         i32,
        mm:         i32,
        ss:         i32,
        fsec:       i32,
        result:     *mut OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-07D8A23E-58E2-420B-B4CA-EF37420F7549
    fn OCIIntervalSetYearMonth(
        hndl:       *const c_void,
        err:        *const OCIError,
        yr:         i32,
        mnth:       i32,
        result:     *mut OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-2D0465BC-B8EA-4F41-B200-587F49D0B2CB
    fn OCIIntervalSubtract(
        hndl:       *const c_void,
        err:        *const OCIError,
        minuend:    *const OCIInterval,
        subtrahend: *const OCIInterval,
        result:     *mut OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-7B403C69-F618-42A6-94F3-41FB17F7F0AD
    fn OCIIntervalToNumber(
        hndl:       *const c_void,
        err:        *const OCIError,
        interval:   *const OCIInterval,
        number:     *const OCINumber,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-DC306081-C4C3-48F5-818D-4C02DD945192
    fn OCIIntervalToText(
        hndl:       *const c_void,
        err:        *const OCIError,
        interval:   *const OCIInterval,
        lfprec:     u8,
        fsprec:     u8,
        buffer:     *const u8,
        buflen:     size_t,
        resultlen:  *const size_t,
    ) -> i32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-61FB0D0F-6EA7-45DD-AF40-310D86FB8BAE
    fn OCINumberAbs(
        err:      *const OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-F3DC6DF6-9110-4BAC-AB97-DC604CA04BCD
    fn OCINumberAdd(
        err:      *const OCIError,
        number1:  *const OCINumber,
        number2:  *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-E7A8B43C-F8B0-4009-A770-94CD7E13EE75
    fn OCINumberArcCos(
        err:      *const OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-3956D4AC-62E5-41FD-BA48-2DA89E207259
    fn OCINumberArcSin(
        err:      *const OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-43E9438C-AA74-4392-889D-171F411EBBE2
    fn OCINumberArcTan(
        err:      *const OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-62C977EF-DB7E-457F-847A-BF0D46E36CD5
    fn OCINumberArcTan2(
        err:      *const OCIError,
        number1:  *const OCINumber,
        number2:  *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-0C78F351-550E-48F0-8D4C-A9AD8A28DA66
    fn OCINumberAssign(
        err:      *const OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-48974097-47D4-4757-A627-4E09406AAFD5
    fn OCINumberCeil(
        err:      *const OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-554A4409-946B-47E9-B239-4140B8F3D1F9
    fn OCINumberCmp(
        err:      *const OCIError,
        number1:  *const OCINumber,
        number2:  *const OCINumber,
        result:   *const i32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-150F3245-ECFC-4352-AA73-AAF29BC6A74C
    fn OCINumberCos(
        err:      *const OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-370FD18E-47D3-4110-817C-658A2F059361
    fn OCINumberDec(
        err:      *const OCIError,
        number:   *const OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-36A6C0EA-85A4-44EE-8489-FB7DB4257513
    fn OCINumberDiv(
        err:      *const OCIError,
        number1:  *const OCINumber,
        number2:  *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-B56F44FC-158A-420B-830E-FB82894A62C8
    fn OCINumberExp(
        err:      *const OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-CF35CBDF-DC88-4E86-B586-0EEFD35C0458
    fn OCINumberFloor(
        err:      *const OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-E8940E06-F4EF-4172-AEE5-AF8E4F6B3AEE
    // fn OCINumberFromInt(
    //     err:      *const OCIError,
    //     inum:     *const c_void,
    //     inum_len: u32,
    //     sign_typ: u32,
    //     number:   *const OCINumber
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-EC8E2C9E-BCD2-4D1E-A052-3E657B552461
    fn OCINumberFromReal(
        err:      *const OCIError,
        rnum:     *const c_void,
        rnum_len: u32,              // sizeof(float | double | long double)
        number:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-F2E458B5-BECC-482E-9223-B92BC696CA17
    fn OCINumberFromText(
        err:      *const OCIError,
        txt:      *const u8,
        txt_len:  u32,
        fmt:      *const u8,
        fmt_len:  u32,
        nls_par:  *const u8,
        nls_len:  u32,
        number:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-08CCC2C4-5AB3-45EB-9E0D-28186A2AA234
    fn OCINumberHypCos(
        err:      *const OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-E7391F43-2DFB-4146-9AB7-816D009F31E5
    fn OCINumberHypSin(
        err:      *const OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-4254930A-DCDC-4590-8710-AC46EC4F3473
    fn OCINumberHypTan(
        err:      *const OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-A3B07A3A-7E18-421E-9085-BE4B3E742C83
    fn OCINumberInc(
        err:      *const OCIError,
        number:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-D5CF4199-D6D2-4D31-A914-FB74F5BC5412
    fn OCINumberIntPower(
        err:      *const OCIError,
        base:     *const OCINumber,
        exp:      i32,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-F1254BAD-7236-4728-A9DA-B8701D8BAA14
    fn OCINumberIsInt(
        err:      *const OCIError,
        number:   *const OCINumber,
        result:   *const i32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-40F344FC-3ED0-4893-AFB1-0853D02D79C9
    fn OCINumberIsZero(
        err:      *const OCIError,
        number:   *const OCINumber,
        result:   *const i32          // set to TRUE if equal to zero else FALSE
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-C1E572F2-F68D-4AF4-831A-2095BFEDDBC3
    fn OCINumberLn(
        err:      *const OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-561769B0-B559-44AA-8012-985EA7ADFB47
    fn OCINumberLog(
        err:      *const OCIError,
        base:     *const OCINumber,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-B5DAB7F2-6AC6-4693-8F04-8C13F9538CE9
    fn OCINumberMod(
        err:      *const OCIError,
        number1:  *const OCINumber,
        number2:  *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-8AAAC840-3776-4283-9DC5-5764CAC2359A
    fn OCINumberMul(
        err:      *const OCIError,
        number1:  *const OCINumber,
        number2:  *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-8810FFCB-51E7-4890-B551-61BE85624764
    fn OCINumberNeg(
        err:      *const OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-E755AD46-4285-4DAF-B2A5-886333A2395D
    fn OCINumberPower(
        err:      *const OCIError,
        base:     *const OCINumber,
        exp:      *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-BE4B0E6D-75B6-4256-A355-9DFAFEC477C9
    fn OCINumberPrec(
        err:      *const OCIError,
        number:   *const OCINumber,
        num_dig:  i32,              // number of decimal digits desired in the result
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-F3B89623-73E3-428F-A677-5526AC5F4622
    fn OCINumberRound(
        err:      *const OCIError,
        number:   *const OCINumber,
        num_dig:  i32,              // number of decimal digits to the right of the decimal point to round to. Negative values are allowed.
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-FA067559-D0F7-426D-940A-1D24F4C60C70
    pub(crate) fn OCINumberSetPi(
        err:      *const OCIError,
        result:   *mut OCINumber
    );

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-8152D558-61D9-49F4-9113-DA1455BB5C72
    pub(crate) fn OCINumberSetZero(
        err:      *const OCIError,
        result:   *mut OCINumber
    );

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-EA7D0DA0-A154-4A87-8215-E5B5A7D091E3
    fn OCINumberShift(
        err:      *const OCIError,
        number:   *const OCINumber,
        num_dec:  i32,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-A535F6F1-0689-4FE1-9C07-C8D341582622
    fn OCINumberSign(
        err:      *const OCIError,
        number:   *const OCINumber,
        result:   *const i32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-65293408-5AF2-4A0C-9C51-82C1C929EE54
    fn OCINumberSin(
        err:      *const OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-9D68D274-B18C-43F4-AB37-BB99C9062B3E
    fn OCINumberSqrt(
        err:      *const OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-192725C3-8F5C-4D0A-848E-4EE9690F4A4E
    fn OCINumberSub(
        err:      *const OCIError,
        number1:  *const OCINumber,
        number2:  *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-1EB45341-6026-47AD-A2EF-D92A20A46ECF
    fn OCINumberTan(
        err:      *const OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-067F138E-E689-4922-9ED7-4A7B0E46447E
    // fn OCINumberToInt(
    //     err:      *const OCIError,
    //     number:   *const OCINumber,
    //     res_len:  u32,
    //     sign_typ: u32,
    //     result:   *const c_void
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-76C4BC1E-EC64-4CF6-82A4-94D5DC242649
    fn OCINumberToReal(
        err:      *const OCIError,
        number:   *const OCINumber,
        res_len:  u32,              // sizeof( float | double | long double)
        result:   *const c_void
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-A850D4E3-2B7B-4DFE-A3E9-618515DACA9E
    // fn OCINumberToRealArray(
    //     err:      *const OCIError,
    //     numbers:  &*const OCINumber,
    //     elems:    u32,
    //     res_len:  u32,              // sizeof( float | double | long double)
    //     result:   *const c_void
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-129A5433-6927-43B7-A10F-5FE6AA354232
    fn OCINumberToText(
        err:      *const OCIError,
        number:   *const OCINumber,
        fmt:      *const u8,
        fmt_len:  u32,
        nls_par:  *const u8,
        nls_len:  u32,
        buf_size: *const u32,
        buf:      *const u8
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-FD8D2A9A-222B-4A0E-B4E3-99588FF19BCA
    fn OCINumberTrunc(
        err:      *const OCIError,
        number:   *const OCINumber,
        num_dig:  i32,
        result:   *mut OCINumber
    ) -> i32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-raw-functions.html#GUID-4856A258-8883-4470-9881-51F27FA050F6
    fn OCIRawAllocSize(
        env:        *const OCIEnv,
        err:        *const OCIError,
        raw:        *const OCIRaw,
        size:       *const u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-raw-functions.html#GUID-3BB4239F-8579-4CC1-B76F-0786BDBAEF9A
    fn OCIRawAssignBytes(
        env:        *const OCIEnv,
        err:        *const OCIError,
        rhs:        *const u8,
        rhs_len:    u32,
        lhs:        *mut *mut OCIRaw
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-raw-functions.html#GUID-27DBFBE0-4511-4B34-8476-B9AC720E3F51
    fn OCIRawAssignRaw(
        env:        *const OCIEnv,
        err:        *const OCIError,
        rhs:        *const OCIRaw,
        lhs:        *mut *mut OCIRaw
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-raw-functions.html#GUID-B05C44C5-7168-438B-AC2A-BD3AD309AAEA
    pub(crate) fn OCIRawPtr(
        env:        *const OCIEnv,
        raw:        *const OCIRaw
    ) -> *const u8;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-raw-functions.html#GUID-7D757B00-DF25-4F61-A3DF-8C72F18FDC9E
    pub(crate) fn OCIRawResize(
        env:        *const OCIEnv,
        err:        *const OCIError,
        size:       u32,
        raw:        *mut *mut OCIRaw
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-raw-functions.html#GUID-D74E75FA-5985-4DDC-BC25-430B415B8837
    pub(crate) fn OCIRawSize(
        env:        *const OCIEnv,
        raw:        *const OCIRaw
    ) -> u32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-3B02C8CC-F35C-422F-B35C-47765C998E57
    fn OCIDateTimeAssign (
        hndl:       *const c_void,
        err:        *const OCIError,
        from:       *const OCIDateTime,
        to:         *mut OCIDateTime
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-5C2A63E3-85EC-4346-A636-33B9B4CCBA41
    // fn OCIDateTimeCheck (
    //     hndl:       *const c_void,
    //     err:        *const OCIError,
    //     date:       *const OCIDateTime,
    //     result:     *const u32
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-5FFD4B08-30E1-461E-8E55-940787D6D8EC
    fn OCIDateTimeCompare (
        hndl:       *const c_void,
        err:        *const OCIError,
        date1:      *const OCIDateTime,
        date2:      *const OCIDateTime,
        result:     *const i32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-FC053036-BE93-42D7-A82C-4DDB6843E167
    fn OCIDateTimeConstruct (
        hndl:       *const c_void,
        err:        *const OCIError,
        datetime:   *mut OCIDateTime,
        year:       i16,
        month:      u8,
        day:        u8,
        hour:       u8,
        min:        u8,
        sec:        u8,
        fsec:       u32,
        timezone:   *const u8,
        tz_len:     size_t
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-744793B2-CD2F-47AC-825A-6FF5BEE12BAB
    fn OCIDateTimeConvert (
        hndl:       *const c_void,
        err:        *const OCIError,
        indate:     *const OCIDateTime,
        outdate:    *mut OCIDateTime
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-16189076-75E9-4B46-B418-89CD8DDB42EA
    // fn OCIDateTimeFromArray(
    //     hndl:       *const c_void,
    //     err:        *const OCIError,
    //     inarray:    *const u8,
    //     len:        u32,
    //     dt_type:    u8,
    //     datetime:   *const OCIDateTime,
    //     reftz:      *const OCIInterval,
    //     fsprec:     u8
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-1A453A79-4EEF-462D-B4B3-45820F9EEA4C
    fn OCIDateTimeFromText(
        hndl:       *const c_void,
        err:        *const OCIError,
        date_str:   *const u8,
        dstr_length: size_t,
        fmt:        *const u8,
        fmt_length: u8,
        lang_name:  *const u8,
        lang_length: size_t,
        datetime:   *mut OCIDateTime,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-FE6F9482-913D-43FD-BE5A-FCD9FA7B83AD
    fn OCIDateTimeGetDate(
        hndl:       *const c_void,
        err:        *const OCIError,
        datetime:   *const OCIDateTime,
        year:       *const i16,
        month:      *const u8,
        day:        *const u8,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-D935ABA2-DEEA-4ABA-AA9C-C27E3E5AC1FD
    fn OCIDateTimeGetTime(
        hndl:       *const c_void,
        err:        *const OCIError,
        datetime:   *const OCIDateTime,
        hour:       *const u8,
        min:        *const u8,
        sec:        *const u8,
        fsec:       *const u32,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-489C51F6-43DB-43DB-980F-2A42AFAFB332
    fn OCIDateTimeGetTimeZoneName(
        hndl:       *const c_void,
        err:        *const OCIError,
        datetime:   *const OCIDateTime,
        buf:        *const u8,
        buflen:     *const u32,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-B8DA860B-FD7D-481B-8347-156969B6EE04
    fn OCIDateTimeGetTimeZoneOffset(
        hndl:       *const c_void,
        err:        *const OCIError,
        datetime:   *const OCIDateTime,
        hour:       *const i8,
        min:        *const i8,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-810C6FB3-9B81-4A7C-9B5B-5D2D93B781FA
    fn OCIDateTimeIntervalAdd(
        hndl:       *const c_void,
        err:        *const OCIError,
        datetime:   *const OCIDateTime,
        inter:      *const OCIInterval,
        result:     *mut OCIDateTime,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-DEDBFEF5-52DD-4036-93FE-C21B6ED4E8A5
    fn OCIDateTimeIntervalSub(
        hndl:       *const c_void,
        err:        *const OCIError,
        datetime:   *const OCIDateTime,
        inter:      *const OCIInterval,
        result:     *mut OCIDateTime,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-BD2F6432-81FF-4CD6-9C3D-85E401894528
    fn OCIDateTimeSubtract(
        hndl:       *const c_void,
        err:        *const OCIError,
        indate1:    *const OCIDateTime,
        indate2:    *const OCIDateTime,
        result:     *mut OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-086776F8-1153-417D-ABC6-A864A2A62788
    fn OCIDateTimeSysTimeStamp(
        hndl:       *const c_void,
        err:        *const OCIError,
        sys_date:   *mut OCIDateTime,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-DCA1CF9E-AF92-42E1-B784-8BFC0C9FF8BE
    // fn OCIDateTimeToArray(
    //     hndl:       *const c_void,
    //     err:        *const OCIError,
    //     datetime:   *const OCIDateTime,
    //     reftz:      *const OCIInterval,
    //     outarray:   *const u8,
    //     len:        *const u32,
    //     fsprec:     u8
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-828401C8-8E88-4C53-A66A-24901CCF93C6
    fn OCIDateTimeToText(
        hndl:       *const c_void,
        err:        *const OCIError,
        date:       *const OCIDateTime,
        fmt:        *const u8,
        fmt_length: u8,
        fsprec:     u8,
        lang_name:  *const u8,
        lang_length: size_t,
        buf_size:   *const u32,
        buf:        *const u8,
    ) -> i32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-string-functions.html#GUID-3F336010-D8C8-4B50-89CB-ABCCA98905DA
    fn OCIStringAllocSize(
        env:        *const OCIEnv,
        err:        *const OCIError,
        txt:        *const OCIString,
        size:       *const u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-string-functions.html#GUID-58BC140A-900C-4409-B3D2-C2DC8FB643FF
    fn OCIStringAssign(
        env:        *const OCIEnv,
        err:        *const OCIError,
        rhs:        *const OCIString,
        lhs:        *mut *mut OCIString
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-string-functions.html#GUID-96E8375B-9017-4E06-BF85-09C12DF286F4
    fn OCIStringAssignText(
        env:        *const OCIEnv,
        err:        *const OCIError,
        rhs:        *const u8,
        rhs_len:    u32,
        lhs:        *mut *mut OCIString
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-string-functions.html#GUID-0E1302F7-A32C-46F1-93D7-FB33CF60C24F
    pub(crate) fn OCIStringPtr(
        env:        *const OCIEnv,
        txt:        *const OCIString
    ) -> *const u8;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-string-functions.html#GUID-CA52A8A4-08BA-4F08-A4A3-79F841F6AE9E
    pub(crate) fn OCIStringResize(
        env:        *const OCIEnv,
        err:        *const OCIError,
        size:       u32,
        txt:        *mut *mut OCIString
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-string-functions.html#GUID-DBDAB2D9-4E78-4752-85B6-55D30CA6AF30
    pub(crate) fn OCIStringSize(
        env:        *const OCIEnv,
        txt:        *const OCIString
    ) -> u32;
}

// ================================================================================================

/**
Returns the 5 digit tuple with the Oracle database version number of the client library at run time.

The 5 digits of the version number are:
 - feature release,
 - release update,
 - release update revision,
 - release update increment,
 - extension.
*/
pub fn client_version() -> (i32, i32, i32, i32, i32) {
    let mut release  = std::mem::MaybeUninit::<i32>::uninit();
    let mut update   = std::mem::MaybeUninit::<i32>::uninit();
    let mut revision = std::mem::MaybeUninit::<i32>::uninit();
    let mut incr     = std::mem::MaybeUninit::<i32>::uninit();
    let mut ext      = std::mem::MaybeUninit::<i32>::uninit();
    unsafe {
        OCIClientVersion(
            release.as_mut_ptr(),
            update.as_mut_ptr(),
            revision.as_mut_ptr(),
            incr.as_mut_ptr(),
            ext.as_mut_ptr(),
        );
    }
    unsafe { (
        release.assume_init(),
        update.assume_init(),
        revision.assume_init(),
        incr.assume_init(),
        ext.assume_init(),
    ) }
}

pub(crate) fn oci_session_release(svc: &OCISvcCtx, err: &OCIError) -> i32 {
    unsafe { OCISessionRelease(svc, err, std::ptr::null(), 0, OCI_DEFAULT) }
}

pub(crate) fn oci_connection_pool_destroy(pool: &OCICPool, err: &OCIError) -> i32 {
    unsafe { OCIConnectionPoolDestroy(pool, err, OCI_DEFAULT) }
}

pub(crate) fn oci_session_pool_destroy(pool: &OCISPool, err: &OCIError) -> i32 {
    unsafe { OCISessionPoolDestroy(pool, err, OCI_DEFAULT) }
}

pub(crate) fn oci_stmt_release(stmt: &OCIStmt, err: &OCIError) -> i32 {
    unsafe { OCIStmtRelease(stmt, err, std::ptr::null(), 0, OCI_DEFAULT) }
}

pub(crate) fn oci_trans_rollback(svchp: &OCISvcCtx, errhp: &OCIError) -> i32 {
    unsafe { OCITransRollback(svchp, errhp, OCI_DEFAULT) }
}

// ================================================================================================

macro_rules! ok_or_env_err {
    ( |$env:ident| $stmt:stmt ) => {{
        let res = unsafe { $stmt };
        if res < 0 {
            Err(Error::env($env, res))
        } else {
            Ok(())
        }
    }};
}

pub(crate) fn descriptor_alloc<T>(parenth: &OCIEnv, descpp: *mut *mut T, desctype: u32) -> Result<()>
where T: OCIStruct
{
    ok_or_env_err!(|parenth|
        OCIDescriptorAlloc(parenth, descpp as _, desctype, 0, std::ptr::null())
    )
}

pub(crate) fn handle_alloc<T: HandleType>(
    parenth: &OCIEnv,
    hndlpp:  *mut *mut T,
    hndltype: u32,
) -> Result<()> {
    ok_or_env_err!(|parenth|
        OCIHandleAlloc(parenth, hndlpp as _, hndltype, 0, std::ptr::null())
    )
}

macro_rules! ok_or_oci_err {
    ( |$err:ident| $stmt:stmt ) => {{
        let res = unsafe { $stmt };
        if res < 0 {
            Err(Error::oci($err, res))
        } else {
            Ok(())
        }
    }};
    ( |$err:ident| $block:block ) => {{
        let res = unsafe { $block };
        if res < 0 {
            Err(Error::oci($err, res))
        } else {
            Ok(())
        }
    }};
}

pub(crate) fn attr_get<T>(
    trgthndlp:  &T,
    trghndltyp: u32,
    attributep: *mut c_void,
    sizep:      &mut u32,
    attrtype:   u32,
    errhp:      &OCIError
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCIAttrGet(trgthndlp as *const T as _, trghndltyp, attributep, sizep, attrtype, errhp)
    )
}

pub(crate) fn attr_set<T>(
    // Even though intuitively `trgthndlp` should be `&mut T` some of the handles that
    // use `attr_set` are behind `Arc`. To be pure we should use `Arc<RwLock>` for those.
    // However, that is entirely unnecessary as all those OCI handles already have
    // internal protection from access by multiple threads as OCIEnv is initialized as
    // OCI_THREADED. Thus, we can cheat a little by declaring handle pointer as `*const`.
    trgthndlp:  &T,
    trghndltyp: u32,
    attributep: *const c_void,
    size:       u32,
    attrtype:   u32,
    errhp:      &OCIError
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCIAttrSet(trgthndlp as *const T as _, trghndltyp, attributep, size, attrtype, errhp)
    )
}

pub(crate) fn param_get(
    hndlp:      &OCIStmt,
    htype:      u32,
    errhp:      &OCIError,
    descr:      *mut *mut OCIParam,
    pos:        u32
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCIParamGet(hndlp as *const OCIStmt as _, htype, errhp, descr as _, pos)
    )
}

pub(crate) fn session_get(
    envhp:      &OCIEnv,
    errhp:      &OCIError,
    svchp:      *mut *mut OCISvcCtx,
    authinfop:  &OCIAuthInfo,
    dbname:     *const u8,
    dbname_len: u32,
    found:      *mut u8,
    mode:       u32
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCISessionGet(envhp, errhp, svchp, authinfop, dbname, dbname_len, std::ptr::null(), 0, std::ptr::null_mut(), std::ptr::null_mut(), found, mode)
    )
}

pub(crate) fn ping(
    svchp: &OCISvcCtx,
    errhp: &OCIError,
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCIPing(svchp, errhp, OCI_DEFAULT)
    )
}


pub(crate) fn trans_commit(
    svchp: &OCISvcCtx,
    errhp: &OCIError,
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCITransCommit(svchp, errhp, OCI_DEFAULT)
    )
}

pub(crate) fn trans_rollback(
    svchp: &OCISvcCtx,
    errhp: &OCIError,
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCITransRollback(svchp, errhp, OCI_DEFAULT)
    )
}


pub(crate) fn connection_pool_create (
    envhp:          &OCIEnv,
    errhp:          &OCIError,
    cpoolhp:        &OCICPool,
    pool_name:      *mut *const u8,
    pool_name_len:  *mut u32,
    dblink:         *const u8,
    dblink_len:     u32,
    conn_min:       u32,
    conn_max:       u32,
    conn_incr:      u32,
    username:       *const u8,
    username_len:   u32,
    password:       *const u8,
    password_len:   u32,
    mode:           u32
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCIConnectionPoolCreate(envhp, errhp, cpoolhp, pool_name, pool_name_len, dblink, dblink_len, conn_min, conn_max, conn_incr, username, username_len, password, password_len, mode)
    )
}

pub(crate) fn session_pool_create (
    envhp:          &OCIEnv,
    errhp:          &OCIError,
    spoolhp:        &OCISPool,
    pool_name:      *mut *const u8,
    pool_name_len:  *mut u32,
    conn_str:       *const u8,
    conn_str_len:   u32,
    sess_min:       u32,
    sess_max:       u32,
    sess_incr:      u32,
    userid:         *const u8,
    userid_len:     u32,
    password:       *const u8,
    password_len:   u32,
    mode:           u32
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCISessionPoolCreate(envhp, errhp, spoolhp, pool_name, pool_name_len, conn_str, conn_str_len, sess_min, sess_max, sess_incr, userid, userid_len, password, password_len, mode)
    )
}

pub(crate) fn stmt_prepare(
    svchp:      &OCISvcCtx,
    stmthp:     *mut *mut OCIStmt,
    errhp:      &OCIError,
    stmttext:   *const u8,
    stmt_len:   u32,
    language:   u32,
    mode:       u32
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCIStmtPrepare2(svchp, stmthp, errhp, stmttext, stmt_len, std::ptr::null(), 0, language, mode)
    )
}

pub(crate) fn stmt_release(
    stmtp:      &OCIStmt,
    errhp:      &OCIError,
    key:        *const u8,
    keylen:     u32,
    mode:       u32
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCIStmtRelease(stmtp, errhp, key, keylen, mode)
    )
}

pub(crate) fn stmt_get_bind_info(
    stmtp:      &OCIStmt,
    errhp:      &OCIError,
    size:       u32,
    startloc:   u32,
    found:      *mut i32,
    bvnp:       *mut *mut u8,
    bvnl:       *mut u8,
    invp:       *mut *mut u8,
    invl:       *mut u8,
    dupl:       *mut u8,
    hndl:       *mut *mut OCIBind,
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCIStmtGetBindInfo(stmtp, errhp, size, startloc, found, bvnp, bvnl, invp, invl, dupl, hndl)
    )
}

pub(crate) fn bind_by_pos(
    stmtp:      &OCIStmt,
    bindpp:     *mut *mut OCIBind,
    errhp:      &OCIError,
    position:   u32,
    valuep:     *mut c_void,
    value_sz:   i64,
    dty:        u16,
    indp:       *mut i16,
    alenp:      *mut u32,
    mode:       u32
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCIBindByPos2(stmtp, bindpp, errhp, position, valuep, value_sz, dty, indp, alenp, std::ptr::null_mut::<u16>(), 0, std::ptr::null_mut::<u32>(), mode)
    )
}

pub(crate) fn stmt_execute(
    svchp:      &OCISvcCtx,
    stmtp:      &OCIStmt,
    errhp:      &OCIError,
    iters:      u32,
    rowoff:     u32,
    mode:       u32
) -> Result<i32> {
    let res = unsafe {
        OCIStmtExecute(svchp, stmtp, errhp, iters, rowoff, std::ptr::null(), std::ptr::null(), mode)
    };
    match res {
        OCI_ERROR | OCI_INVALID_HANDLE => { Err(Error::oci(errhp, res)) },
        _ => { Ok(res) }
    }
}

pub(crate) fn stmt_fetch(
    stmtp:      &OCIStmt,
    errhp:      &OCIError,
    nrows:      u32,
    orient:     u16,
    offset:     i16,
    mode:       u32
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCIStmtFetch2(stmtp, errhp, nrows, orient, offset, mode)
    )
}

pub(crate) fn define_by_pos(
    stmtp:      &OCIStmt,
    defnpp:     *mut *mut OCIDefine,
    errhp:      &OCIError,
    position:   u32,
    valuep:     *mut c_void,
    value_sz:   i64,
    dty:        u16,
    indp:       *mut i16,
    rlenp:      *mut u32,
    rcodep:     *mut u16,
    mode:       u32
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCIDefineByPos2(stmtp, defnpp, errhp, position, valuep, value_sz, dty, indp, rlenp, rcodep, mode)
    )
}

pub(crate) fn rowid_to_char(
    desc:   &OCIRowid,
    text:   *mut u8,
    size:   *mut u16,
    errhp:  &OCIError,
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCIRowidToChar(desc, text, size, errhp)
    )
}

pub(crate) fn lob_append(
    svchp:      &OCISvcCtx,
    errhp:      &OCIError,
    dst:        &OCILobLocator,
    src:        &OCILobLocator,
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCILobAppend(svchp, errhp, dst, src)
    )
}

pub(crate) fn lob_char_set_form(
    envhp:      &OCIEnv,
    errhp:      &OCIError,
    src:        &OCILobLocator,
    csform:     *mut u8
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCILobCharSetForm(envhp, errhp, src, csform)
    )
}

pub(crate) fn lob_char_set_id(
    envhp:      &OCIEnv,
    errhp:      &OCIError,
    src:        &OCILobLocator,
    csid:       *mut u16
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCILobCharSetId(envhp, errhp, src, csid)
    )
}

pub(crate) fn lob_close(
    svchp:      &OCISvcCtx,
    errhp:      &OCIError,
    loc:        &OCILobLocator,
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCILobClose(svchp, errhp, loc)
    )
}

pub(crate) fn lob_copy(
    svchp:      &OCISvcCtx,
    errhp:      &OCIError,
    dst:        &OCILobLocator,
    src:        &OCILobLocator,
    amount:     u64,
    dst_off:    u64,
    src_off:    u64,
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCILobCopy2(svchp, errhp, dst, src, amount, dst_off, src_off)
    )
}

pub(crate) fn lob_create_temporary(
    svchp:      &OCISvcCtx,
    errhp:      &OCIError,
    loc:        &OCILobLocator,
    csid:       u16,
    csfrm:      u8,
    lob_type:   u8,
    cache:      u8,
    duration:   u16,
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCILobCreateTemporary(svchp, errhp, loc, csid, csfrm, lob_type, cache, duration)
    )
}

pub(crate) fn lob_erase(
    svchp:      &OCISvcCtx,
    errhp:      &OCIError,
    loc:        &OCILobLocator,
    amount:     *mut u64,
    offset:     u64,
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCILobErase2(svchp, errhp, loc, amount, offset)
    )
}

pub(crate) fn lob_file_close(
    svchp:      &OCISvcCtx,
    errhp:      &OCIError,
    filep:      &OCILobLocator,
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCILobFileClose(svchp, errhp, filep)
    )
}

pub(crate) fn lob_file_exists(
    svchp:      &OCISvcCtx,
    errhp:      &OCIError,
    filep:      &OCILobLocator,
    flag:       *mut u8
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCILobFileExists(svchp, errhp, filep, flag)
    )
}

pub(crate) fn lob_file_get_name(
    envhp:      &OCIEnv,
    errhp:      &OCIError,
    loc:        &OCILobLocator,
    dir:        *mut u8,
    dir_len:    *mut u16,
    filename:   *mut u8,
    name_len:   *mut u16,
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCILobFileGetName(envhp, errhp, loc, dir, dir_len, filename, name_len)
    )
}

pub(crate) fn lob_file_is_open(
    svchp:      &OCISvcCtx,
    errhp:      &OCIError,
    filep:      &OCILobLocator,
    flag:       *mut u8
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCILobFileIsOpen(svchp, errhp, filep, flag)
    )
}

pub(crate) fn lob_file_open(
    svchp:      &OCISvcCtx,
    errhp:      &OCIError,
    filep:      &OCILobLocator,
    mode:       u8
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCILobFileOpen(svchp, errhp, filep, mode)
    )
}

pub(crate) fn lob_file_set_name(
    envhp:      &OCIEnv,
    errhp:      &OCIError,
    filepp:     *const *mut OCIBFileLocator,
    dir:        *const u8,
    dir_len:    u16,
    filename:   *const u8,
    name_len:   u16,
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCILobFileSetName(envhp, errhp, filepp as _, dir, dir_len, filename, name_len)
    )
}

pub(crate) fn lob_free_temporary(
    svchp:      &OCISvcCtx,
    errhp:      &OCIError,
    loc:        &OCILobLocator,
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCILobFreeTemporary(svchp, errhp, loc)
    )
}

pub(crate) fn lob_get_chunk_size(
    svchp:      &OCISvcCtx,
    errhp:      &OCIError,
    loc:        &OCILobLocator,
    size:       *mut u32,
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCILobGetChunkSize(svchp, errhp, loc, size)
    )
}

pub(crate) fn lob_get_content_type(
    envhp:      &OCIEnv,
    svchp:      &OCISvcCtx,
    errhp:      &OCIError,
    loc:        &OCILobLocator,
    ctx_type:   *mut u8,
    len:        *mut u32,
    mode:       u32
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCILobGetContentType(envhp, svchp, errhp, loc, ctx_type, len, mode)
    )
}

pub(crate) fn lob_get_length(
    svchp:      &OCISvcCtx,
    errhp:      &OCIError,
    loc:        &OCILobLocator,
    len:        *mut u64,
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCILobGetLength2(svchp, errhp, loc, len)
    )
}

pub(crate) fn lob_is_equal(
    envhp:      &OCIEnv,
    loc1:       &OCILobLocator,
    loc2:       &OCILobLocator,
    flag:       *mut u8,
) -> Result<()> {
    ok_or_env_err!(|envhp|
        OCILobIsEqual(envhp, loc1, loc2, flag)
    )
}

pub(crate) fn lob_is_open(
    svchp:      &OCISvcCtx,
    errhp:      &OCIError,
    loc:        &OCILobLocator,
    flag:       *mut u8,
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCILobIsOpen(svchp, errhp, loc, flag)
    )
}

pub(crate) fn lob_is_temporary(
    svchp:      &OCISvcCtx,
    errhp:      &OCIError,
    loc:        &OCILobLocator,
    flag:       *mut u8,
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCILobIsTemporary(svchp, errhp, loc, flag)
    )
}

pub(crate) fn lob_load_from_file(
    svchp:      &OCISvcCtx,
    errhp:      &OCIError,
    dst:        &OCILobLocator,
    src:        &OCILobLocator,
    amount:     u64,
    dst_off:    u64,
    src_off:    u64,
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCILobLoadFromFile2(svchp, errhp, dst, src, amount, dst_off, src_off)
    )
}

pub(crate) fn lob_locator_assign(
    svchp:      &OCISvcCtx,
    errhp:      &OCIError,
    src:        &OCILobLocator,
    dst:        *mut *mut OCILobLocator,
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCILobLocatorAssign(svchp, errhp, src, dst)
    )
}

pub(crate) fn lob_locator_is_init(
    envhp:      &OCIEnv,
    errhp:      &OCIError,
    src:        &OCILobLocator,
    flag:       *mut u8,
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCILobLocatorIsInit(envhp, errhp, src, flag)
    )
}

pub(crate) fn lob_open(
    svchp:      &OCISvcCtx,
    errhp:      &OCIError,
    loc:        &OCILobLocator,
    mode:       u8,
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCILobOpen(svchp, errhp, loc, mode)
    )
}

pub(crate) fn lob_read(
    svchp:      &OCISvcCtx,
    errhp:      &OCIError,
    loc:        &OCILobLocator,
    byte_cnt:   *mut u64,
    char_cnt:   *mut u64,
    offset:     u64,
    buf:        *mut u8,
    buf_len:    u64,
    piece:      u8,
    csid:       u16,
    csfrm:      u8,
) -> Result<i32> {
    let res = unsafe {
        OCILobRead2(svchp, errhp, loc, byte_cnt, char_cnt, offset, buf, buf_len, piece, std::ptr::null_mut::<c_void>(), std::ptr::null::<c_void>(), csid, csfrm)
    };
    if res < 0 {
        Err(Error::oci(errhp, res))
    } else {
        Ok(res)
    }
}

pub(crate) fn lob_set_content_type(
    envhp:      &OCIEnv,
    svchp:      &OCISvcCtx,
    errhp:      &OCIError,
    loc:        &OCILobLocator,
    ctype:      *const u8,
    len:        u32,
    mode:       u32,
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCILobSetContentType(envhp, svchp, errhp, loc, ctype, len, mode)
    )
}

pub(crate) fn lob_trim(
    svchp:      &OCISvcCtx,
    errhp:      &OCIError,
    loc:        &OCILobLocator,
    len:        u64,
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCILobTrim2(svchp, errhp, loc, len)
    )
}

pub(crate) fn lob_write(
    svchp:      &OCISvcCtx,
    errhp:      &OCIError,
    loc:        &OCILobLocator,
    byte_cnt:   *mut u64,
    char_cnt:   *mut u64,
    offset:     u64,
    buf:        *const u8,
    buf_len:    u64,
    piece:      u8,
    ctx:        *mut c_void,
    write_cb:   *const c_void,
    csid:       u16,
    csfrm:      u8,
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCILobWrite2(svchp, errhp, loc, byte_cnt, char_cnt, offset, buf, buf_len, piece, ctx, write_cb, csid, csfrm)
    )
}

pub(crate) fn lob_write_append(
    svchp:      &OCISvcCtx,
    errhp:      &OCIError,
    loc:        &OCILobLocator,
    byte_cnt:   *mut u64,
    char_cnt:   *mut u64,
    buf:        *const u8,
    buf_len:    u64,
    piece:      u8,
    ctx:        *mut c_void,
    write_cb:   *const c_void,
    csid:       u16,
    csfrm:      u8,
) -> Result<()> {
    ok_or_oci_err!(|errhp|
        OCILobWriteAppend2(svchp, errhp, loc, byte_cnt, char_cnt, buf, buf_len, piece, ctx, write_cb, csid, csfrm)
    )
}

pub(crate) fn date_add_days(
    err:        &OCIError,
    date:       &OCIDate,
    num_days:   i32,
    result:     *mut OCIDate
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIDateAddDays(err, date, num_days, result)
    )
}

pub(crate) fn date_add_months(
    err:        &OCIError,
    date:       &OCIDate,
    num_months: i32,
    result:     *mut OCIDate
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIDateAddMonths(err, date, num_months, result)
    )
}

pub(crate) fn date_assign(
    err:        &OCIError,
    date:       &OCIDate,
    result:     *mut OCIDate,
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIDateAssign(err, date, result)
    )
}

pub(crate) fn date_compare(
    err:        &OCIError,
    date1:      &OCIDate,
    date2:      &OCIDate,
    result:     *mut i32
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIDateCompare(err, date1, date2, result)
    )
}

pub(crate) fn date_days_between(
    err:        &OCIError,
    date1:      &OCIDate,
    date2:      &OCIDate,
    result:     *mut i32
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIDateDaysBetween(err, date1, date2, result)
    )
}

pub(crate) fn date_from_text(
    err:        &OCIError,
    txt:        *const u8,
    txt_len:    u32,
    fmt:        *const u8,
    fmt_len:    u8,
    result:     *mut OCIDate
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIDateFromText(err, txt, txt_len, fmt, fmt_len, std::ptr::null(), 0, result)
    )
}

pub(crate) fn date_last_day(
    err:        &OCIError,
    date:       &OCIDate,
    result:     *mut OCIDate
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIDateLastDay(err, date, result)
    )
}

pub(crate) fn date_next_day(
    err:        &OCIError,
    date:       &OCIDate,
    day:        *const u8,
    day_len:    u32,
    result:     *mut OCIDate
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIDateNextDay(err, date, day, day_len, result)
    )
}

pub(crate) fn date_to_text(
    err:        &OCIError,
    date:       &OCIDate,
    fmt:        *const u8,
    fmt_len:    u8,
    buf_size:   *mut u32,
    buf:        *mut u8
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIDateToText(err, date, fmt, fmt_len, std::ptr::null(), 0, buf_size, buf)
    )
}

pub(crate) fn date_sys_date(
    err:        &OCIError,
    result:     *mut OCIDate
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIDateSysDate(err, result)
    )
}

pub(crate) fn interval_add(
    hndl:       *const c_void,
    err:        &OCIError,
    addend1:    &OCIInterval,
    addend2:    &OCIInterval,
    result:     &mut OCIInterval,
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIIntervalAdd(hndl, err, addend1, addend2, result)
    )
}

pub(crate) fn interval_assign(
    hndl:       *const c_void,
    err:        &OCIError,
    inpinter:   &OCIInterval,
    outinter:   &mut OCIInterval,
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIIntervalAssign(hndl, err, inpinter, outinter)
    )
}

pub(crate) fn interval_compare(
    hndl:       *const c_void,
    err:        &OCIError,
    inter1:     &OCIInterval,
    inter2:     &OCIInterval,
    result:     *mut i32,
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIIntervalCompare(hndl, err, inter1, inter2, result)
    )
}

pub(crate) fn interval_divide(
    hndl:       *const c_void,
    err:        &OCIError,
    dividend:   &OCIInterval,
    divisor:    &OCINumber,
    result:     &mut OCIInterval,
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIIntervalDivide(hndl, err, dividend, divisor, result)
    )
}

pub(crate) fn interval_from_number(
    hndl:       *const c_void,
    err:        &OCIError,
    interval:   &mut OCIInterval,
    number:     &OCINumber,
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIIntervalFromNumber(hndl, err, interval, number)
    )
}

pub(crate) fn interval_from_text(
    hndl:       *const c_void,
    err:        &OCIError,
    inpstring:  *const u8,
    str_len:    size_t,
    result:     &mut OCIInterval,
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIIntervalFromText(hndl, err, inpstring, str_len, result)
    )
}

pub(crate) fn interval_from_tz(
    hndl:       *const c_void,
    err:        &OCIError,
    inpstring:  *const u8,
    str_len:    size_t,
    result:     &mut OCIInterval,
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIIntervalFromTZ(hndl, err, inpstring, str_len, result)
    )
}

pub(crate) fn interval_get_day_second(
    hndl:       *const c_void,
    err:        &OCIError,
    dy:         *mut i32,
    hr:         *mut i32,
    mm:         *mut i32,
    ss:         *mut i32,
    fsec:       *mut i32,
    interval:   &OCIInterval,
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIIntervalGetDaySecond(hndl, err, dy, hr, mm, ss, fsec, interval)
    )
}

pub(crate) fn interval_get_year_month(
    hndl:       *const c_void,
    err:        &OCIError,
    yr:         *mut i32,
    mnth:       *mut i32,
    interval:   &OCIInterval,
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIIntervalGetYearMonth(hndl, err, yr, mnth, interval)
    )
}

pub(crate) fn interval_multiply(
    hndl:       *const c_void,
    err:        &OCIError,
    inter:      &OCIInterval,
    nfactor:    &OCINumber,
    result:     &mut OCIInterval,
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIIntervalMultiply(hndl, err, inter, nfactor, result)
    )
}

pub(crate) fn interval_set_day_second(
    hndl:       *const c_void,
    err:        &OCIError,
    dy:         i32,
    hr:         i32,
    mm:         i32,
    ss:         i32,
    fsec:       i32,
    result:     &mut OCIInterval,
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIIntervalSetDaySecond(hndl, err, dy, hr, mm, ss, fsec, result)
    )
}

pub(crate) fn interval_set_year_month(
    hndl:       *const c_void,
    err:        &OCIError,
    yr:         i32,
    mnth:       i32,
    result:     &mut OCIInterval,
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIIntervalSetYearMonth(hndl, err, yr, mnth, result)
    )
}

pub(crate) fn interval_subtract(
    hndl:       *const c_void,
    err:        &OCIError,
    minuend:    &OCIInterval,
    subtrahend: &OCIInterval,
    result:     &mut OCIInterval,
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIIntervalSubtract(hndl, err, minuend, subtrahend, result)
    )
}

pub(crate) fn interval_to_number(
    hndl:       *const c_void,
    err:        &OCIError,
    interval:   &OCIInterval,
    number:     *mut OCINumber,
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIIntervalToNumber(hndl, err, interval, number)
    )
}

pub(crate) fn interval_to_text(
    hndl:       *const c_void,
    err:        &OCIError,
    interval:   &OCIInterval,
    lfprec:     u8,
    fsprec:     u8,
    buffer:     *mut u8,
    buflen:     size_t,
    resultlen:  *mut size_t,
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIIntervalToText(hndl, err, interval, lfprec, fsprec, buffer, buflen, resultlen)
    )
}

pub(crate) fn raw_alloc_size(
    env:        &OCIEnv,
    err:        &OCIError,
    raw:        &OCIRaw,
    size:       *mut u32
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIRawAllocSize(env, err, raw, size)
    )
}

pub(crate) fn raw_assign_bytes(
    env:        &OCIEnv,
    err:        &OCIError,
    rhs:        *const u8,
    rhs_len:    u32,
    lhs:        *mut *mut OCIRaw
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIRawAssignBytes(env, err, rhs, rhs_len, lhs)
    )
}

pub(crate) fn raw_assign_raw(
    env:        &OCIEnv,
    err:        &OCIError,
    rhs:        &OCIRaw,
    lhs:        *mut *mut OCIRaw
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIRawAssignRaw(env, err, rhs, lhs)
    )
}

pub(crate) fn raw_resize(
    env:        &OCIEnv,
    err:        &OCIError,
    size:       u32,
    raw:        *mut *mut OCIRaw
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIRawResize(env, err, size, raw)
    )
}

pub(crate) fn string_alloc_size(
    env:        &OCIEnv,
    err:        &OCIError,
    txt:        &OCIString,
    size:       *mut u32
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIStringAllocSize(env, err, txt, size)
    )
}

pub(crate) fn string_assign(
    env:        &OCIEnv,
    err:        &OCIError,
    rhs:        &OCIString,
    lhs:        *mut *mut OCIString
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIStringAssign(env, err, rhs, lhs)
    )
}

pub(crate) fn string_assign_text(
    env:        &OCIEnv,
    err:        &OCIError,
    rhs:        *const u8,
    rhs_len:    u32,
    lhs:        *mut *mut OCIString
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIStringAssignText(env, err, rhs, rhs_len, lhs)
    )
}

pub(crate) fn string_resize(
    env:        &OCIEnv,
    err:        &OCIError,
    size:       u32,
    txt:        *mut *mut OCIString
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIStringResize(env, err, size, txt)
    )
}

pub(crate) fn date_time_assign (
    hndl:       *const c_void,
    err:        &OCIError,
    from:       &OCIDateTime,
    to:         *mut OCIDateTime
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIDateTimeAssign(hndl, err, from, to)
    )
}

pub(crate) fn date_time_compare (
    hndl:       *const c_void,
    err:        &OCIError,
    date1:      &OCIDateTime,
    date2:      &OCIDateTime,
    result:     *mut i32
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIDateTimeCompare(hndl, err, date1, date2, result)
    )
}

pub(crate) fn date_time_construct (
    hndl:       *const c_void,
    err:        &OCIError,
    datetime:   &mut OCIDateTime,
    year:       i16,
    month:      u8,
    day:        u8,
    hour:       u8,
    min:        u8,
    sec:        u8,
    fsec:       u32,
    timezone:   *const u8,
    tz_len:     size_t
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIDateTimeConstruct(hndl, err, datetime, year, month, day, hour, min, sec, fsec, timezone, tz_len)
    )
}

pub(crate) fn date_time_convert (
    hndl:       *const c_void,
    err:        &OCIError,
    indate:     &OCIDateTime,
    outdate:    &mut OCIDateTime
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIDateTimeConvert(hndl, err, indate, outdate)
    )
}

pub(crate) fn date_time_from_text(
    hndl:       *const c_void,
    err:        &OCIError,
    date_str:   *const u8,
    dstr_length: size_t,
    fmt:        *const u8,
    fmt_length: u8,
    lang_name:  *const u8,
    lang_length: size_t,
    datetime:   &mut OCIDateTime,
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIDateTimeFromText(hndl, err, date_str, dstr_length, fmt, fmt_length, lang_name, lang_length, datetime)
    )
}

pub(crate) fn date_time_get_date(
    hndl:       *const c_void,
    err:        &OCIError,
    datetime:   &OCIDateTime,
    year:       *mut i16,
    month:      *mut u8,
    day:        *mut u8,
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIDateTimeGetDate(hndl, err, datetime, year, month, day)
    )
}

pub(crate) fn date_time_get_time(
    hndl:       *const c_void,
    err:        &OCIError,
    datetime:   &OCIDateTime,
    hour:       *mut u8,
    min:        *mut u8,
    sec:        *mut u8,
    fsec:       *mut u32,
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIDateTimeGetTime(hndl, err, datetime, hour, min, sec, fsec)
    )
}

pub(crate) fn date_time_get_time_zone_name(
    hndl:       *const c_void,
    err:        &OCIError,
    datetime:   &OCIDateTime,
    buf:        *mut u8,
    buflen:     *mut u32,
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIDateTimeGetTimeZoneName(hndl, err, datetime, buf, buflen)
    )
}

pub(crate) fn date_time_get_time_zone_offset(
    hndl:       *const c_void,
    err:        &OCIError,
    datetime:   &OCIDateTime,
    hour:       *mut i8,
    min:        *mut i8,
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIDateTimeGetTimeZoneOffset(hndl, err, datetime, hour, min)
    )
}

pub(crate) fn date_time_interval_add(
    hndl:       *const c_void,
    err:        &OCIError,
    datetime:   &OCIDateTime,
    inter:      &OCIInterval,
    result:     &mut OCIDateTime,
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIDateTimeIntervalAdd(hndl, err, datetime, inter, result)
    )
}

pub(crate) fn date_time_interval_sub(
    hndl:       *const c_void,
    err:        &OCIError,
    datetime:   &OCIDateTime,
    inter:      &OCIInterval,
    result:     &mut OCIDateTime,
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIDateTimeIntervalSub(hndl, err, datetime, inter, result)
    )
}

pub(crate) fn date_time_subtract(
    hndl:       *const c_void,
    err:        &OCIError,
    indate1:    &OCIDateTime,
    indate2:    &OCIDateTime,
    result:     &mut OCIInterval,
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIDateTimeSubtract(hndl, err, indate1, indate2, result)
    )
}

pub(crate) fn date_time_sys_time_stamp(
    hndl:       *const c_void,
    err:        &OCIError,
    sys_date:   &mut OCIDateTime,
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIDateTimeSysTimeStamp(hndl, err, sys_date)
    )
}

pub(crate) fn date_time_to_text(
    hndl:       *const c_void,
    err:        &OCIError,
    date:       &OCIDateTime,
    fmt:        *const u8,
    fmt_length: u8,
    fsprec:     u8,
    buf_size:   *mut u32,
    buf:        *mut u8,
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCIDateTimeToText(hndl, err, date, fmt, fmt_length, fsprec, std::ptr::null(), 0, buf_size, buf)
    )
}

pub(crate) fn number_abs(
    err:      &OCIError,
    number:   &OCINumber,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberAbs(err, number, result)
    )
}

pub(crate) fn number_add(
    err:      &OCIError,
    number1:  &OCINumber,
    number2:  &OCINumber,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberAdd(err, number1, number2, result)
    )
}

pub(crate) fn number_arc_cos(
    err:      &OCIError,
    number:   &OCINumber,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberArcCos(err, number, result)
    )
}

pub(crate) fn number_arc_sin(
    err:      &OCIError,
    number:   &OCINumber,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberArcSin(err, number, result)
    )
}

pub(crate) fn number_arc_tan(
    err:      &OCIError,
    number:   &OCINumber,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberArcTan(err, number, result)
    )
}

pub(crate) fn number_arc_tan2(
    err:      &OCIError,
    number1:  &OCINumber,
    number2:  &OCINumber,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberArcTan2(err, number1, number2, result)
    )
}

pub(crate) fn number_assign(
    err:      &OCIError,
    number:   &OCINumber,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberAssign(err, number, result)
    )
}

pub(crate) fn number_ceil(
    err:      &OCIError,
    number:   &OCINumber,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberCeil(err, number, result)
    )
}

pub(crate) fn number_cmp(
    err:      &OCIError,
    number1:  &OCINumber,
    number2:  &OCINumber,
    result:   *mut i32
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberCmp(err, number1, number2, result)
    )
}

pub(crate) fn number_cos(
    err:      &OCIError,
    number:   &OCINumber,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberCos(err, number, result)
    )
}

pub(crate) fn number_dec(
    err:      &OCIError,
    number:   &OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberDec(err, number)
    )
}

pub(crate) fn number_div(
    err:      &OCIError,
    number1:  &OCINumber,
    number2:  &OCINumber,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberDiv(err, number1, number2, result)
    )
}

pub(crate) fn number_exp(
    err:      &OCIError,
    number:   &OCINumber,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberExp(err, number, result)
    )
}

pub(crate) fn number_floor(
    err:      &OCIError,
    number:   &OCINumber,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberFloor(err, number, result)
    )
}

pub(crate) fn number_from_real(
    err:      &OCIError,
    rnum:     *const c_void,
    rnum_len: u32,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberFromReal(err, rnum, rnum_len, result)
    )
}

pub(crate) fn number_from_text(
    err:      &OCIError,
    txt:      *const u8,
    txt_len:  u32,
    fmt:      *const u8,
    fmt_len:  u32,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberFromText(err, txt, txt_len, fmt, fmt_len, std::ptr::null(), 0, result)
    )
}

pub(crate) fn number_hyp_cos(
    err:      &OCIError,
    number:   &OCINumber,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberHypCos(err, number, result)
    )
}

pub(crate) fn number_hyp_sin(
    err:      &OCIError,
    number:   &OCINumber,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberHypSin(err, number, result)
    )
}

pub(crate) fn number_hyp_tan(
    err:      &OCIError,
    number:   &OCINumber,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberHypTan(err, number, result)
    )
}

pub(crate) fn number_inc(
    err:      &OCIError,
    number:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberInc(err, number)
    )
}

pub(crate) fn number_int_power(
    err:      &OCIError,
    base:     &OCINumber,
    exp:      i32,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberIntPower(err, base, exp, result)
    )
}

pub(crate) fn number_is_int(
    err:      &OCIError,
    number:   &OCINumber,
    result:   *mut i32
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberIsInt(err, number, result)
    )
}

pub(crate) fn number_is_zero(
    err:      &OCIError,
    number:   &OCINumber,
    result:   *mut i32
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberIsZero(err, number, result)
    )
}

pub(crate) fn number_ln(
    err:      &OCIError,
    number:   &OCINumber,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberLn(err, number, result)
    )
}

pub(crate) fn number_log(
    err:      &OCIError,
    base:     &OCINumber,
    number:   &OCINumber,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberLog(err, base, number, result)
    )
}

pub(crate) fn number_mod(
    err:      &OCIError,
    number1:  &OCINumber,
    number2:  &OCINumber,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberMod(err, number1, number2, result)
    )
}

pub(crate) fn number_mul(
    err:      &OCIError,
    number1:  &OCINumber,
    number2:  &OCINumber,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberMul(err, number1, number2, result)
    )
}

pub(crate) fn number_neg(
    err:      &OCIError,
    number:   &OCINumber,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberNeg(err, number, result)
    )
}

pub(crate) fn number_power(
    err:      &OCIError,
    base:     &OCINumber,
    exp:      &OCINumber,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberPower(err, base, exp, result)
    )
}

pub(crate) fn number_prec(
    err:      &OCIError,
    number:   &OCINumber,
    num_dig:  i32,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberPrec(err, number, num_dig, result)
    )
}

pub(crate) fn number_round(
    err:      &OCIError,
    number:   &OCINumber,
    num_dig:  i32,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberRound(err, number, num_dig, result)
    )
}

pub(crate) fn number_shift(
    err:      &OCIError,
    number:   &OCINumber,
    num_dec:  i32,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberShift(err, number, num_dec, result)
    )
}

pub(crate) fn number_sign(
    err:      &OCIError,
    number:   &OCINumber,
    result:   *mut i32
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberSign(err, number, result)
    )
}

pub(crate) fn number_sin(
    err:      &OCIError,
    number:   &OCINumber,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberSin(err, number, result)
    )
}

pub(crate) fn number_sqrt(
    err:      &OCIError,
    number:   &OCINumber,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberSqrt(err, number, result)
    )
}

pub(crate) fn number_sub(
    err:      &OCIError,
    number1:  &OCINumber,
    number2:  &OCINumber,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberSub(err, number1, number2, result)
    )
}

pub(crate) fn number_tan(
    err:      &OCIError,
    number:   &OCINumber,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberTan(err, number, result)
    )
}

pub(crate) fn number_to_real(
    err:      &OCIError,
    number:   &OCINumber,
    res_len:  u32,
    result:   *mut c_void
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberToReal(err, number, res_len, result)
    )
}

pub(crate) fn number_to_text(
    err:      &OCIError,
    number:   &OCINumber,
    fmt:      *const u8,
    fmt_len:  u32,
    buf_size: *mut u32,
    buf:      *mut u8
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberToText(err, number, fmt, fmt_len, std::ptr::null(), 0, buf_size, buf)
    )
}

pub(crate) fn number_trunc(
    err:      &OCIError,
    number:   &OCINumber,
    num_dig:  i32,
    result:   *mut OCINumber
) -> Result<()> {
    ok_or_oci_err!(|err|
        OCINumberTrunc(err, number, num_dig, result)
    )
}

// The End.
// `oci.rs` is used as an input in some CLOB tests (thus far it is the largest file).
// The following random supplemental symbols are added to make it not pure ASCII and
// ensure that byte vs char counts are not the same):
// 🤸🥀🥂🥃🥅🥇🥕🥝🥞🥡🥤🥥🥨🧤🧦🧨🧪🧬🧭🧮🧯🧰🧲🧵