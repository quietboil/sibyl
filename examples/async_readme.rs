/*!
This example demos a single-threaded program that:
- Connects to the specified database,
- Prepares an SQL statement,
- Executes the prepared statement,
- Fetches the results.

SQL in this example finds the first person that was hired after the New Year of 2005.

*Note* that `block_on` used in this example abstracts `block_on` for
various async executors and is only intended to execute Sibyl's async
tests and examples. While you can certainly use it, most likely you'd
want to create your own.
*/
#[cfg(feature="nonblocking")]
fn main() -> sibyl::Result<()> {
    sibyl::block_on(async {
        use sibyl as oracle;

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
            if first_name.is_some() {
                println!("{}: {} {}", hire_date, first_name.unwrap(), last_name);
            } else {
                println!("{}: {}", hire_date, last_name);
            }
        }
        if stmt.row_count()? == 0 {
            println!("No one was hired after {}", date.to_string("FMMonth DD, YYYY")?);
        }
        Ok(())
    })
}

#[cfg(feature="blocking")]
fn main() {}