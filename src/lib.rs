/*!
Sibyl is an [OCI][1]-based interface between Rust applications and Oracle databases. Sibyl supports both sync (blocking) and async (nonblocking) API.

## Blocking Mode Example

```
# #[cfg(feature="blocking")]
fn main() -> sibyl::Result<()> {
    let oracle = sibyl::env()?;

    let dbname = std::env::var("DBNAME").expect("database name");
    let dbuser = std::env::var("DBUSER").expect("user name");
    let dbpass = std::env::var("DBPASS").expect("password");

    let session = oracle.connect(&dbname, &dbuser, &dbpass)?;

    let stmt = session.prepare("
        SELECT c.country_name, Median(e.salary)
          FROM hr.employees e
          JOIN hr.departments d ON d.department_id = e.department_id
          JOIN hr.locations l   ON l.location_id = d.location_id
          JOIN hr.countries c   ON c.country_id = l.country_id
          JOIN hr.regions r     ON r.region_id = c.region_id
         WHERE r.region_name = :REGION_NAME
      GROUP BY c.country_name
    ")?;

    let rows = stmt.query("Europe")?;

    while let Some(row) = rows.next()? {
        let country_name : &str = row.get(0)?;
        let median_salary : u16 = row.get(1)?;
        println!("{:25}: {:>5}", country_name, median_salary);
    }
    Ok(())
}
# #[cfg(feature="nonblocking")]
# fn main() {}
```

## Nonblocking Mode Example

```
# #[cfg(feature="nonblocking")]
fn main() -> sibyl::Result<()> {
# use sibyl::block_on;
  block_on(async {
    let oracle = sibyl::env()?;

    let dbname = std::env::var("DBNAME").expect("database name");
    let dbuser = std::env::var("DBUSER").expect("user name");
    let dbpass = std::env::var("DBPASS").expect("password");

    let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;

    let stmt = session.prepare("
        SELECT c.country_name, Median(e.salary)
          FROM hr.employees e
          JOIN hr.departments d ON d.department_id = e.department_id
          JOIN hr.locations l   ON l.location_id = d.location_id
          JOIN hr.countries c   ON c.country_id = l.country_id
          JOIN hr.regions r     ON r.region_id = c.region_id
         WHERE r.region_name = :REGION_NAME
      GROUP BY c.country_name
    ").await?;

    let rows = stmt.query("Europe").await?;

    while let Some(row) = rows.next().await? {
        let country_name : &str = row.get(0)?;
        let median_salary : u16 = row.get(1)?;
        println!("{:25}: {:>5}", country_name, median_salary);
    }
    Ok(())
  })
}
# #[cfg(feature="blocking")]
# fn main() {}
```

> Note that `block_on` in the example is an internal abstraction over `block_on` of different async executors. It is intended only to help running Sibyl's own tests and examples.

# Features

Sibyl has 2 main features - `blocking` and `nonblocking`. They are **exclusive** and **one** must be explicitly selected as neither is the default.

Sibyl compiled with a `nonblocking` feature needs to integrate with the async executor that the application uses. At the moment Sybil supports
[Tokio][2], [Actix][3], [async-std][4], and [async-global-executor][5]. One (and only one) of those must be selected together with a `nonblocking` feature:

| Feature | async Runtime |
| ------- | ------------- |
| `tokio` | [Tokio][2] |
| `actix` | [Actix][3] |
| `async-std` | [async-std][4] |
| `async-global` | [async-global-executor][5] |

Thus, for example, when Sibyl is used as a dependency, it might be included as:

```toml
[dependencies]
sibyl = { version = "0.6", features = ["blocking"] }
```

Or, when Sibyl is used in nonblocking mode as:

```toml
[dependencies]
sibyl = { version = "0.6", features = ["nonblocking", "tokio"] }
```

[1]: https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/index.html
[2]: https://crates.io/crates/tokio
[3]: https://crates.io/crates/actix-rt
[4]: https://crates.io/crates/async-std
[5]: https://crates.io/crates/async-global-executor
*/

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
mod nvl;

#[cfg(feature="blocking")]
pub use pool::ConnectionPool;

#[cfg(feature="nonblocking")]
#[doc(hidden)]
pub use task::{spawn, block_on};

pub use err::Error;
pub use env::Environment;
pub use session::Session;
pub use pool::{SessionPool, SessionPoolGetMode};
pub use stmt::{Statement, Cursor, Rows, Row, ToSql, FromSql, ColumnType, Position};
pub use types::{Date, Raw, Number, Varchar, RowID, DateTime, Interval};
pub use types::number::Integer;
pub use oci::{Cache, CharSetForm, client_version};
#[cfg(feature="nonblocking")]
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
pub use oci::futures::NUM_ACTIVE_ASYNC_DROPS;
pub use lob::LOB;
pub use nvl::Nvl;

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

#[doc(hidden)]
pub mod test_env {
    #[cfg(all(feature="blocking",not(docsrs)))]
    mod blocking {
        use once_cell::sync::OnceCell;
        use crate::{Environment, SessionPool, Session, Result};

        static ORACLE : OnceCell<Environment> = OnceCell::new();
        static POOL : OnceCell<SessionPool> = OnceCell::new();

        pub fn get_session() -> Result<Session<'static>> {
            let pool = POOL.get_or_try_init(|| {
                let dbname = std::env::var("DBNAME").expect("database name");
                let dbuser = std::env::var("DBUSER").expect("user name");
                let dbpass = std::env::var("DBPASS").expect("password");    
                let oracle = ORACLE.get_or_try_init(|| Environment::new())?;
                oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10)
            })?;
            pool.get_session()
        }
    }
    #[cfg(all(feature="blocking",not(docsrs)))]
    pub use self::blocking::get_session;

 
    #[cfg(all(feature="nonblocking",not(docsrs)))]
    mod nonblocking {
        use once_cell::sync::OnceCell;
        use async_once_cell::OnceCell as AsyncOnceCell;
        use crate::{Environment, SessionPool, Session, Result};

        static ORACLE : OnceCell<Environment> = OnceCell::new();
        static POOL : AsyncOnceCell<SessionPool> = AsyncOnceCell::new();

        pub async fn get_session() -> Result<Session<'static>> {
            let pool = POOL.get_or_try_init(async {
                let dbname = std::env::var("DBNAME").expect("database name");
                let dbuser = std::env::var("DBUSER").expect("user name");
                let dbpass = std::env::var("DBPASS").expect("password");    
                let oracle = ORACLE.get_or_try_init(|| Environment::new())?;
                oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10).await
            }).await?;
            pool.get_session().await
        }
    }
    #[cfg(all(feature="nonblocking",not(docsrs)))]
    pub use self::nonblocking::get_session;    
}
