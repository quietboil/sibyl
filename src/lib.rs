#![cfg_attr(not(doctest), doc=include_str!("../README.md"))]

#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(all(feature="blocking",feature="nonblocking",not(docsrs)))]
compile_error!("'blocking' and 'nonblocking' features are exclusive");

#[cfg(feature="nonblocking")]
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
mod task;

mod oci;
mod err;
mod env;
mod types;
mod lob;
mod conn;
mod stmt;

pub use err::Error;
pub use env::Environment;
pub use conn::Connection;
pub use stmt::{Statement, Cursor, Rows, Row, ToSql, ToSqlOut, SqlInArg, SqlOutArg, cols::ColumnType};
pub use types::{Date, Number, Raw, Varchar};
pub use oci::{Cache, CharSetForm};

pub type Result<T>        = std::result::Result<T, Error>;
pub type RowID            = oci::Descriptor<oci::OCIRowid>;
pub type Timestamp<'a>    = types::timestamp::Timestamp<'a, oci::OCITimestamp>;
pub type TimestampTZ<'a>  = types::timestamp::Timestamp<'a, oci::OCITimestampTZ>;
pub type TimestampLTZ<'a> = types::timestamp::Timestamp<'a, oci::OCITimestampLTZ>;
pub type IntervalYM<'a>   = types::interval::Interval<'a, oci::OCIIntervalYearToMonth>;
pub type IntervalDS<'a>   = types::interval::Interval<'a, oci::OCIIntervalDayToSecond>;
pub type CLOB<'a>         = lob::LOB<'a,oci::OCICLobLocator>;
pub type BLOB<'a>         = lob::LOB<'a,oci::OCIBLobLocator>;
pub type BFile<'a>        = lob::LOB<'a,oci::OCIBFileLocator>;

/**
    Returns a new environment handle, which is then used by the OCI functions.

    While there can be multiple environments, most applications most likely will
    need only one.

    As nothing can outlive its environment, when only one environment is used,
    it might be created either in `main` function:

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
