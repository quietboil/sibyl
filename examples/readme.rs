/*!
    This example demos a single-threaded program that:
     - Connects to the specified database,
     - Prepares an SQL statement,
     - Executes the prepared statement,
     - Fetches the results.

    SQL in this example finds the first person that
    was hired after the New Year of 2005.

    Note that `current_thread_block_on` used in nonblocking version of this example
    abstracts `block_on` for various executors and is intended to execute async tests
    and examples.
*/
use sibyl as oracle;

#[cfg(feature="blocking")]
fn main() -> oracle::Result<()> {
    let oracle = oracle::env()?;

    let dbname = std::env::var("DBNAME").expect("database name");
    let dbuser = std::env::var("DBUSER").expect("user name");
    let dbpass = std::env::var("DBPASS").expect("password");

    let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    let stmt = conn.prepare("
        SELECT first_name, last_name, hire_date
          FROM (
                SELECT first_name, last_name, hire_date
                     , Row_Number() OVER (ORDER BY hire_date) hire_date_rank
                  FROM hr.employees
                 WHERE hire_date >= :hire_date
               )
         WHERE hire_date_rank = 1
    ")?;
    let date = oracle::Date::from_string("January 1, 2005", "MONTH DD, YYYY", &conn)?;
    let rows = stmt.query(&[ &date ])?;
    // The SELECT above will return either 1 or 0 rows, thus `if let` is sufficient.
    // When more than one row is expected, `while let` should be used to process rows
    if let Some( row ) = rows.next()? {
        // All column values are returned as Options. A not NULL value is returned as Some
        // and NULLs are returned as None. Here, FIRST_NAME is NULL-able and thus we have
        // to retrieve it as an Option and then explcitly check the returned value.
        let first_name : Option<&str> = row.get(0)?;

        // Unlike FIRST_NAME, LAST_NAME is NOT NULL and thus it will never ever be None.
        // NOT NULL column values can be safely unwrapped without checking.
        //
        // Note also that the type of `last_name` is `&str`. This makes it borrow from the
        // internal row buffer. This also restricts its lifetime to the lifetime of the `row`.
        // If the returned value is intended to be used beyond the lifetime of the current
        // row it should be retrieved as a `String`.
        let last_name : &str = row.get(1)?.unwrap();
        let name = first_name.map_or(last_name.to_string(), |first_name| format!("{} {}", first_name, last_name));

        let hire_date : oracle::Date = row.get(2)?.unwrap();
        let hire_date = hire_date.to_string("FMMonth DD, YYYY")?;

        println!("{} was hired on {}", name, hire_date);
    } else {
        println!("No one was hired after {}", date.to_string("FMMonth DD, YYYY")?);
    }
    Ok(())
}

#[cfg(feature="nonblocking")]
fn main() -> oracle::Result<()> {
    sibyl::current_thread_block_on(async {
        let oracle = oracle::env()?;

        let dbname = std::env::var("DBNAME").expect("database name");
        let dbuser = std::env::var("DBUSER").expect("user name");
        let dbpass = std::env::var("DBPASS").expect("password");
    
        let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        let stmt = conn.prepare("
            SELECT first_name, last_name, hire_date
              FROM (
                    SELECT first_name, last_name, hire_date
                         , Row_Number() OVER (ORDER BY hire_date) hire_date_rank
                      FROM hr.employees
                     WHERE hire_date >= :hire_date
                   )
             WHERE hire_date_rank = 1
        ").await?;
        let date = oracle::Date::from_string("January 1, 2005", "MONTH DD, YYYY", &oracle)?;
        let rows = stmt.query(&[ &date ]).await?;
        if let Some( row ) = rows.next().await? {
            let first_name : Option<&str> = row.get("FIRST_NAME")?;
            let last_name : &str = row.get("LAST_NAME")?.unwrap();
            let name = first_name.map_or(last_name.to_string(),
                |first_name| format!("{}, {}", last_name, first_name)
            );
            let hire_date : oracle::Date = row.get("HIRE_DATE")?.unwrap();
            let hire_date = hire_date.to_string("FMMonth DD, YYYY")?;
    
            println!("{} was hired on {}", name, hire_date);
        } else {
            println!("No one was hired after {}", date.to_string("FMMonth DD, YYYY")?);
        }
        Ok(())
    })    
}
