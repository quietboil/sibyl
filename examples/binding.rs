#[cfg(feature="blocking")]
fn main() -> sibyl::Result<()> {
    use sibyl::*;

    let dbname = std::env::var("DBNAME").expect("database name");
    let dbuser = std::env::var("DBUSER").expect("user name");
    let dbpass = std::env::var("DBPASS").expect("password");
    let oracle = Environment::new()?;
    let session = oracle.connect(&dbname, &dbuser, &dbpass)?;

    let stmt = session.prepare("
        SELECT country_name
          FROM hr.countries
         WHERE country_id = :COUNTRY_ID
    ")?;
    let row = stmt.query_single("UK")?.unwrap();
    let name : String = row.get(0)?;
    // note that the `name` is not declared mutable

    let stmt = session.prepare("
    BEGIN
        SELECT country_name
          INTO :COUNTRY_NAME
          FROM hr.countries
         WHERE country_id = :COUNTRY_ID
           AND country_name != :COUNTRY_NAME;
    END;
    ")?;
    stmt.execute((
        ("COUNTRY_ID", "NL"),
        ("COUNTRY_NAME", &name),
        // `:COUNTRY_NAME` is INOUT but `name` is bound only for reading
    ))?;
    println!("country_name={name}");
    #[cfg(not(feature="unsafe-direct-binds"))]
    // `name` has not changed despite the binding mistake
    assert_eq!(name, "United Kingdom");
    #[cfg(feature="unsafe-direct-binds")]
    assert_eq!(name, "Netherlandsdom");

    Ok(())
}

#[cfg(feature="nonblocking")]
fn main() {}