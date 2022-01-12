#![cfg_attr(not(doctest), doc=include_str!("../README.md"))]

#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(all(feature="blocking",feature="nonblocking",not(docsrs)))]
compile_error!("'blocking' and 'nonblocking' features are exclusive");

#[cfg(not(any(feature="blocking",feature="nonblocking")))]
compile_error!("either 'blocking' or 'nonblocking' feature must be explicitly specified");

#[cfg(feature="nonblocking")]
mod task;

mod oci;
mod err;
mod env;
mod session;
mod pool;
mod types;
mod stmt;
mod lob;

#[cfg(feature="blocking")]
pub use pool::ConnectionPool;

#[cfg(feature="nonblocking")]
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
pub use task::{spawn, block_on};

pub use err::Error;
pub use env::Environment;
pub use session::Session;
pub use pool::{SessionPool, SessionPoolGetMode};
pub use stmt::{Statement, Cursor, Rows, Row, ToSql, ToSqlOut, ColumnType, Position};
pub use types::{Date, Raw, Number, Varchar, RowID, DateTime, Interval};
pub use oci::{Cache, CharSetForm};
pub use lob::LOB;

/// A specialized `Result` type for Sibyl.
pub type Result<T>        = std::result::Result<T, Error>;
/// Represents the `TIMESTAMP` data type. It stores year, month, day, hour, minute, second and fractional seconds.
pub type Timestamp<'a>    = types::DateTime<'a, oci::OCITimestamp>;
/// Represents the `TIMESTAMP WITH TIME ZONE` data type. It's a variant of `TIMESTAMP` that includes of a time zone region name or time zone offset in its value.
pub type TimestampTZ<'a>  = types::DateTime<'a, oci::OCITimestampTZ>;
/// Represents the `TIMESTAMP WITH LOCAL TIME ZONE` data type. It's a variant of `TIMESTAMP` that is normalized to the database time zone.
pub type TimestampLTZ<'a> = types::DateTime<'a, oci::OCITimestampLTZ>;
/// Represents `INTERVAL YEAR TO MONTH` data type. It stores a period of time in terms of years and months.
pub type IntervalYM<'a>   = types::Interval<'a, oci::OCIIntervalYearToMonth>;
/// Represents `INTERVAL DAY TO SECOND` data type. It stores a period of time in terms of days, hours, minutes, and seconds.
pub type IntervalDS<'a>   = types::Interval<'a, oci::OCIIntervalDayToSecond>;
/// A character large object locator.
pub type CLOB<'a>         = LOB<'a,oci::OCICLobLocator>;
/// A binary large object locator.
pub type BLOB<'a>         = LOB<'a,oci::OCIBLobLocator>;
/// A locator to a large binary file.
pub type BFile<'a>        = LOB<'a,oci::OCIBFileLocator>;

/**
Returns a new environment handle, which is then used by the OCI functions.

While there can be multiple environments, most applications most likely will
need only one.

As nothing can outlive its environment, when only one environment is used,
it might be created either in the `main` function:

```
use sibyl as oracle; // pun intended :)
fn main() {
    let oracle = oracle::env().expect("Oracle OCI environment");
    // ...
}
```

and passed around, or it might be created statically:

```
use sibyl::{Environment, Result};
use once_cell::sync::OnceCell;

fn oracle() -> Result<&'static Environment> {
    static OCI_ENV: OnceCell<Environment> = OnceCell::new();
    OCI_ENV.get_or_try_init(||
        sibyl::env()
    )
}

fn main() -> Result<()> {
    let oracle = oracle()?;
    // ...
    Ok(())
}
```
*/
pub fn env() -> Result<Environment> {
    Environment::new()
}
