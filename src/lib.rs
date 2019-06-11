//! An OCI-based driver for Rust applications to interface with Oracle databases.

mod defs;
#[macro_use] mod err;
mod handle;
mod desc;
mod attr;
mod param;
mod env;
mod types;
mod tosql;
mod tosqlout;
mod fromsql;
mod conn;
mod stmt;
mod rowid;
mod cursor;
mod column;
mod rows;
mod lob;

/// Returns a new environment handle, which is then used by the OCI functions.
/// 
/// While there can be multiple environments, most application likely will only
/// need one.
///
/// As nothing can outlive the environment at which it was created, when only one
/// environment is used, it might be created in `main`:
/// ```no_run
/// use sibyl as oracle; // pun intended :)
/// fn main() {
///     let oraenv = oracle::env().expect("Oracle OCI environment");
///     // ...
/// }
/// ```
/// Or even statically:
/// ```ignore
/// use sibyl::Environment;
/// lazy_static! {
///     pub static ref ORACLE : Environment = sibyl::env().expect("Oracle OCI environment");
/// }
/// ```
pub fn env() -> Result<Environment> {
    Environment::new()
}

pub(crate) use crate::defs::*;
pub(crate) use crate::handle::Handle;

pub use crate::err::Error;
pub use crate::env::{ Environment, Env };
pub use crate::conn::{ Connection, Conn };
pub use crate::stmt::{ Statement, ColumnType, SqlInArg, SqlOutArg, Stmt };
pub use crate::rows::{ Rows, Row };
pub use crate::cursor::Cursor;
pub use crate::types::number::Number;
pub use crate::types::date::Date;
pub use crate::types::raw::Raw;
pub use crate::types::varchar::Varchar;
pub use crate::tosql::ToSql;
pub use crate::tosqlout::ToSqlOut;
pub use crate::fromsql::FromSql;

pub type Result<T>          = std::result::Result<T, Error>;
pub type Timestamp<'a>      = crate::types::timestamp::Timestamp<'a, OCITimestamp>;
pub type TimestampTZ<'a>    = crate::types::timestamp::Timestamp<'a, OCITimestampTZ>;
pub type TimestampLTZ<'a>   = crate::types::timestamp::Timestamp<'a, OCITimestampLTZ>;
pub type IntervalYM<'a>     = crate::types::interval::Interval<'a, OCIIntervalYearToMonth>;
pub type IntervalDS<'a>     = crate::types::interval::Interval<'a, OCIIntervalDayToSecond>;
pub type CLOB<'a>           = crate::lob::LOB<'a,OCICLobLocator>;
pub type BLOB<'a>           = crate::lob::LOB<'a,OCIBLobLocator>;
pub type BFile<'a>          = crate::lob::LOB<'a,OCIBFileLocator>;
pub type RowID              = crate::desc::Descriptor<OCIRowid>;

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
