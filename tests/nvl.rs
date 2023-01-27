#[cfg(feature="blocking")]
mod tests {
    use once_cell::sync::OnceCell;
    use sibyl::*;

    static ORACLE : OnceCell<Environment> = OnceCell::new();
    static POOL : OnceCell<SessionPool> = OnceCell::new();

    fn get_session() -> Result<Session<'static>> {
        let dbname = std::env::var("DBNAME").expect("database name");
        let dbuser = std::env::var("DBUSER").expect("user name");
        let dbpass = std::env::var("DBPASS").expect("password");

        let oracle = ORACLE.get_or_try_init(|| {
            Environment::new()
        })?;
        let pool = POOL.get_or_try_init(|| {
            oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10)
        })?;
        pool.get_session()
    }

    #[test]
    fn primitive_numbers() -> Result<()> {
        let session = get_session()?;

        let stmt = session.prepare("
        BEGIN
            IF :GET_VAL AND :VAL IS NULL THEN
                :VAL := Nvl(:VAL, 0) + 1;
            ELSIF :VAL IS NULL THEN
                :VAL := NULL;
            ELSE
                :VAL := 99;
            END IF;
        END;
        ")?;

        let mut val = Nvl::new(42);
        let cnt = stmt.execute((true, &mut val, ()))?;
        assert!(cnt > 0);
        assert_eq!(val.as_ref(), Some(&1));

        let mut val = Nvl::new(42);
        let cnt = stmt.execute((false, &mut val, ()))?;
        assert!(cnt > 0);
        assert_eq!(val.as_ref(), None);

        Ok(())
    }

    #[test]
    fn strings() -> Result<()> {
        let session = get_session()?;

        let stmt = session.prepare("
        BEGIN
            IF :GET_VAL AND :VAL IS NULL THEN
                :VAL := 'Hello, World!';
            ELSE
                :VAL := NULL;
            END IF;
        END;
        ")?;

        let mut val = Nvl::new(String::from("xxxxxxxxxxxxxxxx"));
        let cnt = stmt.execute((true, &mut val, ()))?;
        assert!(cnt > 0);
        assert_eq!(val.as_ref(), Some(&String::from("Hello, World!")));

        let cnt = stmt.execute((false, &mut val, ()))?;
        assert!(cnt > 0);
        assert_eq!(val.as_ref(), None);

        // Nvl is not really necessary for Strings as an empty
        // String is seen as NULL by Oracle

        let mut val = String::with_capacity(16);
        let cnt = stmt.execute((true, &mut val, ()))?;
        assert!(cnt > 0);
        assert!(!stmt.is_null(1)?);
        assert_eq!(val, "Hello, World!");

        let cnt = stmt.execute((false, &mut val, ()))?;
        assert!(cnt > 0);
        assert!(stmt.is_null(1)?);
        assert!(val.is_empty());

        Ok(())
    }

    #[test]
    fn bytes() -> Result<()> {
        let session = get_session()?;

        let stmt = session.prepare("
        BEGIN
            IF :GET_VAL AND :VAL IS NULL THEN
                :VAL := Utl_Raw.Cast_To_Raw('Hello, World!');
            ELSE
                :VAL := NULL;
            END IF;
        END;
        ")?;
        let mut val = [0; 16];
        let mut val = Nvl::new(val.as_mut_slice());
        let cnt = stmt.execute((true, &mut val, ()))?;
        assert!(cnt > 0);
        assert_eq!(val.as_ref(), Some(&[0x48u8, 0x65, 0x6C, 0x6C, 0x6F, 0x2C, 0x20, 0x57, 0x6F, 0x72, 0x6C, 0x64, 0x21, 0x00, 0x00, 0x00].as_mut()));

        let cnt = stmt.execute((false, &mut val, ()))?;
        assert!(cnt > 0);
        assert_eq!(val.as_ref(), None);

        Ok(())
    }

    #[test]
    fn byte_vec() -> Result<()> {
        let session = get_session()?;

        let stmt = session.prepare("
        BEGIN
            IF :GET_VAL AND :VAL IS NULL THEN
                :VAL := Utl_Raw.Cast_To_Raw('Hello, World!');
            ELSE
                :VAL := NULL;
            END IF;
        END;
        ")?;
        let val = [0; 16].to_vec();
        let mut val = Nvl::new(val);
        let cnt = stmt.execute((true, &mut val, ()))?;
        assert!(cnt > 0);
        assert_eq!(val.as_ref(), Some(&[0x48u8, 0x65, 0x6C, 0x6C, 0x6F, 0x2C, 0x20, 0x57, 0x6F, 0x72, 0x6C, 0x64, 0x21].to_vec()));

        let cnt = stmt.execute((false, &mut val, ()))?;
        assert!(cnt > 0);
        assert_eq!(val.as_ref(), None);

        // Nvl is not really necessary for Vec as an empty
        // Vec is seen as NULL by Oracle

        let mut val = Vec::with_capacity(16);
        let cnt = stmt.execute((true, &mut val, ()))?;
        assert!(cnt > 0);
        assert_eq!(val.as_slice(), &[0x48u8, 0x65, 0x6C, 0x6C, 0x6F, 0x2C, 0x20, 0x57, 0x6F, 0x72, 0x6C, 0x64, 0x21]);

        let cnt = stmt.execute((false, &mut val, ()))?;
        assert!(cnt > 0);
        assert!(val.is_empty());
        assert!(stmt.is_null(1)?);

        Ok(())
    }

    #[test]
    fn odt_date() -> Result<()> {
        let session = get_session()?;

        let stmt = session.prepare("
        BEGIN
            IF :GET_VAL AND :VAL IS NULL THEN
                :VAL := To_Date('1969-07-16 13:32:00','YYYY-MM-DD HH24:MI:SS');
            ELSE
                :VAL := NULL;
            END IF;
        END;
        ")?;
        let val = Date::new(&session);
        let mut val = Nvl::new(val);
        let cnt = stmt.execute((true, &mut val, ()))?;
        assert!(cnt > 0);
        let expected_date = Date::from_string("1969-07-16 13:32:00", "YYYY-MM-DD HH24:MI:SS", &session)?;
        assert_eq!(val.as_ref().unwrap().compare(&expected_date)?, std::cmp::Ordering::Equal);

        let cnt = stmt.execute((false, &mut val, ()))?;
        assert!(cnt > 0);
        assert!(val.as_ref().is_none());

        // Date can be used without Nvl. `Date::new()` creates dates that are seen as NULL
        // by the argument binding code.

        let mut val = Date::new(&session);
        let cnt = stmt.execute((true, &mut val, ()))?;
        assert!(cnt > 0);
        let expected_date = Date::from_string("1969-07-16 13:32:00", "YYYY-MM-DD HH24:MI:SS", &session)?;
        assert_eq!(val.compare(&expected_date)?, std::cmp::Ordering::Equal);

        let cnt = stmt.execute((false, &mut val, ()))?;
        assert!(cnt > 0);
        assert!(stmt.is_null("VAL")?);

        Ok(())
    }

    #[test]
    fn odt_interval() -> Result<()> {
        let session = get_session()?;

        let stmt = session.prepare("
        BEGIN
            IF :GET_VAL AND :VAL IS NULL THEN
                :VAL := To_Timestamp('1969-07-24 16:50:35','YYYY-MM-DD HH24:MI:SS') - To_Timestamp('1969-07-16 13:32:00','YYYY-MM-DD HH24:MI:SS');
            ELSE
                :VAL := NULL;
            END IF;
        END;
        ")?;
        let val = IntervalDS::new(&session)?;
        let mut val = Nvl::new(val);
        let cnt = stmt.execute((true, &mut val, ()))?;
        assert!(cnt > 0);
        let expected_interval = IntervalDS::from_string("+8 03:18:35.00", &session)?;
        assert_eq!(val.as_ref().unwrap().compare(&expected_interval)?, std::cmp::Ordering::Equal);

        let cnt = stmt.execute((false, &mut val, ()))?;
        assert!(cnt > 0);
        assert!(val.as_ref().is_none());

        Ok(())
    }

    #[test]
    fn odt_timestamp() -> Result<()> {
        let session = get_session()?;

        let stmt = session.prepare("
        BEGIN
            IF :GET_VAL AND :VAL IS NULL THEN
                :VAL := To_Timestamp('1969-07-24 16:50:35','YYYY-MM-DD HH24:MI:SS');
            ELSE
                :VAL := NULL;
            END IF;
        END;
        ")?;
        let val = Timestamp::new(&session)?;
        let mut val = Nvl::new(val);
        let cnt = stmt.execute((true, &mut val, ()))?;
        assert!(cnt > 0);
        let expected_timestamp = Timestamp::from_string("1969-07-24 16:50:35", "YYYY-MM-DD HH24:MI:SS", &session)?;
        assert_eq!(val.as_ref().unwrap().compare(&expected_timestamp)?, std::cmp::Ordering::Equal);

        let cnt = stmt.execute((false, &mut val, ()))?;
        assert!(cnt > 0);
        assert!(val.as_ref().is_none());

        // Timestamp can be used without Nvl. `Timestamp::new()` creates timestamps that are seen as NULL
        // by the argument binding code.

        let mut val = Timestamp::new(&session)?;
        let cnt = stmt.execute((true, &mut val, ()))?;
        assert!(cnt > 0);
        let expected_timestamp = Timestamp::from_string("1969-07-24 16:50:35", "YYYY-MM-DD HH24:MI:SS", &session)?;
        assert_eq!(val.compare(&expected_timestamp)?, std::cmp::Ordering::Equal);

        let cnt = stmt.execute((false, &mut val, ()))?;
        assert!(cnt > 0);
        assert!(stmt.is_null("VAL")?);

        Ok(())
    }

    #[test]
    fn odt_number() -> Result<()> {
        let session = get_session()?;

        let stmt = session.prepare("
        BEGIN
            IF :GET_VAL AND :VAL IS NULL THEN
                :VAL := 8191;
            ELSE
                :VAL := NULL;
            END IF;
        END;
        ")?;
        let val = Number::new(&session);
        let mut val = Nvl::new(val);
        let cnt = stmt.execute((true, &mut val, ()))?;
        assert!(cnt > 0);
        let expected_number = Number::from_int(8191, &session)?;
        assert_eq!(val.as_ref().unwrap().compare(&expected_number)?, std::cmp::Ordering::Equal);

        let cnt = stmt.execute((false, &mut val, ()))?;
        assert!(cnt > 0);
        assert!(val.as_ref().is_none());

        // Number can be used without Nvl. `Number::new()` creates numbers that are seen as NULL
        // by the argument binding code.

        let mut val = Number::new(&session);
        let cnt = stmt.execute((true, &mut val, ()))?;
        assert!(cnt > 0);
        assert_eq!(val.compare(&expected_number)?, std::cmp::Ordering::Equal);

        let cnt = stmt.execute((false, &mut val, ()))?;
        assert!(cnt > 0);
        assert!(stmt.is_null("VAL")?);

        Ok(())
    }

    #[test]
    fn odt_varchar() -> Result<()> {
        let session = get_session()?;

        let stmt = session.prepare("
        BEGIN
            IF :GET_VAL AND :VAL IS NULL THEN
                :VAL := 'Hello, World!';
            ELSE
                :VAL := NULL;
            END IF;
        END;
        ")?;
        let mut val = Nvl::new(Varchar::with_capacity(16, &session)?);
        let cnt = stmt.execute((true, &mut val, ()))?;
        assert!(cnt > 0);
        assert_eq!(val.as_ref().unwrap().as_str(), "Hello, World!");

        let cnt = stmt.execute((false, &mut val, ()))?;
        assert!(cnt > 0);
        assert!(val.as_ref().is_none());

        // Nvl is not really necessary for Varchars as an empty
        // Varchar is seen as NULL by Oracle

        let mut val = Varchar::with_capacity(16, &session)?;
        let cnt = stmt.execute((true, &mut val, ()))?;
        assert!(cnt > 0);
        assert_eq!(val.as_str(), "Hello, World!");

        let cnt = stmt.execute((false, &mut val, ()))?;
        assert!(cnt > 0);
        assert_eq!(val.len(), 0);
        assert!(stmt.is_null("VAL")?);

        Ok(())
    }

    #[test]
    fn odt_raw() -> Result<()> {
        let session = get_session()?;

        let stmt = session.prepare("
        BEGIN
            IF :GET_VAL AND :VAL IS NULL THEN
                :VAL := Utl_Raw.Cast_To_Raw('Hello, World!');
            ELSE
                :VAL := NULL;
            END IF;
        END;
        ")?;
        let mut val = Nvl::new(Raw::with_capacity(16, &session)?);
        let cnt = stmt.execute((true, &mut val, ()))?;
        assert!(cnt > 0);
        assert_eq!(val.as_ref().unwrap().as_bytes(), [0x48u8, 0x65, 0x6C, 0x6C, 0x6F, 0x2C, 0x20, 0x57, 0x6F, 0x72, 0x6C, 0x64, 0x21]);

        let cnt = stmt.execute((false, &mut val, ()))?;
        assert!(cnt > 0);
        assert!(val.as_ref().is_none());

        // Raw can be used without Nvl, because an empty Raw is seen as NULL
        // by the argument binding code.

        let mut val = Raw::with_capacity(16, &session)?;
        let cnt = stmt.execute((true, &mut val, ()))?;
        assert!(cnt > 0);
        assert_eq!(val.as_bytes(), [0x48u8, 0x65, 0x6C, 0x6C, 0x6F, 0x2C, 0x20, 0x57, 0x6F, 0x72, 0x6C, 0x64, 0x21]);

        let cnt = stmt.execute((false, &mut val, ()))?;
        assert!(cnt > 0);
        assert!(stmt.is_null("VAL")?);
        assert_eq!(val.len(), 0);

        Ok(())
    }

    #[test]
    fn odt_rawid() -> Result<()> {
        let session = get_session()?;

        let stmt = session.prepare("
        BEGIN
            IF :GET_VAL AND :VAL IS NULL THEN
                SELECT rowid 
                  INTO :VAL 
                  FROM system.help 
                 WHERE topic='@' 
                   AND seq = 2;
            ELSE
                :VAL := NULL;
            END IF;
        END;
        ")?;
        let mut val = Nvl::new(RowID::new(&session)?);
        let cnt = stmt.execute((true, &mut val, ()))?;
        assert!(cnt > 0);
        assert!(!stmt.is_null("VAL")?);

        let assert_stmt = session.prepare("SELECT info FROM system.help WHERE rowid = :ROW_ID")?;
        let row = assert_stmt.query_single(val.as_ref())?.unwrap();
        let txt : &str = row.get(0)?;
        assert_eq!(txt, r#" @ ("at" sign)"#);

        let cnt = stmt.execute((false, &mut val, ()))?;
        assert!(cnt > 0);
        assert!(val.as_ref().is_none());

        // RawID can be used without Nvl, because a new/uninitialized RawID
        // is seen as NULL by the argument binding code.

        let mut val = RowID::new(&session)?;
        let cnt = stmt.execute((true, &mut val, ()))?;
        assert!(cnt > 0);
        assert!(!stmt.is_null("VAL")?);

        let row = assert_stmt.query_single(&val)?.unwrap();
        let txt : &str = row.get(0)?;
        assert_eq!(txt, r#" @ ("at" sign)"#);

        let cnt = stmt.execute((false, &mut val, ()))?;
        assert!(cnt > 0);
        assert!(stmt.is_null("VAL")?);

        Ok(())
    }


}
