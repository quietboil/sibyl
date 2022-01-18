# Connections

Use `Environment::connect` method to connect to a database and start a new user session:

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
