# Session Pool

[Session pool][1] can be used in both `blocking` and `nonblocking` mode applicaitons.

## Blocking Mode Pool

```rust,noplayground
use std::{env, thread, sync::Arc};
use once_cell::sync::OnceCell;
use sibyl::*;

fn main() -> sibyl::Result<()> {
    static ORACLE : OnceCell<Environment> = OnceCell::new();
    let oracle = ORACLE.get_or_try_init(|| {
        Environment::new()
    })?;

    let dbname = env::var("DBNAME").expect("database name");
    let dbuser = env::var("DBUSER").expect("user name");
    let dbpass = env::var("DBPASS").expect("password");

    // All sessions will be authenticated with the provided user name and password.
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10)?;
    // This pool has 0 available session at this time, will create 1 session at a
    // time when they are needed (session is requested and there are no available
    // sessions in the pool), up to the maximum of 10 sessions.
    let pool = Arc::new(pool);

    let mut workers = Vec::new();
    for _i in 0..100 {
        let pool = pool.clone();
        let handle = thread::spawn(move || -> Result<()> {
            let session = pool.get_session()?;
            // ...
            Ok(())
        }
        workers.push(handle);
    }
    for handle in workers {
        let _ = handle.join();
    }
    Ok(())
}
```

## Nonblocking Mode Pool

```rust,noplayground
use std::{env, thread, sync::Arc};
use once_cell::sync::OnceCell;
use sibyl::*;

#[tokio::main]
async fn main() -> sibyl::Result<()> {
    static ORACLE : OnceCell<Environment> = OnceCell::new();
    let oracle = ORACLE.get_or_try_init(|| {
        Environment::new()
    })?;

    let dbname = env::var("DBNAME").expect("database name");
    let dbuser = env::var("DBUSER").expect("user name");
    let dbpass = env::var("DBPASS").expect("password");

    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10).await?;
    let pool = Arc::new(pool);

    let mut workers = Vec::new();
    for _i in 0..100 {
        let pool = pool.clone();
        let handle = tokio::task::spawn(async move {
            let session = pool.get_session().await?;
            // ...
            Ok::<_,Error>(())
        }
        workers.push(handle);
    }
    for handle in workers {
        let _ = handle.await;
    }
    Ok(())
}
```

[1]: https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/session-and-connection-pooling.html#GUID-F9662FFB-EAEF-495C-96FC-49C6D1D9625C