use sibyl as oracle;

fn main() -> Result<(),Box<dyn std::error::Error>> {
    let dbname = std::env::var("DBNAME")?;
    let dbuser = std::env::var("DBUSER")?;
    let dbpass = std::env::var("DBPASS")?;

    let oracle = oracle::env()?;
    let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    let stmt = conn.prepare("
        SELECT first_name, last_name, hire_date
          FROM (
                SELECT first_name, last_name, hire_date
                     , row_number() OVER (ORDER BY hire_date) ord
                  FROM hr.employees
                 WHERE hire_date >= :hire_date
               )
         WHERE ord = 1
    ")?;
    let date = oracle::Date::from_string("January 1, 2005", "MONTH DD, YYYY", &oracle)?;
    let rows = stmt.query(&[ &date ])?;
    if let Some( row ) = rows.next()? {
        let last_name = row.get::<&str>(0)?.unwrap();
        let name =
            if let Some( first_name ) = row.get::<&str>(1)? {
                format!("{}, {}", last_name, first_name)
            } else {
                last_name.to_string()
            }
        ;
        let hire_date = row.get::<oracle::Date>(2)?.unwrap();
        let hire_date = hire_date.to_string("FMMonth DD, YYYY")?;

        println!("{} was hired on {}", name, hire_date);
    } else {
        println!("No one was hired after {}", date.to_string("FMMonth DD, YYYY")?);
    }
    Ok(())
}
