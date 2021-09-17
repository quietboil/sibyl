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
    let date = oracle::Date::from_string("January 1, 2005", "MONTH DD, YYYY", &conn)?;
    let rows = stmt.query(&[ &date ])?;
    if let Some( row ) = rows.next()? {
        let first_name : Option<&str> = row.get(0)?;
        let last_name : &str = row.get(1)?.unwrap();
        let name = first_name.map_or(last_name.to_string(), |first_name| format!("{}, {}", last_name, first_name));
        let hire_date : oracle::Date = row.get(2)?.unwrap();
        let hire_date = hire_date.to_string("FMMonth DD, YYYY")?;

        println!("{} was hired on {}", name, hire_date);
    } else {
        println!("No one was hired after {}", date.to_string("FMMonth DD, YYYY")?);
    }
    Ok(())
}
