//! Oracle OCI

#![allow(dead_code)]

use libc::{size_t, c_void};

pub mod ptr;
pub mod attr;
pub mod param;
pub mod handle;
pub mod desc;

pub use ptr::Ptr;
pub use handle::Handle;
pub use desc::Descriptor;

pub(crate) use desc::DescriptorType;

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

// Attribute Constants
pub(crate) const OCI_ATTR_ROW_COUNT         : u32 = 9;
pub(crate) const OCI_ATTR_PREFETCH_ROWS     : u32 = 11;
pub(crate) const OCI_ATTR_PARAM_COUNT       : u32 = 18;     // number of columns in the select list
pub(crate) const OCI_ATTR_STMT_TYPE         : u32 = 24;
pub(crate) const OCI_ATTR_BIND_COUNT        : u32 = 190;
pub(crate) const OCI_ATTR_ROWS_FETCHED      : u32 = 197;
pub(crate) const OCI_ATTR_STMT_IS_RETURNING : u32 = 218;
pub(crate) const OCI_ATTR_UB8_ROW_COUNT     : u32 = 457;
pub(crate) const OCI_ATTR_INVISIBLE_COL     : u32 = 461;

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

// Handle Definitions
#[repr(C)] pub struct OCIEnv                { _private: [u8; 0] }
#[repr(C)] pub struct OCIError              { _private: [u8; 0] }
#[repr(C)] pub struct OCISvcCtx             { _private: [u8; 0] }
#[repr(C)] pub struct OCIStmt               { _private: [u8; 0] }
#[repr(C)] pub struct OCIBind               { _private: [u8; 0] }
#[repr(C)] pub struct OCIDefine             { _private: [u8; 0] }
#[repr(C)] pub struct OCIDescribe           { _private: [u8; 0] }
#[repr(C)] pub struct OCIServer             { _private: [u8; 0] }
#[repr(C)] pub struct OCISession            { _private: [u8; 0] }
#[repr(C)] pub struct OCIRaw                { _private: [u8; 0] }

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
#[repr(C)] pub struct OCIResult             { _private: [u8; 0] }
#[repr(C)] pub struct OCILobLocator         { _private: [u8; 0] }
#[repr(C)] pub struct OCILobRegion          { _private: [u8; 0] }
#[repr(C)] pub struct OCIParam              { _private: [u8; 0] }
#[repr(C)] pub struct OCIRowid              { _private: [u8; 0] }
#[repr(C)] pub struct OCIDateTime           { _private: [u8; 0] }
#[repr(C)] pub struct OCIInterval           { _private: [u8; 0] }
#[repr(C)] pub struct OCIString             { _private: [u8; 0] }

// Virtual descriptors
pub struct OCICLobLocator           {}
pub struct OCIBLobLocator           {}
pub struct OCIBFileLocator          {}
pub struct OCITimestamp             {}
pub struct OCITimestampTZ           {}
pub struct OCITimestampLTZ          {}
pub struct OCIIntervalYearToMonth   {}
pub struct OCIIntervalDayToSecond   {}

/// Marker trait for OCI handles and descriptors
pub trait OCIStruct {}

macro_rules! mark_as_oci {
    ($($t:ty),+) => {
        $(
            impl OCIStruct for $t {}
        )+
    };
}

mark_as_oci!(OCIEnv, OCIError, OCISvcCtx, OCIStmt, OCIBind, OCIDefine, OCIDescribe, OCIServer, OCISession, OCIRaw);
mark_as_oci!(OCIResult, OCILobLocator, OCILobRegion, OCIParam, OCIRowid, OCIDateTime, OCIInterval, OCIString);
mark_as_oci!(OCICLobLocator, OCIBLobLocator, OCIBFileLocator, OCITimestamp, OCITimestampTZ, OCITimestampLTZ, OCIIntervalYearToMonth, OCIIntervalDayToSecond);

/// C mapping of the Oracle NUMBER
#[repr(C)] pub struct OCINumber {
    pub(crate) bytes: [u8; 22]
}

/// C mapping of the Oracle DATE type (SQLT_ODT)
#[derive(Debug)]
#[repr(C)]
pub struct OCIDate {
    pub(crate) year: i16, // gregorian year: range is -4712 <= year <= 9999
    pub(crate) month: u8, // month: range is 1 <= month <= 12
    pub(crate) day:   u8, // day: range is 1 <= day <= 31
    pub(crate) hour:  u8, // hours: range is 0 <= hours <= 23
    pub(crate) min:   u8, // minutes: range is 0 <= minutes <= 59
    pub(crate) sec:   u8  // seconds: range is 0 <= seconds <= 59
}

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
    pub(crate) fn OCIHandleAlloc(
        parenth:    *mut OCIEnv,
        hndlpp:     *mut *mut  c_void,
        hndl_type:  u32,
        xtramem_sz: size_t,
        usrmempp:   *const c_void
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/handle-and-descriptor-functions.html#GUID-E87E9F91-D3DC-4F35-BE7C-F1EFBFEEBA0A
    pub(crate) fn OCIHandleFree(
        hndlp:      *mut c_void,
        hnd_type:   u32
    ) -> i32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/handle-and-descriptor-functions.html#GUID-E9EF2766-E078-49A7-B1D1-738E4BA4814F
    pub(crate) fn OCIDescriptorAlloc(
        parenth:    *mut OCIEnv,
        descpp:     *mut *mut  c_void,
        desc_type:  u32,
        xtramem_sz: size_t,
        usrmempp:   *const c_void
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/handle-and-descriptor-functions.html#GUID-A32BF051-3DC1-491C-AAFD-A46034DD1629
    pub(crate) fn OCIDescriptorFree(
        descp:      *mut c_void,
        desc_type:  u32
    ) -> i32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/handle-and-descriptor-functions.html#GUID-FA199A99-4D7A-42C2-BB0A-C20047B95DF9
    pub(crate) fn OCIAttrGet(
        trgthndlp:  *const c_void,
        trghndltyp: u32,
        attributep: *mut c_void,
        sizep:      *mut u32,
        attrtype:   u32,
        errhp:      *mut OCIError
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/handle-and-descriptor-functions.html#GUID-3741D7BD-7652-4D7A-8813-AC2AEA8D3B03
    pub(crate) fn OCIAttrSet(
        trgthndlp:  *mut c_void,
        trghndltyp: u32,
        attributep: *const c_void,
        size:       u32,
        attrtype:   u32,
        errhp:      *mut OCIError
    ) -> i32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/handle-and-descriptor-functions.html#GUID-35D2FF91-139B-4A5C-97C8-8BC29866CCA4
    pub(crate) fn OCIParamGet(
        hndlp:      *const c_void,
        htype:      u32,
        errhp:      *mut OCIError,
        descr:      *mut *mut c_void,
        pos:        u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/handle-and-descriptor-functions.html#GUID-280CF9E5-3537-4785-9AFA-4E63DE29A266
    // fn OCIParamSet(
    //     hndlp:      *const c_void,
    //     htype:      u32,
    //     errhp:      *mut OCIError,
    //     descr:      *const c_void,
    //     dtype:      u32,
    //     pos:        u32
    // ) -> i32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/miscellaneous-functions.html#GUID-4B99087C-74F6-498A-8310-D6645172390A
    pub(crate) fn OCIErrorGet(
        hndlp:      *const c_void,
        recordno:   u32,
        sqlstate:   *const c_void,
        errcodep:   *mut i32,
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
    pub(crate) fn OCIServerAttach(
        srvhp:      *mut OCIServer,
        errhp:      *mut OCIError,
        dblink:     *const u8,
        dblink_len: u32,
        mode:       u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/connect-authorize-and-initialize-functions.html#GUID-402B540A-05FF-464B-B9C8-B2E7B4ABD564
    pub(crate) fn OCIServerDetach(
        srvhp:      *mut OCIServer,
        errhp:      *mut OCIError,
        mode:       u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/connect-authorize-and-initialize-functions.html#GUID-31B1FDB3-056E-4AF9-9B89-8DA6AA156947
    pub(crate) fn OCISessionBegin(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        userhp:     *mut OCISession,
        credt:      u32,
        mode:       u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/connect-authorize-and-initialize-functions.html#GUID-2AE88BDC-2C44-4958-B26A-434B0407F06F
    pub(crate) fn OCISessionEnd(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        userhp:     *mut OCISession,
        mode:       u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/transaction-functions.html#GUID-DDAE3122-8769-4A30-8D78-EB2A3CCF77D4
    pub(crate) fn OCITransCommit(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        flags:      u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/transaction-functions.html#GUID-06EF9A0A-01A3-40CE-A0B7-DF0504A93366
    pub(crate) fn OCITransRollback(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        flags:      u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/miscellaneous-functions.html#GUID-033BF96D-D88D-4F18-909A-3AB7C2F6C70F
    pub(crate) fn OCIPing(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        mode:       u32
    ) -> i32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/statement-functions.html#GUID-E6C1DC67-D464-4D2A-9F19-737423D31779
    pub(crate) fn OCIStmtPrepare2(
        svchp:      *mut OCISvcCtx,
        stmthp:     *mut *mut OCIStmt,
        errhp:      *mut OCIError,
        stmttext:   *const u8,
        stmt_len:   u32,
        key:        *const u8,
        keylen:     u32,
        language:   u32,
        mode:       u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/statement-functions.html#GUID-256034CE-2ADB-4BE5-BC8D-748307F2EA8E
    pub(crate) fn OCIStmtRelease(
        stmtp:      *mut OCIStmt,
        errhp:      *mut OCIError,
        key:        *const u8,
        keylen:     u32,
        mode:       u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/bind-define-describe-functions.html#GUID-87D50C09-F18D-45BB-A8AF-1E6AFEC6FE2E
    pub(crate) fn OCIStmtGetBindInfo(
        stmtp:      *mut OCIStmt,
        errhp:      *mut OCIError,
        size:       u32,
        startloc:   u32,
        found:      *mut i32,
        bvnp:       *const *mut u8,
        bvnl:       *mut u8,
        invp:       *const *mut u8,
        invl:       *mut u8,
        dupl:       *mut u8,
        hndl:       *const *mut OCIBind
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/bind-define-describe-functions.html#GUID-CD63DF78-2178-4727-A896-B9673C4A37F0
    // fn OCIBindByName2(
    //     stmtp:      *mut OCIStmt,
    //     bindpp:     *mut *mut OCIBind,
    //     errhp:      *mut OCIError,
    //     namep:      *const u8,
    //     name_len:   i32,
    //     valuep:     *mut c_void,
    //     value_sz:   i64,
    //     dty:        u16,
    //     indp:       *mut c_void,
    //     alenp:      *mut u32,
    //     rcodep:     *mut u16,
    //     maxarr_len: u32,
    //     curelep:    *mut u32,
    //     mode:       u32
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/bind-define-describe-functions.html#GUID-D28DF5A7-3C75-4E52-82F7-A5D6D5714E69
    pub(crate) fn OCIBindByPos2(
        stmtp:      *mut OCIStmt,
        bindpp:     *mut *mut OCIBind,
        errhp:      *mut OCIError,
        position:   u32,
        valuep:     *mut c_void,
        value_sz:   i64,
        dty:        u16,
        indp:       *mut c_void,
        alenp:      *mut u32,
        rcodep:     *mut u16,
        maxarr_len: u32,
        curelep:    *mut u32,
        mode:       u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/bind-define-describe-functions.html#GUID-030270CB-346A-412E-B3B3-556DD6947BE2
    // fn OCIBindDynamic(
    //     bindp:      *mut OCIBind,
    //     errhp:      *mut OCIError,
    //     ictxp:      *mut c_void,
    //     icbfp:      OCICallbackInBind,
    //     octxp:      *mut c_void,
    //     ocbfp:      OCICallbackOutBind
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/statement-functions.html#GUID-98B26708-3E02-45C0-8258-5D5544F32BE9
    pub(crate) fn OCIStmtExecute(
        svchp:      *mut OCISvcCtx,
        stmtp:      *mut OCIStmt,
        errhp:      *mut OCIError,
        iters:      u32,
        rowoff:     u32,
        snap_in:    *const c_void,  // *const OCISnapshot
        snap_out:   *mut c_void,    // *mut OCISnapshot
        mode:       u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/statement-functions.html#GUID-60B998F9-F213-43BA-AB84-76F1EC6A6687
    pub(crate) fn OCIStmtGetNextResult(
        stmtp:      *mut OCIStmt,
        errhp:      *mut OCIError,
        result:     *mut *mut OCIStmt,
        rtype:      *mut u32,
        mode:       u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/statement-functions.html#GUID-DF585B90-58BA-45FC-B7CE-6F7F987C03B9
    pub(crate) fn OCIStmtFetch2(
        stmtp:      *mut OCIStmt,
        errhp:      *mut OCIError,
        nrows:      u32,
        orient:     u16,
        offset:     i16,
        mode:       u32
    ) -> i32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/bind-define-describe-functions.html#GUID-74939FB5-919E-4D24-B327-AFB532435061
    pub(crate) fn OCIDefineByPos2(
        stmtp:      *mut OCIStmt,
        defnpp:     *mut *mut OCIDefine,
        errhp:      *mut OCIError,
        position:   u32,
        valuep:     *mut c_void,
        value_sz:   i64,
        dty:        u16,
        indp:       *mut i16,
        rlenp:      *mut u32,
        rcodep:     *mut u16,
        mode:       u32
    ) -> i32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/miscellaneous-functions.html#GUID-064F2680-453A-40D1-9C36-518F1E2B31DF
    pub(crate) fn OCIRowidToChar(
        desc:   *mut OCIRowid,
        text:   *mut u8,
        size:   *mut u16,
        err:    *mut OCIError,
    ) -> i32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-84EA4A66-27BF-470C-8464-3DE31937702A
    // fn OCIDurationBegin(
    //     envhp:      *mut OCIEnv,
    //     errhp:      *mut OCIError,
    //     svchp:      *const OCISvcCtx,
    //     parent:     u16,
    //     duration:   *mut u16
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-AABC3F29-C91B-45A7-AF1E-D486C12E4962
    // fn OCIDurationEnd(
    //     envhp:      *mut OCIEnv,
    //     errhp:      *mut OCIError,
    //     svchp:      *const OCISvcCtx,
    //     duration:   u16
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-5B43FC88-A649-4764-8C1E-6D792F05F7CE
    pub(crate) fn OCILobAppend(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        dst:        *mut OCILobLocator,
        src:        *const OCILobLocator,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-9B25760D-649E-4B83-A0AA-8C4F3C479BC8
    pub(crate) fn OCILobCharSetForm(
        envhp:      *mut OCIEnv,
        errhp:      *mut OCIError,
        src:        *const OCILobLocator,
        csform:     *mut u8
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-A243691D-8180-4AF6-AA6E-DF9333F8258B
    pub(crate) fn OCILobCharSetId(
        envhp:      *mut OCIEnv,
        errhp:      *mut OCIError,
        src:        *const OCILobLocator,
        csid:       *mut u16
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-CBEB9238-6B47-4A08-8C8D-FC2E5ED56557
    pub(crate) fn OCILobClose(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-404C8A50-516F-4DFD-939D-646A232AF7DF
    pub(crate) fn OCILobCopy2(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        dst:        *mut OCILobLocator,
        src:        *mut OCILobLocator,
        amount:     u64,
        dst_off:    u64,
        src_off:    u64,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-63F75EC5-EB14-4E25-B593-270FF814615A
    pub(crate) fn OCILobCreateTemporary(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
        csid:       u16,
        csfrm:      u8,
        lob_type:   u8,
        cache:      u8,
        duration:   u16,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-264797B2-B3EA-4F6D-9A0E-BF8A4DDA13FA
    pub(crate) fn OCILobErase2(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
        amount:     *mut u64,
        offset:     u64,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-40AFA7A3-3A24-4DF7-A719-AECA7C1F522A
    pub(crate) fn OCILobFileClose(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        filep:      *mut OCILobLocator,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-977F905D-DAFB-4D88-8FE0-7A345837B147
    pub(crate) fn OCILobFileExists(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        filep:      *mut OCILobLocator,
        flag:       *mut u8
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-BF637A34-B18A-47EE-A060-93C4E79D1813
    pub(crate) fn OCILobFileGetName(
        envhp:      *mut OCIEnv,
        errhp:      *mut OCIError,
        loc:        *const OCILobLocator,
        dir:        *mut u8,
        dir_len:    *mut u16,
        filename:   *mut u8,
        name_len:   *mut u16,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-A662166C-DC74-40B4-9BFA-8D3ED216FDE7
    pub(crate) fn OCILobFileIsOpen(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        filep:      *mut OCILobLocator,
        flag:       *mut u8
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-2E933BBA-BCE3-41F2-B8A2-4F9485F0BCB0
    pub(crate) fn OCILobFileOpen(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        filep:      *mut OCILobLocator,
        mode:       u8
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-507AC0EF-4CAB-437E-BB94-1FD77EDC1B5C
    pub(crate) fn OCILobFileSetName(
        envhp:      *mut OCIEnv,
        errhp:      *mut OCIError,
        filepp:     *mut *mut OCILobLocator,
        dir:        *const u8,
        dir_len:    u16,
        filename:   *const u8,
        name_len:   u16,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-E0FBF017-1B08-410C-9E53-F6E14008813A
    pub(crate) fn OCILobFreeTemporary(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-ABB71585-172E-4F3E-A0CF-F70D709F2072
    pub(crate) fn OCILobGetChunkSize(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
        size:       *mut u32,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-D62200EF-FA60-4788-950F-0C0686D807FD
    pub(crate) fn OCILobGetContentType(
        envhp:      *mut OCIEnv,
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
        ctx_type:   *mut u8,
        len:        *mut u32,
        mode:       u32
    )-> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-9BC0A78A-37CB-432F-AE2B-22C905608C4C
    pub(crate) fn OCILobGetLength2(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
        len:        *mut u64,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-5142710F-03AD-43D5-BBAB-6732B874E52E
    pub(crate) fn OCILobIsEqual(
        envhp:      *mut OCIEnv,
        loc1:       *const OCILobLocator,
        loc2:       *const OCILobLocator,
        flag:       *mut u8,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-FFF883CE-3B99-4319-A81C-A11F8740209E
    pub(crate) fn OCILobIsOpen(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
        flag:       *mut u8,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-071D8134-F9E7-4C5A-8E63-E90831FA7AC3
    pub(crate) fn OCILobIsTemporary(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
        flag:       *mut u8,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-DA1CD18B-7044-4E40-B1F4-4FCC1FCAB6C4
    pub(crate) fn OCILobLoadFromFile2(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        dst:        *mut OCILobLocator,
        src:        *mut OCILobLocator,
        amount:     u64,
        dst_off:    u64,
        src_off:    u64,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-F7887376-4B3C-430C-94A3-11FE96E26627
    pub(crate) fn OCILobLocatorAssign(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        src:        *const OCILobLocator,
        dst:        *const *mut OCILobLocator,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-4CA17A83-795F-43B2-8B76-611B13E4C8DE
    pub(crate) fn OCILobLocatorIsInit(
        envhp:      *mut OCIEnv,
        errhp:      *mut OCIError,
        src:        *const OCILobLocator,
        flag:       *mut u8,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-B007A3C7-999B-4AD7-8BF7-C6D14572F470
    pub(crate) fn OCILobOpen(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
        mode:       u8,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-6AC6E6DA-236B-4BF9-942F-9FCC4178FEDA
    pub(crate) fn OCILobRead2(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
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
    pub(crate) fn OCILobSetContentType(
        envhp:      *mut OCIEnv,
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
        ctype:      *const u8,
        len:        u32,
        mode:       u32,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-ABDB1543-1782-4216-AD80-55FA82CFF733
    pub(crate) fn OCILobTrim2(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
        len:        u64,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-77F056CA-9EEE-4550-8A8E-0155DF994DBE
    pub(crate) fn OCILobWrite2(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
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
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-87D3275A-B042-4991-B261-AB531BB83CA2
    pub(crate) fn OCILobWriteAppend2(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
        byte_cnt:   *mut u64,
        char_cnt:   *mut u64,
        buf:        *const u8,
        buf_len:    u64,
        piece:      u8,
        ctx:        *mut c_void,
        write_cb:   *const c_void,
        csid:       u16,
        csfrm:      u8,
    ) -> i32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-E0890180-8714-4243-A585-0FD21EB05CA9
    pub(crate) fn OCIDateAddDays(
        env:        *mut OCIError,
        date:       *const OCIDate,
        num_days:   i32,
        result:     *mut OCIDate
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-CE37ECF1-622A-49A9-A9FD-40E1BD67C941
    pub(crate) fn OCIDateAddMonths(
        env:        *mut OCIError,
        date:       *const OCIDate,
        num_months: i32,
        result:     *mut OCIDate
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-2251373B-4F7B-4680-BB90-F9013216465A
    pub(crate) fn OCIDateAssign(
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
    pub(crate) fn OCIDateCompare(
        env:        *mut OCIError,
        date1:      *const OCIDate,
        date2:      *const OCIDate,
        result:     *mut i32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-42422C47-805F-4EAA-BF44-E6DE6164082E
    pub(crate) fn OCIDateDaysBetween(
        env:        *mut OCIError,
        date1:      *const OCIDate,
        date2:      *const OCIDate,
        result:     *mut i32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-EA8FEB07-401C-477E-805B-CC9E89FB13F4
    pub(crate) fn OCIDateFromText(
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
    pub(crate) fn OCIDateLastDay(
        env:        *mut OCIError,
        date:       *const OCIDate,
        result:     *mut OCIDate
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-A16AB88E-A3BF-4B50-8FEF-6427926198F4
    pub(crate) fn OCIDateNextDay(
        env:        *mut OCIError,
        date:       *const OCIDate,
        day:        *const u8,
        day_len:    u32,
        result:     *mut OCIDate
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-123DD789-48A2-4AD7-8B1E-5E454DFE3F1E
    pub(crate) fn OCIDateToText(
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
    pub(crate) fn OCIDateSysDate(
        env:        *mut OCIError,
        result:     *mut OCIDate
    ) -> i32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-0E4AF4DD-5EEB-434D-BA3A-F4EDE7038FF5
    pub(crate) fn OCIIntervalAdd(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        addend1:    *const OCIInterval,
        addend2:    *const OCIInterval,
        result:     *mut OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-A218E261-3D40-4B69-AD64-41B697A18C98
    pub(crate) fn OCIIntervalAssign(
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
    pub(crate) fn OCIIntervalCompare(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        inter1:     *const OCIInterval,
        inter2:     *const OCIInterval,
        result:     *mut i32,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-16880D01-45BE-43A3-9CF2-AEAE07B64A6B
    pub(crate) fn OCIIntervalDivide(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        dividend:   *const OCIInterval,
        divisor:    *const OCINumber,
        result:     *mut OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-1F8A4B39-9EA5-4CEF-9468-079E4203B68D
    pub(crate) fn OCIIntervalFromNumber(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        interval:   *mut OCIInterval,
        number:     *const OCINumber,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-247BB9B8-307B-4132-A1ED-5CA658B0DAA6
    pub(crate) fn OCIIntervalFromText(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        inpstring:  *const u8,
        str_len:    size_t,
        result:     *mut OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-12B19818-0001-42F1-8B2C-FD96B7C3231C
    pub(crate) fn OCIIntervalFromTZ(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        inpstring:  *const u8,
        str_len:    size_t,
        result:     *mut OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-210C4C25-3E8D-4F6D-9502-20B258DACA60
    pub(crate) fn OCIIntervalGetDaySecond(
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
    pub(crate) fn OCIIntervalGetYearMonth(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        yr:         *mut i32,
        mnth:       *mut i32,
        interval:   *const OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-4DBA1745-E675-4774-99AB-DEE2A1FC3788
    pub(crate) fn OCIIntervalMultiply(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        inter:      *const OCIInterval,
        nfactor:    *const OCINumber,
        result:     *mut OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-303A876B-E1EA-4AF8-8BD1-FC133C5F3F84
    pub(crate) fn OCIIntervalSetDaySecond(
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
    pub(crate) fn OCIIntervalSetYearMonth(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        yr:         i32,
        mnth:       i32,
        result:     *mut OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-2D0465BC-B8EA-4F41-B200-587F49D0B2CB
    pub(crate) fn OCIIntervalSubtract(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        minuend:    *const OCIInterval,
        subtrahend: *const OCIInterval,
        result:     *mut OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-7B403C69-F618-42A6-94F3-41FB17F7F0AD
    pub(crate) fn OCIIntervalToNumber(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        interval:   *const OCIInterval,
        number:     *mut OCINumber,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-DC306081-C4C3-48F5-818D-4C02DD945192
    pub(crate) fn OCIIntervalToText(
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

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-61FB0D0F-6EA7-45DD-AF40-310D86FB8BAE
    pub(crate) fn OCINumberAbs(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-F3DC6DF6-9110-4BAC-AB97-DC604CA04BCD
    pub(crate) fn OCINumberAdd(
        err:      *mut OCIError,
        number1:  *const OCINumber,
        number2:  *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-E7A8B43C-F8B0-4009-A770-94CD7E13EE75
    pub(crate) fn OCINumberArcCos(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-3956D4AC-62E5-41FD-BA48-2DA89E207259
    pub(crate) fn OCINumberArcSin(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-43E9438C-AA74-4392-889D-171F411EBBE2
    pub(crate) fn OCINumberArcTan(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-62C977EF-DB7E-457F-847A-BF0D46E36CD5
    pub(crate) fn OCINumberArcTan2(
        err:      *mut OCIError,
        number1:  *const OCINumber,
        number2:  *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-0C78F351-550E-48F0-8D4C-A9AD8A28DA66
    pub(crate) fn OCINumberAssign(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-48974097-47D4-4757-A627-4E09406AAFD5
    pub(crate) fn OCINumberCeil(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-554A4409-946B-47E9-B239-4140B8F3D1F9
    pub(crate) fn OCINumberCmp(
        err:      *mut OCIError,
        number1:  *const OCINumber,
        number2:  *const OCINumber,
        result:   *mut i32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-150F3245-ECFC-4352-AA73-AAF29BC6A74C
    pub(crate) fn OCINumberCos(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-370FD18E-47D3-4110-817C-658A2F059361
    pub(crate) fn OCINumberDec(
        err:      *mut OCIError,
        number:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-36A6C0EA-85A4-44EE-8489-FB7DB4257513
    pub(crate) fn OCINumberDiv(
        err:      *mut OCIError,
        number1:  *const OCINumber,
        number2:  *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-B56F44FC-158A-420B-830E-FB82894A62C8
    pub(crate) fn OCINumberExp(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-CF35CBDF-DC88-4E86-B586-0EEFD35C0458
    pub(crate) fn OCINumberFloor(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-E8940E06-F4EF-4172-AEE5-AF8E4F6B3AEE
    // fn OCINumberFromInt(
    //     err:      *mut OCIError,
    //     inum:     *const c_void,
    //     inum_len: u32,
    //     sign_typ: u32,
    //     number:   *mut OCINumber
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-EC8E2C9E-BCD2-4D1E-A052-3E657B552461
    pub(crate) fn OCINumberFromReal(
        err:      *mut OCIError,
        rnum:     *const c_void,
        rnum_len: u32,              // sizeof(float | double | long double)
        number:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-F2E458B5-BECC-482E-9223-B92BC696CA17
    pub(crate) fn OCINumberFromText(
        err:      *mut OCIError,
        txt:      *const u8,
        txt_len:  u32,
        fmt:      *const u8,
        fmt_len:  u32,
        nls_par:  *const u8,
        nls_len:  u32,
        number:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-08CCC2C4-5AB3-45EB-9E0D-28186A2AA234
    pub(crate) fn OCINumberHypCos(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-E7391F43-2DFB-4146-9AB7-816D009F31E5
    pub(crate) fn OCINumberHypSin(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-4254930A-DCDC-4590-8710-AC46EC4F3473
    pub(crate) fn OCINumberHypTan(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-A3B07A3A-7E18-421E-9085-BE4B3E742C83
    pub(crate) fn OCINumberInc(
        err:      *mut OCIError,
        number:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-D5CF4199-D6D2-4D31-A914-FB74F5BC5412
    pub(crate) fn OCINumberIntPower(
        err:      *mut OCIError,
        base:     *const OCINumber,
        exp:      i32,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-F1254BAD-7236-4728-A9DA-B8701D8BAA14
    pub(crate) fn OCINumberIsInt(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut i32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-40F344FC-3ED0-4893-AFB1-0853D02D79C9
    pub(crate) fn OCINumberIsZero(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut i32          // set to TRUE if equal to zero else FALSE
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-C1E572F2-F68D-4AF4-831A-2095BFEDDBC3
    pub(crate) fn OCINumberLn(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-561769B0-B559-44AA-8012-985EA7ADFB47
    pub(crate) fn OCINumberLog(
        err:      *mut OCIError,
        base:     *const OCINumber,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-B5DAB7F2-6AC6-4693-8F04-8C13F9538CE9
    pub(crate) fn OCINumberMod(
        err:      *mut OCIError,
        number1:  *const OCINumber,
        number2:  *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-8AAAC840-3776-4283-9DC5-5764CAC2359A
    pub(crate) fn OCINumberMul(
        err:      *mut OCIError,
        number1:  *const OCINumber,
        number2:  *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-8810FFCB-51E7-4890-B551-61BE85624764
    pub(crate) fn OCINumberNeg(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-E755AD46-4285-4DAF-B2A5-886333A2395D
    pub(crate) fn OCINumberPower(
        err:      *mut OCIError,
        base:     *const OCINumber,
        exp:      *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-BE4B0E6D-75B6-4256-A355-9DFAFEC477C9
    pub(crate) fn OCINumberPrec(
        err:      *mut OCIError,
        number:   *const OCINumber,
        num_dig:  i32,              // number of decimal digits desired in the result
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-F3B89623-73E3-428F-A677-5526AC5F4622
    pub(crate) fn OCINumberRound(
        err:      *mut OCIError,
        number:   *const OCINumber,
        num_dig:  i32,              // number of decimal digits to the right of the decimal point to round to. Negative values are allowed.
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-FA067559-D0F7-426D-940A-1D24F4C60C70
    pub(crate) fn OCINumberSetPi(
        err:      *mut OCIError,
        number:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-8152D558-61D9-49F4-9113-DA1455BB5C72
    pub(crate) fn OCINumberSetZero(
        err:      *mut OCIError,
        number:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-EA7D0DA0-A154-4A87-8215-E5B5A7D091E3
    pub(crate) fn OCINumberShift(
        err:      *mut OCIError,
        number:   *const OCINumber,
        num_dec:  i32,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-A535F6F1-0689-4FE1-9C07-C8D341582622
    pub(crate) fn OCINumberSign(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut i32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-65293408-5AF2-4A0C-9C51-82C1C929EE54
    pub(crate) fn OCINumberSin(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-9D68D274-B18C-43F4-AB37-BB99C9062B3E
    pub(crate) fn OCINumberSqrt(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-192725C3-8F5C-4D0A-848E-4EE9690F4A4E
    pub(crate) fn OCINumberSub(
        err:      *mut OCIError,
        number1:  *const OCINumber,
        number2:  *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-1EB45341-6026-47AD-A2EF-D92A20A46ECF
    pub(crate) fn OCINumberTan(
        err:      *mut OCIError,
        number:   *const OCINumber,
        result:   *mut OCINumber
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-067F138E-E689-4922-9ED7-4A7B0E46447E
    // fn OCINumberToInt(
    //     err:      *mut OCIError,
    //     number:   *const OCINumber,
    //     res_len:  u32,
    //     sign_typ: u32,
    //     result:   *mut c_void
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-76C4BC1E-EC64-4CF6-82A4-94D5DC242649
    pub(crate) fn OCINumberToReal(
        err:      *mut OCIError,
        number:   *const OCINumber,
        res_len:  u32,              // sizeof( float | double | long double)
        result:   *mut c_void
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-A850D4E3-2B7B-4DFE-A3E9-618515DACA9E
    // fn OCINumberToRealArray(
    //     err:      *mut OCIError,
    //     numbers:  &*const OCINumber,
    //     elems:    u32,
    //     res_len:  u32,              // sizeof( float | double | long double)
    //     result:   *mut c_void
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-129A5433-6927-43B7-A10F-5FE6AA354232
    pub(crate) fn OCINumberToText(
        err:      *mut OCIError,
        number:   *const OCINumber,
        fmt:      *const u8,
        fmt_len:  u32,
        nls_par:  *const u8,
        nls_len:  u32,
        buf_size: *mut u32,
        buf:      *mut u8
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-NUMBER-functions.html#GUID-FD8D2A9A-222B-4A0E-B4E3-99588FF19BCA
    pub(crate) fn OCINumberTrunc(
        err:      *mut OCIError,
        number:   *const OCINumber,
        num_dig:  i32,
        result:   *mut OCINumber
    ) -> i32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-raw-functions.html#GUID-4856A258-8883-4470-9881-51F27FA050F6
    pub(crate) fn OCIRawAllocSize(
        env:        *mut OCIEnv,
        err:        *mut OCIError,
        raw:        *const OCIRaw,
        size:       *mut u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-raw-functions.html#GUID-3BB4239F-8579-4CC1-B76F-0786BDBAEF9A
    pub(crate) fn OCIRawAssignBytes(
        env:        *mut OCIEnv,
        err:        *mut OCIError,
        rhs:        *const u8,
        rhs_len:    u32,
        lhs:        *mut *mut OCIRaw
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-raw-functions.html#GUID-27DBFBE0-4511-4B34-8476-B9AC720E3F51
    pub(crate) fn OCIRawAssignRaw(
        env:        *mut OCIEnv,
        err:        *mut OCIError,
        rhs:        *const OCIRaw,
        lhs:        *mut *mut OCIRaw
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-raw-functions.html#GUID-B05C44C5-7168-438B-AC2A-BD3AD309AAEA
    pub(crate) fn OCIRawPtr(
        env:        *mut OCIEnv,
        raw:        *const OCIRaw
    ) -> *mut u8;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-raw-functions.html#GUID-7D757B00-DF25-4F61-A3DF-8C72F18FDC9E
    pub(crate) fn OCIRawResize(
        env:        *mut OCIEnv,
        err:        *mut OCIError,
        size:       u32,
        raw:        *mut *mut OCIRaw
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-raw-functions.html#GUID-D74E75FA-5985-4DDC-BC25-430B415B8837
    pub(crate) fn OCIRawSize(
        env:        *mut OCIEnv,
        raw:        *const OCIRaw
    ) -> u32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-3B02C8CC-F35C-422F-B35C-47765C998E57
    pub(crate) fn OCIDateTimeAssign (
        hndl:       *mut c_void,
        err:        *mut OCIError,
        from:       *const OCIDateTime,
        to:         *mut OCIDateTime
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-5C2A63E3-85EC-4346-A636-33B9B4CCBA41
    // fn OCIDateTimeCheck (
    //     hndl:       *mut c_void,
    //     err:        *mut OCIError,
    //     date:       *const OCIDateTime,
    //     result:     *mut u32
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-5FFD4B08-30E1-461E-8E55-940787D6D8EC
    pub(crate) fn OCIDateTimeCompare (
        hndl:       *mut c_void,
        err:        *mut OCIError,
        date1:      *const OCIDateTime,
        date2:      *const OCIDateTime,
        result:     *mut i32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-FC053036-BE93-42D7-A82C-4DDB6843E167
    pub(crate) fn OCIDateTimeConstruct (
        hndl:       *mut c_void,
        err:        *mut OCIError,
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
    pub(crate) fn OCIDateTimeConvert (
        hndl:       *mut c_void,
        err:        *mut OCIError,
        indate:     *const OCIDateTime,
        outdate:    *mut OCIDateTime
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-16189076-75E9-4B46-B418-89CD8DDB42EA
    // fn OCIDateTimeFromArray(
    //     hndl:       *mut c_void,
    //     err:        *mut OCIError,
    //     inarray:    *const u8,
    //     len:        u32,
    //     dt_type:    u8,
    //     datetime:   *mut OCIDateTime,
    //     reftz:      *const OCIInterval,
    //     fsprec:     u8
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-1A453A79-4EEF-462D-B4B3-45820F9EEA4C
    pub(crate) fn OCIDateTimeFromText(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        date_str:   *const u8,
        dstr_length: size_t,
        fmt:        *const u8,
        fmt_length: u8,
        lang_name:  *const u8,
        lang_length: size_t,
        datetime:   *mut OCIDateTime,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-FE6F9482-913D-43FD-BE5A-FCD9FA7B83AD
    pub(crate) fn OCIDateTimeGetDate(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        datetime:   *const OCIDateTime,
        year:       *mut i16,
        month:      *mut u8,
        day:        *mut u8,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-D935ABA2-DEEA-4ABA-AA9C-C27E3E5AC1FD
    pub(crate) fn OCIDateTimeGetTime(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        datetime:   *const OCIDateTime,
        hour:       *mut u8,
        min:        *mut u8,
        sec:        *mut u8,
        fsec:       *mut u32,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-489C51F6-43DB-43DB-980F-2A42AFAFB332
    pub(crate) fn OCIDateTimeGetTimeZoneName(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        datetime:   *const OCIDateTime,
        buf:        *mut u8,
        buflen:     *mut u32,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-B8DA860B-FD7D-481B-8347-156969B6EE04
    pub(crate) fn OCIDateTimeGetTimeZoneOffset(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        datetime:   *const OCIDateTime,
        hour:       *mut i8,
        min:        *mut i8,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-810C6FB3-9B81-4A7C-9B5B-5D2D93B781FA
    pub(crate) fn OCIDateTimeIntervalAdd(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        datetime:   *const OCIDateTime,
        inter:      *const OCIInterval,
        outdatetime: *mut OCIDateTime,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-DEDBFEF5-52DD-4036-93FE-C21B6ED4E8A5
    pub(crate) fn OCIDateTimeIntervalSub(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        datetime:   *const OCIDateTime,
        inter:      *const OCIInterval,
        outdatetime: *mut OCIDateTime,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-BD2F6432-81FF-4CD6-9C3D-85E401894528
    pub(crate) fn OCIDateTimeSubtract(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        indate1:    *const OCIDateTime,
        indate2:    *const OCIDateTime,
        inter:      *mut OCIInterval,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-086776F8-1153-417D-ABC6-A864A2A62788
    pub(crate) fn OCIDateTimeSysTimeStamp(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        sys_date:   *mut OCIDateTime,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-DCA1CF9E-AF92-42E1-B784-8BFC0C9FF8BE
    // fn OCIDateTimeToArray(
    //     hndl:       *mut c_void,
    //     err:        *mut OCIError,
    //     datetime:   *const OCIDateTime,
    //     reftz:      *const OCIInterval,
    //     outarray:   *mut u8,
    //     len:        *mut u32,
    //     fsprec:     u8
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-date-datetime-and-interval-functions.html#GUID-828401C8-8E88-4C53-A66A-24901CCF93C6
    pub(crate) fn OCIDateTimeToText(
        hndl:       *mut c_void,
        err:        *mut OCIError,
        date:       *const OCIDateTime,
        fmt:        *const u8,
        fmt_length: u8,
        fsprec:     u8,
        lang_name:  *const u8,
        lang_length: size_t,
        buf_size:   *mut u32,
        buf:        *mut u8,
    ) -> i32;
}

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-string-functions.html#GUID-3F336010-D8C8-4B50-89CB-ABCCA98905DA
    pub(crate) fn OCIStringAllocSize(
        env:        *mut OCIEnv,
        err:        *mut OCIError,
        txt:        *const OCIString,
        size:       *mut u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-string-functions.html#GUID-58BC140A-900C-4409-B3D2-C2DC8FB643FF
    pub(crate) fn OCIStringAssign(
        env:        *mut OCIEnv,
        err:        *mut OCIError,
        rhs:        *const OCIString,
        lhs:        *mut *mut OCIString
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-string-functions.html#GUID-96E8375B-9017-4E06-BF85-09C12DF286F4
    pub(crate) fn OCIStringAssignText(
        env:        *mut OCIEnv,
        err:        *mut OCIError,
        rhs:        *const u8,
        rhs_len:    u32,
        lhs:        *mut *mut OCIString
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-string-functions.html#GUID-0E1302F7-A32C-46F1-93D7-FB33CF60C24F
    pub(crate) fn OCIStringPtr(
        env:        *mut OCIEnv,
        txt:        *const OCIString
    ) -> *mut u8;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-string-functions.html#GUID-CA52A8A4-08BA-4F08-A4A3-79F841F6AE9E
    pub(crate) fn OCIStringResize(
        env:        *mut OCIEnv,
        err:        *mut OCIError,
        size:       u32,
        txt:        *mut *mut OCIString
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/oci-string-functions.html#GUID-DBDAB2D9-4E78-4752-85B6-55D30CA6AF30
    pub(crate) fn OCIStringSize(
        env:        *mut OCIEnv,
        txt:        *const OCIString
    ) -> u32;
}

