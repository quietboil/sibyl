# Sibyl

Sibyl is an [OCI][1]-based interface between Rust applications and Oracle databases. Sibyl supports both blocking (threads) and nonblocking (async) API.

[![crates.io](https://img.shields.io/crates/v/sibyl)](https://crates.io/crates/sibyl)
[![Documentation](https://docs.rs/sibyl/badge.svg)](https://docs.rs/sibyl)
![MIT](https://img.shields.io/crates/l/sibyl.svg)

## Example

### Blocking Mode

```rust
use sibyl as oracle; // pun intended :)

fn main() -> Result<(),Box<dyn std::error::Error>> {
    let oracle = oracle::env()?;

    let dbname = std::env::var("DBNAME").expect("database name");
    let dbuser = std::env::var("DBUSER").expect("user name");
    let dbpass = std::env::var("DBPASS").expect("password");

    let session = oracle.connect(&dbname, &dbuser, &dbpass)?;
    let stmt = session.prepare("
        SELECT first_name, last_name, hire_date
          FROM hr.employees
         WHERE hire_date >= :hire_date
      ORDER BY hire_date
    ")?;
    let date = oracle::Date::from_string("January 1, 2005", "MONTH DD, YYYY", &session)?;
    let rows = stmt.query(&date)?;
    while let Some( row ) = rows.next()? {
        let first_name : Option<&str>  = row.get(0)?;
        let last_name  : &str          = row.get(1)?;
        let hire_date  : oracle::Date  = row.get(2)?;

        let hire_date = hire_date.to_string("FMMonth DD, YYYY")?;
        if let Some(first_name) = first_name {
            println!("{}: {} {}", hire_date, first_name, last_name);
        } else {
            println!("{}: {}", hire_date, last_name);
        }
    }
    if stmt.row_count()? == 0 {
        println!("No one was hired after {}", date.to_string("FMMonth DD, YYYY")?);
    }
    Ok(())
}
```

### Nonblocking Mode

```rust
use sibyl as oracle;

#[tokio::main]
async fn main() -> Result<(),Box<dyn std::error::Error>> {
    let oracle = oracle::env()?;

    let dbname = std::env::var("DBNAME").expect("database name");
    let dbuser = std::env::var("DBUSER").expect("user name");
    let dbpass = std::env::var("DBPASS").expect("password");

    let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
    let stmt = session.prepare("
        SELECT first_name, last_name, hire_date
          FROM hr.employees
         WHERE hire_date >= :hire_date
      ORDER BY hire_date
    ").await?;
    let date = oracle::Date::from_string("January 1, 2005", "MONTH DD, YYYY", &oracle)?;
    let rows = stmt.query(&date).await?;
    while let Some( row ) = rows.next().await? {
        let first_name : Option<&str>  = row.get(0)?;
        let last_name  : &str          = row.get(1)?;
        let hire_date  : oracle::Date  = row.get(2)?;

        let hire_date = hire_date.to_string("FMMonth DD, YYYY")?;
        if let Some(first_name) = first_name {
            println!("{}: {} {}", hire_date, first_name, last_name);
        } else {
            println!("{}: {}", hire_date, last_name);
        }
    }
    if stmt.row_count()? == 0 {
        println!("No one was hired after {}", date.to_string("FMMonth DD, YYYY")?);
    }
    Ok(())
}
```

> Note that:
> - The nonblocking mode example is almost a verbatim copy of the blocking mode example with `await`s added.
> - The async example uses and depends on [Tokio][2]
> - For the moment, Sibyl can use only Tokio, Actix, async-std or async-global-executor as an async executor.

# Documentation

- [User Guide](https://quietboil.github.io/sibyl)
- [API](https://docs.rs/sibyl)

[1]: https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/index.html
[2]: https://crates.io/crates/tokio
