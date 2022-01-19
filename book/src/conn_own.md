# Connection Per Thread

```rust,noplayground
fn main() -> sibyl::Result<()> {
    let oracle = sibyl::env()?;

    let dbname = std::env::var("DBNAME").expect("database name");
    let dbuser = std::env::var("DBUSER").expect("user name");
    let dbpass = std::env::var("DBPASS").expect("password");

    let session = oracle.connect(&dbname, &dbuser, &dbpass)?;
    // ...
    Ok(())
}
```

Where `dbname` can be any name that is acceptable to Oracle clients - from local TNS name to EZConnect identifier to a connect descriptor.

Or with multiple threads:

```rust,noplayground
use std::{env, thread, sync::Arc};
use sibyl::*;

fn main() -> Result<()> {
    let oracle = sibyl::env()?;
    let oracle = Arc::new(oracle);

    let mut workers = Vec::new();
    for _i in 0..10 {
        let oracle = oracle.clone();
        let handle = thread::spawn(move || -> Result<()> {
            let dbname = env::var("DBNAME").expect("database name");
            let dbuser = env::var("DBUSER").expect("user name");
            let dbpass = env::var("DBPASS").expect("password");

            let session = oracle.connect(&dbname, &dbuser, &dbpass)?;            
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
