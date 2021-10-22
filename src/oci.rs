#![allow(dead_code)]

pub(crate) mod ptr;

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


use libc::{ size_t, c_void };


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
