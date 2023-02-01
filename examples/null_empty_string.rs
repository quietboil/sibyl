#[cfg(feature="blocking")]
fn main() -> sibyl::Result<()> {
    use sibyl::*;

    let dbname = std::env::var("DBNAME").expect("database name");
    let dbuser = std::env::var("DBUSER").expect("user name");
    let dbpass = std::env::var("DBPASS").expect("password");
    let oracle = Environment::new()?;
    let session = oracle.connect(&dbname, &dbuser, &dbpass)?;

    let stmt = session.prepare("
        SELECT Nvl(:VAL,'nil') FROM dual
    ")?;
    if let Some(row) = stmt.query_single("")? {
        let val: Option<&str> = row.get(0)?;

        assert!(val.is_some());
        assert_eq!(val.unwrap(), "nil");
    }

    let stmt = session.prepare("
    BEGIN
        IF :VAL IS NULL THEN
            :VAL := 'nil';
        ELSE
            :VAL := NULL;
        END IF;
    END;
    ")?;
    // allocate space for future output
    let mut val = String::with_capacity(4);

    stmt.execute(&mut val)?;
    assert!(!stmt.is_null("VAL")?);
    assert_eq!(val, "nil");

    stmt.execute(&mut val)?;
    assert!(stmt.is_null("VAL")?);
    assert_eq!(val, "");

    Ok(())
}

#[cfg(feature="nonblocking")]
fn main() {}