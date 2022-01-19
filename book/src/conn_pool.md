# Connection Pool

[Connection pool][1] can only be used in `blocking` mode applications.

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

    // Connection pool needs to establish an internal session with the database.
    // `dbuser` and `dbpass` here are for that session.
    let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 0, 1, 4)?;
    // This pool has 0 available connections at this time, will create 1 connection
    // at a time when they are needed (session needs a connection to run and there
    // are no available connections in the pool), up to the maximum of 4 connections.
    let pool = Arc::new(pool);

    let mut workers = Vec::new();
    for i in 0..10 {
        let pool = pool.clone();
        let handle = thread::spawn(move || -> Result<()> {
            let dbuser = env::var(format!("DBUSER{}",i)).expect("user name");
            let dbpass = env::var(format!("DBPASS{}",i)).expect("password");
            // Here `dbuser` and `dbpass` are used to create a new database session.
            // While these sessions share pooled connections, they are entirely
            // "owned" by the threads that created them and as such might use
            // different users for authentication.
            let session = pool.get_session(&dbuser, &dbpass)?;
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


[1]: https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/session-and-connection-pooling.html#GUID-1C9A6E8F-EF5A-478D-B65E-CE39D4F00683