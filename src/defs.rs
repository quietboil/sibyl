#![allow(dead_code)]

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

// Attributes

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

// Data types
pub const SQLT_CHR              : u16 = 1;   // (ORANET TYPE) character string
pub const SQLT_NUM              : u16 = 2;   // (ORANET TYPE) oracle numeric
pub const SQLT_INT              : u16 = 3;   // (ORANET TYPE) integer
pub const SQLT_FLT              : u16 = 4;   // (ORANET TYPE) Floating point number
pub const SQLT_STR              : u16 = 5;   // zero terminated string
pub const SQLT_VNU              : u16 = 6;   // NUM with preceding length byte
pub const SQLT_PDN              : u16 = 7;   // (ORANET TYPE) Packed Decimal Numeric
pub const SQLT_LNG              : u16 = 8;   // long
pub const SQLT_VCS              : u16 = 9;   // Variable character string
pub const SQLT_NON              : u16 = 10;  // Null/empty PCC Descriptor entry
pub const SQLT_RID              : u16 = 11;  // rowid
pub const SQLT_DAT              : u16 = 12;  // date in oracle format
pub const SQLT_VBI              : u16 = 15;  // binary in VCS format
pub const SQLT_BFLOAT           : u16 = 21;  // Native Binary float
pub const SQLT_BDOUBLE          : u16 = 22;  // NAtive binary double
pub const SQLT_BIN              : u16 = 23;  // binary data(DTYBIN)
pub const SQLT_LBI              : u16 = 24;  // long binary
pub const SQLT_UIN              : u16 = 68;  // unsigned integer
pub const SQLT_SLS              : u16 = 91;  // Display sign leading separate
pub const SQLT_LVC              : u16 = 94;  // Longer longs (char)
pub const SQLT_LVB              : u16 = 95;  // Longer long binary
pub const SQLT_AFC              : u16 = 96;  // Ansi fixed char
pub const SQLT_AVC              : u16 = 97;  // Ansi Var char
pub const SQLT_IBFLOAT          : u16 = 100; // binary float canonical
pub const SQLT_IBDOUBLE         : u16 = 101; // binary double canonical
pub const SQLT_CUR              : u16 = 102; // cursor  type
pub const SQLT_RDD              : u16 = 104; // rowid descriptor
pub const SQLT_LAB              : u16 = 105; // label type
pub const SQLT_OSL              : u16 = 106; // oslabel type

pub const SQLT_NTY              : u16 = 108; // named object type, a.k.a. user-defined type
pub const SQLT_REF              : u16 = 110; // ref type
pub const SQLT_CLOB             : u16 = 112; // character lob
pub const SQLT_BLOB             : u16 = 113; // binary lob
pub const SQLT_BFILE            : u16 = 114; // binary file lob
pub const SQLT_CFILE            : u16 = 115; // character file lob
pub const SQLT_RSET             : u16 = 116; // result set type
pub const SQLT_NCO              : u16 = 122; // named collection type (varray or nested table)
pub const SQLT_VST              : u16 = 155; // OCIString type
pub const SQLT_ODT              : u16 = 156; // OCIDate type

// datetimes and intervals
pub const SQLT_DATE             : u16 = 184; // ANSI Date
pub const SQLT_TIME             : u16 = 185; // TIME
pub const SQLT_TIME_TZ          : u16 = 186; // TIME WITH TIME ZONE
pub const SQLT_TIMESTAMP        : u16 = 187; // TIMESTAMP
pub const SQLT_TIMESTAMP_TZ     : u16 = 188; // TIMESTAMP WITH TIME ZONE
pub const SQLT_INTERVAL_YM      : u16 = 189; // INTERVAL YEAR TO MONTH
pub const SQLT_INTERVAL_DS      : u16 = 190; // INTERVAL DAY TO SECOND
pub const SQLT_TIMESTAMP_LTZ    : u16 = 232; // TIMESTAMP WITH LOCAL TZ

pub const SQLT_PNTY             : u16 = 241; // pl/sql representation of named types

// some pl/sql specific types
pub const SQLT_REC              : u16 = 250; // pl/sql 'record' (or %rowtype)
pub const SQLT_TAB              : u16 = 251; // pl/sql 'indexed table'
pub const SQLT_BOL              : u16 = 252; // pl/sql 'boolean'

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
