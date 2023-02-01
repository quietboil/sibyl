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
            :VAL := Utl_Raw.Cast_To_Raw('Hello, World!');
        END IF;
    END;
    ")?;
    let mut buf = [0; 16];
    let mut val = Nvl::new(buf.as_mut_slice());

    stmt.execute(&mut val)?;
    assert!(!stmt.is_null("VAL")?);
    assert!(val.as_ref().is_some());
    assert_eq!(
        val.as_ref().unwrap(),
        &[0x48u8, 0x65, 0x6C, 0x6C, 0x6F, 0x2C, 0x20, 0x57, 0x6F, 0x72, 0x6C, 0x64, 0x21, 0x00, 0x00, 0x00]
        // note the "trailing" initial zeroes are not overwritten by output --------------^^^^--^^^^--^^^^
    );
    let output_len = stmt.len_of("VAL")?;
    let output = &val.as_ref().unwrap()[0..output_len];
    assert_eq!(output, &[0x48u8, 0x65, 0x6C, 0x6C, 0x6F, 0x2C, 0x20, 0x57, 0x6F, 0x72, 0x6C, 0x64, 0x21]);

    Ok(())
}

#[cfg(feature="nonblocking")]
fn main() {}