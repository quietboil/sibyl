/*!
    This example demos a single-threaded program that:
     - Connects to the specified database,
     - Prepares an SQL statement,
     - Executes the prepared statement,
     - Fetches the results.

    SQL in this example finds the first person that was hired after the New Year of 2005.

    *Note* that `nonblocking` version of this example is almost a verbatim
    copy of the `blocking` one. The only visible difference - some (async)
    calls are `await`-ed.

    *Note* also that `block_on` used in nonblocking version of this example
    abstracts `block_on` for various async executors and is only intended to
    execute Sibyl's async tests and examples. While you can certainly
    use it, most likely you'd want to create your own version of it.
*/
use sibyl as oracle;

#[cfg(feature="blocking")]
fn main() -> oracle::Result<()> {
    let oracle = oracle::env()?;

    let dbname = std::env::var("DBNAME").expect("database name");
    let dbuser = std::env::var("DBUSER").expect("user name");
    let dbpass = std::env::var("DBPASS").expect("password");

    let session = oracle.connect(&dbname, &dbuser, &dbpass)?;
    let stmt = session.prepare("
        SELECT first_name, last_name, hire_date
          FROM (
                SELECT first_name, last_name, hire_date
                     , Row_Number() OVER (ORDER BY hire_date) hire_date_rank
                  FROM hr.employees
                 WHERE hire_date >= :hire_date
               )
         WHERE hire_date_rank = 1
    ")?;
    let date = oracle::Date::from_string("January 1, 2005", "MONTH DD, YYYY", &session)?;
    let rows = stmt.query(&date)?;

    // The SELECT above will return either 1 or 0 rows, thus `if let` is sufficient.
    // When more than one row is expected, `while let` is used to process rows
    if let Some( row ) = rows.next()? {
        // FIRST_NAME is NULL-able and thus we have to retrieve it as an Option and then
        // explcitly check the returned value.
        let first_name : Option<&str> = row.get("FIRST_NAME")?;
        // LAST_NAME is NOT NULL, thus we can avoid getting an `Option`
        let last_name : &str = row.get_not_null("LAST_NAME")?;
        let hire_date : oracle::Date = row.get_not_null(2)?;

        // Note that the type of `last_name` is `&str`. Similarly, `first_name` is
        // `Option<&str>`. This makes them borrow directly from the internal row
        // buffer. This also restricts their lifetime to the lifetime of the `row`.
        // If the returned value is intended to be used beyond the lifetime of the
        // current row it should be retrieved as a `String`.

        let name = first_name.map_or(last_name.to_string(), |first_name| format!("{} {}", first_name, last_name));
        let hire_date = hire_date.to_string("FMMonth DD, YYYY")?;

        println!("{} was hired on {}", name, hire_date);
    } else {
        println!("No one was hired after {}", date.to_string("FMMonth DD, YYYY")?);
    }
    Ok(())
}

#[cfg(feature="nonblocking")]
fn main() -> oracle::Result<()> {
    sibyl::block_on(async {
        let oracle = oracle::env()?;

        let dbname = std::env::var("DBNAME").expect("database name");
        let dbuser = std::env::var("DBUSER").expect("user name");
        let dbpass = std::env::var("DBPASS").expect("password");

        let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        let stmt = session.prepare("
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
        let rows = stmt.query(&date).await?;
        if let Some( row ) = rows.next().await? {
            let first_name : Option<&str> = row.get("FIRST_NAME")?;
            let last_name : &str = row.get_not_null("LAST_NAME")?;
            let name = first_name.map_or(last_name.to_string(),
                |first_name| format!("{}, {}", last_name, first_name)
            );
            let hire_date : oracle::Date = row.get_not_null("HIRE_DATE")?;
            let hire_date = hire_date.to_string("FMMonth DD, YYYY")?;

            println!("{} was hired on {}", name, hire_date);
        } else {
            println!("No one was hired after {}", date.to_string("FMMonth DD, YYYY")?);
        }
        Ok(())
    })
}
