#[cfg(feature="blocking")]
fn main() -> sibyl::Result<()> {
    use sibyl::*;

    let dbname = std::env::var("DBNAME").expect("database name");
    let dbuser = std::env::var("DBUSER").expect("user name");
    let dbpass = std::env::var("DBPASS").expect("password");
    let oracle = Environment::new()?;
    let session = oracle.connect(&dbname, &dbuser, &dbpass)?;

    let stmt = session.prepare("
    BEGIN
        IF :VAL IS NULL THEN
            :VAL := 42;
        ELSE
            :VAL := NULL;
        END IF;
    END;
    ")?;
    let mut val : Option<i32> = None;

    stmt.execute(&mut val)?;
    assert_eq!(val, Some(42));

    stmt.execute(&mut val)?;
    assert!(val.is_none());

    Ok(())
}

#[cfg(feature="nonblocking")]
fn main() {}