#[cfg(feature="blocking")]
mod tests {
    use sibyl::*;

    #[test]
    fn num_values() -> Result<()> {
        let session = sibyl::test_env::get_session()?;

        let stmt = session.prepare("SELECT Nvl(:val,42) FROM dual")?;
        stmt.set_prefetch_rows(1)?;

        let arg : Option<i32> = None;
        let row = stmt.query_single(arg)?.unwrap();
        let val : i32 = row.get(0)?;
        assert_eq!(val, 42);

        let arg : Option<i32> = None;
        let row = stmt.query_single(&arg)?.unwrap();
        let val : i32 = row.get(0)?;
        assert_eq!(val, 42);

        let arg : Option<&i32> = None;
        let row = stmt.query_single(arg)?.unwrap();
        let val : i32 = row.get(0)?;
        assert_eq!(val, 42);

        let arg : Option<&i32> = None;
        let row = stmt.query_single(&arg)?.unwrap();
        let val : i32 = row.get(0)?;
        assert_eq!(val, 42);

        let arg = Some(99);
        let row = stmt.query_single(arg)?.unwrap();
        let val : i32 = row.get(0)?;
        assert_eq!(val, 99);

        let arg = Some(99);
        let row = stmt.query_single(&arg)?.unwrap();
        let val : i32 = row.get(0)?;
        assert_eq!(val, 99);

        let num = 99;
        let arg = Some(&num);
        let row = stmt.query_single(arg)?.unwrap();
        let val : i32 = row.get(0)?;
        assert_eq!(val, 99);

        let arg = Some(&num);
        let row = stmt.query_single(&arg)?.unwrap();
        let val : i32 = row.get(0)?;
        assert_eq!(val, 99);

        let stmt = session.prepare("
        BEGIN
            :VAL := Nvl(:VAL, 0) + 1;
        END;
        ")?;
        let mut val : Option<i32> = None;
        let count = stmt.execute(&mut val)?;
        assert_eq!(count, 1);
        assert_eq!(val, Some(1));

        let mut val = Some(99);
        let count = stmt.execute(&mut val)?;
        assert_eq!(count, 1);
        assert_eq!(val, Some(100));

        let val = 99;
        let mut arg = Some(&val);
        let count = stmt.execute(&mut arg)?;
        assert_eq!(count, 1);

        #[cfg(not(feature="unsafe-direct-binds"))]
        assert_eq!(val, 99);
        #[cfg(feature="unsafe-direct-binds")]
        // val's memory is bound to :VAL and thus OCI reads from it
        // directly, but it also writes back into it as :VAL is also an OUT.
        // The user must either not make these mistakes - binding read-only
        // variable to an OUT parameter - or use default "safe binds", where
        // the value is first copied into a buffer.
        // BTW, if the val was also in a read-only section, like literal str
        // for example, then during `execute` program would fail with SIGSEGV,
        // when OCI would try to write into the bound memory.
        assert_eq!(val, 100);

        let mut val = 99;
        let mut arg = Some(&mut val);
        let count = stmt.execute(&mut arg)?;
        assert_eq!(count, 1);
        assert_eq!(val, 100);

        let stmt = session.prepare("
        BEGIN
            :VAL := NULL;
        END;
        ")?;
        let mut val = Some(99);
        let count = stmt.execute(&mut val)?;
        assert_eq!(count, 1);
        assert!(val.is_none());

        Ok(())
    }

    #[test]
    // Unlike Option that owns the value, where new value can be inserted,
    // Option-al ref cannot be changed, thus we always get ORA-06502 - buffer too small -
    // as the "buffer" here has literal length of 0 bytes.
    fn output_to_none() -> Result<()> {
        let session = sibyl::test_env::get_session()?;

        let stmt = session.prepare("
        BEGIN
            :VAL := Nvl(:VAL, 0) + 1;
        END;
        ")?;
        let mut val : Option<&i32> = None;
        let res = stmt.execute(&mut val);
        match res {
            Err(Error::Oracle(code, _)) => {
                assert_eq!(code, 6502);
            },
            _ => {
                panic!("unexpected result");
            }
        }

        let mut val : Option<&mut i32> = None;
        let res = stmt.execute(&mut val);
        match res {
            Err(Error::Oracle(code, _)) => {
                assert_eq!(code, 6502);
            },
            _ => {
                panic!("unexpected result");
            }
        }

        let stmt = session.prepare("
        BEGIN
            :VAL := 'area 51';
        END;
        ")?;
        let mut val : Option<&str> = None;
        let res = stmt.execute(&mut val);
        match res {
            Err(Error::Oracle(code, _)) => {
                assert_eq!(code, 6502);
            },
            _ => {
                panic!("unexpected result");
            }
        }

        Ok(())
    }

    #[test]
    fn str_slices() -> Result<()> {
        let session = sibyl::test_env::get_session()?;

        let stmt = session.prepare("SELECT Nvl(:val,'None') FROM dual")?;
        stmt.set_prefetch_rows(1)?;

        let arg : Option<&str> = None;
        let row = stmt.query_single(arg)?.unwrap();
        let val : &str = row.get(0)?;
        assert_eq!(val, "None");

        let arg : Option<&str> = None;
        let row = stmt.query_single(&arg)?.unwrap();
        let val : &str = row.get(0)?;
        assert_eq!(val, "None");

        let arg : Option<&&str> = None;
        let row = stmt.query_single(arg)?.unwrap();
        let val : &str = row.get(0)?;
        assert_eq!(val, "None");

        let arg : Option<&&str> = None;
        let row = stmt.query_single(&arg)?.unwrap();
        let val : &str = row.get(0)?;
        assert_eq!(val, "None");

        let arg = Some("Text");
        let row = stmt.query_single(arg)?.unwrap();
        let val : &str = row.get(0)?;
        assert_eq!(val, "Text");

        let arg = Some("Text");
        let row = stmt.query_single(&arg)?.unwrap();
        let val : &str = row.get(0)?;
        assert_eq!(val, "Text");

        let txt = "Text";
        let arg = Some(&txt);
        let row = stmt.query_single(arg)?.unwrap();
        let val : &str = row.get(0)?;
        assert_eq!(val, "Text");

        let arg = Some(&txt);
        let row = stmt.query_single(&arg)?.unwrap();
        let val : &str = row.get(0)?;
        assert_eq!(val, "Text");

        let arg = Some("");
        let row = stmt.query_single(&arg)?.unwrap();
        let val : &str = row.get(0)?;
        assert_eq!(val, "None");

        let stmt = session.prepare("
        BEGIN
            :VAL := 'area 51';
        END;
        ")?;
        // Start with a String becuase we need str in a writable section for unsafe-direct-binds
        // variant of this test. If we use literal str, it'll be placed into .rodata, and during
        // stmt.execute the app will get SIGSEGV.
        let txt = String::from("unknown");
        let txt = txt.as_str();
        let val = Some(&txt);
        let cnt = stmt.execute(val)?;
        assert_eq!(cnt, 1);

        #[cfg(not(feature="unsafe-direct-binds"))]
        assert_eq!(txt, "unknown");
        #[cfg(feature="unsafe-direct-binds")]
        assert_eq!(txt, "area 51");

        let stmt = session.prepare("
        BEGIN
            :VAL := NULL;
        END;
        ")?;
        let mut val = Some("text");
        let cnt = stmt.execute(&mut val)?;
        assert_eq!(cnt, 1);
        assert!(val.is_none());

        Ok(())
    }

    #[test]
    fn strings() -> Result<()> {
        let session = sibyl::test_env::get_session()?;

        let stmt = session.prepare("SELECT Nvl(:val,'None') FROM dual")?;
        stmt.set_prefetch_rows(1)?;

        let arg : Option<String> = None;
        let row = stmt.query_single(arg)?.unwrap();
        let val : &str = row.get(0)?;
        assert_eq!(val, "None");

        let arg : Option<String> = None;
        let row = stmt.query_single(&arg)?.unwrap();
        let val : &str = row.get(0)?;
        assert_eq!(val, "None");

        let arg : Option<&String> = None;
        let row = stmt.query_single(arg)?.unwrap();
        let val : &str = row.get(0)?;
        assert_eq!(val, "None");

        let arg : Option<&String> = None;
        let row = stmt.query_single(&arg)?.unwrap();
        let val : &str = row.get(0)?;
        assert_eq!(val, "None");

        let arg = Some(String::from("Text"));
        let row = stmt.query_single(arg)?.unwrap();
        let val : &str = row.get(0)?;
        assert_eq!(val, "Text");

        let arg = Some(String::from("Text"));
        let row = stmt.query_single(&arg)?.unwrap();
        let val : &str = row.get(0)?;
        assert_eq!(val, "Text");

        let txt = String::from("Text");
        let arg = Some(&txt);
        let row = stmt.query_single(arg)?.unwrap();
        let val : &str = row.get(0)?;
        assert_eq!(val, "Text");

        let arg = Some(&txt);
        let row = stmt.query_single(&arg)?.unwrap();
        let val : &str = row.get(0)?;
        assert_eq!(val, "Text");

        let arg = Some(String::new());
        let row = stmt.query_single(&arg)?.unwrap();
        let val : &str = row.get(0)?;
        assert_eq!(val, "None");

        let stmt = session.prepare("
        BEGIN
            IF :VAL IS NULL THEN
                :VAL := 'Area 51';
            ELSE
                :VAL := '<<' || :VAL || '>>';
            END IF;
        END;
        ")?;
        // NULL IN, VARCHAR OUT
        let mut val = Some(String::with_capacity(16));
        let cnt = stmt.execute(&mut val)?;
        assert_eq!(cnt, 1);
        assert_eq!(val, Some(String::from("Area 51")));

        // VARCHAR IN, VARCHAR OUT
        let cnt = stmt.execute(&mut val)?;
        assert_eq!(cnt, 1);
        assert_eq!(val, Some(String::from("<<Area 51>>")));

        let stmt = session.prepare("
        BEGIN
            :VAL := NULL;
        END;
        ")?;
        // VARCHAR IN, NULL OUT
        let mut val = Some(String::from("text"));
        let cnt = stmt.execute(&mut val)?;
        assert_eq!(cnt, 1);
        assert!(val.is_none());

        Ok(())
    }

    #[test]
    fn bin_slices() -> Result<()> {
        let session = sibyl::test_env::get_session()?;

        let stmt = session.prepare("SELECT Nvl(:VAL,Utl_Raw.Cast_To_Raw('nil')) FROM dual")?;

        let arg : Option<&[u8]> = None;
        let row = stmt.query_single(&arg)?.unwrap();
        let val : &[u8] = row.get(0)?;
        assert_eq!(val, &[0x6e, 0x69, 0x6c]);

        let row = stmt.query_single(arg)?.unwrap();
        let val : &[u8] = row.get(0)?;
        assert_eq!(val, &[0x6e, 0x69, 0x6c]);

        let val = [0x62, 0x69, 0x6e].as_ref();
        let arg = Some(val);
        let row = stmt.query_single(&arg)?.unwrap();
        let res : &[u8] = row.get(0)?;
        assert_eq!(res, val);

        let row = stmt.query_single(arg)?.unwrap();
        let res : &[u8] = row.get(0)?;
        assert_eq!(res, val);

        let val = [].as_ref();
        let arg = Some(val);
        let row = stmt.query_single(&arg)?.unwrap();
        let res : &[u8] = row.get(0)?;
        assert_eq!(res, &[0x6e, 0x69, 0x6c]);

        let stmt = session.prepare("
        BEGIN
            :VAL := Utl_Raw.Cast_To_Raw('Area 51');
        END;
        ")?;
        let mut bin = [0;10];
        let mut bin = bin.as_mut();
        let mut val = Some(&mut bin);
        let cnt = stmt.execute(&mut val)?;
        assert_eq!(cnt, 1);
        assert_eq!(bin, [0x41, 0x72, 0x65, 0x61, 0x20, 0x35, 0x31, 0x00, 0x00, 0x00].as_ref());
        // ---- note how as-is it is not very useful as an OUT ----^^^^--^^^^--^^^^
        assert_eq!(stmt.len_of(0)?, 7);
        let res = bin[0..stmt.len_of(0)?].as_ref();
        assert_eq!(res, [0x41, 0x72, 0x65, 0x61, 0x20, 0x35, 0x31].as_ref());

        // However, it is adequate for the "data IN, NULL OUT" use case:
        let stmt = session.prepare("
        BEGIN
            :VAL := NULL;
        END;
        ")?;
        let mut bin = [0x41, 0x72, 0x65, 0x61, 0x20, 0x35, 0x31];
        let mut bin = bin.as_mut();
        let mut val = Some(&mut bin);
        let cnt = stmt.execute(&mut val)?;
        assert_eq!(cnt, 1);
        assert!(val.is_none());

        Ok(())
    }

    #[test]
    fn bin_vec() -> Result<()> {
        let session = sibyl::test_env::get_session()?;

        let stmt = session.prepare("SELECT Nvl(:VAL,Utl_Raw.Cast_To_Raw('nil')) FROM dual")?;

        let arg : Option<Vec<u8>> = None;
        let row = stmt.query_single(&arg)?.unwrap();
        let val : &[u8] = row.get(0)?;
        assert_eq!(val, &[0x6e, 0x69, 0x6c]);

        let row = stmt.query_single(arg)?.unwrap();
        let val : &[u8] = row.get(0)?;
        assert_eq!(val, &[0x6e, 0x69, 0x6c]);

        let val = [0x62, 0x69, 0x6e].to_vec();
        let arg = Some(val);
        let row = stmt.query_single(&arg)?.unwrap();
        let res : &[u8] = row.get(0)?;
        assert_eq!(res, &[0x62, 0x69, 0x6e]);

        let row = stmt.query_single(arg)?.unwrap();
        let res : &[u8] = row.get(0)?;
        assert_eq!(res, &[0x62, 0x69, 0x6e]);

        let arg = Some(Vec::new());
        let row = stmt.query_single(&arg)?.unwrap();
        let val : &[u8] = row.get(0)?;
        assert_eq!(val, &[0x6e, 0x69, 0x6c]);

        let stmt = session.prepare("
        BEGIN
            IF :VAL IS NULL THEN
                :VAL := Utl_Raw.Cast_To_Raw('Area 51');
            ELSE
                :VAL := Utl_Raw.Concat(
                    Utl_Raw.Cast_To_Raw('<'),
                    :VAL,
                    Utl_Raw.Cast_To_Raw('>')
                );
            END IF;
        END;
        ")?;
        // NULL IN, RAW OUT
        let mut bin = Vec::with_capacity(16);
        let mut val = Some(&mut bin);
        let cnt = stmt.execute(&mut val)?;
        assert_eq!(cnt, 1);
        // Unlike &[u8] above Vec is updated to reflect the returned data.
        assert!(val.is_some());
        let bin = val.unwrap();
        assert_eq!(bin, &[0x41, 0x72, 0x65, 0x61, 0x20, 0x35, 0x31]);

        // RAW IN, RAW OUT
        let mut val = Some(bin);
        let cnt = stmt.execute(&mut val)?;
        assert!(cnt > 0);
        assert!(val.is_some());
        let bin = val.unwrap();
        assert_eq!(bin, &[0x3c, 0x41, 0x72, 0x65, 0x61, 0x20, 0x35, 0x31, 0x3e]);

        let stmt = session.prepare("
        BEGIN
            :VAL := NULL;
        END;
        ")?;
        // RAW IN, NULL OUT
        let mut val = Some([0x41, 0x72, 0x65, 0x61, 0x20, 0x35, 0x31].to_vec());
        let cnt = stmt.execute(&mut val)?;
        assert_eq!(cnt, 1);
        assert!(val.is_none());

        Ok(())
    }

    #[test]
    fn dates() -> Result<()> {
        let session = sibyl::test_env::get_session()?;

        // NULL IN, DATE OUT... kind of :-)
        let stmt = session.prepare("SELECT Nvl(:VAL,To_Date('1969-07-16 13:32:00','YYYY-MM-DD HH24:MI:SS')) FROM dual")?;
        let arg : Option<Date> = None;
        let row = stmt.query_single(arg)?.unwrap();
        let res: Date = row.get(0)?;
        let expected_date = Date::from_string("1969-07-16 13:32:00", "YYYY-MM-DD HH24:MI:SS", &session)?;
        assert_eq!(res.compare(&expected_date)?, std::cmp::Ordering::Equal);

        let stmt = session.prepare("
        BEGIN
            IF :VAL IS NULL THEN
                :VAL := To_Date('1969-07-16 13:32:00','YYYY-MM-DD HH24:MI:SS');
            ELSE
                :VAL := Last_Day(:VAL);
            END IF;
        END;
        ")?;
        // NULL IN, DATE OUT
        let mut val = Some(Date::new(&session));
        let cnt = stmt.execute(&mut val)?;
        assert_eq!(cnt, 1);
        assert!(val.is_some());
        let expected_date = Date::from_string("1969-07-16 13:32:00", "YYYY-MM-DD HH24:MI:SS", &session)?;
        assert_eq!(val.unwrap().compare(&expected_date)?, std::cmp::Ordering::Equal);

        // DATE IN, DATE OUT
        let mut val = Some(Date::from_string("1969-07-16 13:32:00", "YYYY-MM-DD HH24:MI:SS", &session)?);
        let cnt = stmt.execute(&mut val)?;
        assert_eq!(cnt, 1);
        let expected_date = Date::from_string("1969-07-31 13:32:00", "YYYY-MM-DD HH24:MI:SS", &session)?;
        assert_eq!(val.unwrap().compare(&expected_date)?, std::cmp::Ordering::Equal);

        // DATE IN, NULL OUT
        let stmt = session.prepare("
        BEGIN
            :VAL := NULL;
        END;
        ")?;
        let mut val = Some(Date::from_string("1969-07-16 13:32:00", "YYYY-MM-DD HH24:MI:SS", &session)?);
        let cnt = stmt.execute(&mut val)?;
        assert_eq!(cnt, 1);
        assert!(val.is_none());

        Ok(())
    }

    #[test]
    fn intervals() -> Result<()> {
        let session = sibyl::test_env::get_session()?;

        // NULL IN, INTERVAL OUT... kind of :-)
        let stmt = session.prepare("
            SELECT Nvl(:VAL, To_Timestamp('1969-07-24 16:50:35','YYYY-MM-DD HH24:MI:SS') - To_Timestamp('1969-07-16 13:32:00','YYYY-MM-DD HH24:MI:SS'))
              FROM dual
        ")?;
        let arg : Option<IntervalDS> = None;
        let row = stmt.query_single(arg)?.unwrap();
        let res: IntervalDS = row.get(0)?;
        let expected_interval = IntervalDS::from_string("+8 03:18:35.00", &session)?;
        assert_eq!(res.compare(&expected_interval)?, std::cmp::Ordering::Equal);

        // INTERVAL IN, INTERVAL OUT
        let stmt = session.prepare("
        BEGIN
            :VAL := To_Timestamp('1969-07-20','YYYY-MM-DD') + :VAL - To_Timestamp('1969-07-16 13:32:00','YYYY-MM-DD HH24:MI:SS');
        END;
        ")?;
        let mut val = Some(IntervalDS::from_string("+4 16:50:35.00", &session)?);
        let cnt = stmt.execute(&mut val)?;
        assert_eq!(cnt, 1);
        let expected_interval = IntervalDS::from_string("+8 03:18:35.00", &session)?;
        assert_eq!(val.unwrap().compare(&expected_interval)?, std::cmp::Ordering::Equal);

        // INTERVAL IN, NULL OUT
        let stmt = session.prepare("
        BEGIN
            :VAL := NULL;
        END;
        ")?;
        let mut val = Some(IntervalDS::from_string("+4 16:50:35.00", &session)?);
        let cnt = stmt.execute(&mut val)?;
        assert_eq!(cnt, 1);
        assert!(val.is_none());

        Ok(())
    }

    #[test]
    fn timestamps() -> Result<()> {
        let session = sibyl::test_env::get_session()?;

        // NULL IN, TIMESTAMP OUT... kind of :-)
        let stmt = session.prepare("
            SELECT Nvl(:VAL, To_Timestamp('1969-07-24 16:50:35','YYYY-MM-DD HH24:MI:SS'))
              FROM dual
        ")?;
        let arg : Option<Timestamp> = None;
        let row = stmt.query_single(arg)?.unwrap();
        let res: Timestamp = row.get(0)?;
        let expected_timestamp = Timestamp::from_string("1969-07-24 16:50:35", "YYYY-MM-DD HH24:MI:SS", &session)?;
        assert_eq!(res.compare(&expected_timestamp)?, std::cmp::Ordering::Equal);

        let stmt = session.prepare("
        BEGIN
            IF :VAL IS NULL THEN
                :VAL := To_Timestamp('1969-07-16 13:32:00','YYYY-MM-DD HH24:MI:SS');
            ELSE
                :VAL := :VAL + To_DSInterval('+8 03:18:35.00');
            END IF;
        END;
        ")?;
        // NULL IN, TIMESTAMP OUT
        let mut val = Some(Timestamp::new(&session)?);
        let cnt = stmt.execute(&mut val)?;
        assert_eq!(cnt, 1);
        assert!(val.is_some());
        let res = val.unwrap();
        let expected_timestamp = Timestamp::from_string("1969-07-16 13:32:00", "YYYY-MM-DD HH24:MI:SS", &session)?;
        assert_eq!(res.compare(&expected_timestamp)?, std::cmp::Ordering::Equal);

        // TIMESTAMP IN, TIMESTAMP OUT
        let mut val = Some(res);
        let cnt = stmt.execute(&mut val)?;
        assert!(cnt > 0);
        assert!(val.is_some());
        let res = val.unwrap();
        let expected_timestamp = Timestamp::from_string("1969-07-24 16:50:35", "YYYY-MM-DD HH24:MI:SS", &session)?;
        assert_eq!(res.compare(&expected_timestamp)?, std::cmp::Ordering::Equal);

        // TIMESTAMP IN, NULL OUT
        let stmt = session.prepare("
        BEGIN
            :VAL := NULL;
        END;
        ")?;
        let mut val = Some(res);
        let cnt = stmt.execute(&mut val)?;
        assert_eq!(cnt, 1);
        assert!(val.is_none());

        Ok(())
    }

    #[test]
    fn numbers() -> Result<()> {
        let session = sibyl::test_env::get_session()?;

        // NULL IN, NUMBER OUT (kind of)
        let stmt = session.prepare("
            SELECT Nvl(:VAL, 42) FROM dual
        ")?;
        let arg : Option<Number> = None;
        let row = stmt.query_single(arg)?.unwrap();
        let res: Number = row.get(0)?;
        let expected_number = Number::from_int(42, &session)?;
        assert_eq!(res.compare(&expected_number)?, std::cmp::Ordering::Equal);

        let stmt = session.prepare("
        BEGIN
            IF :VAL IS NULL THEN
                :VAL := 99;
            ELSE
                :VAL := :VAL + 1;
            END IF;
        END;
        ")?;
        // NULL IN, NUMBER OUT
        let mut val = Some(Number::new(&session));
        let cnt = stmt.execute(&mut val)?;
        assert_eq!(cnt, 1);
        assert!(val.is_some());
        let expected_number = Number::from_int(99, &session)?;
        assert_eq!(val.unwrap().compare(&expected_number)?, std::cmp::Ordering::Equal);

        // NUMBER IN, NUMBER OUT
        let mut val = Some(expected_number);
        let cnt = stmt.execute(&mut val)?;
        assert_eq!(cnt, 1);
        assert!(val.is_some());
        let expected_number = Number::from_int(100, &session)?;
        assert_eq!(val.unwrap().compare(&expected_number)?, std::cmp::Ordering::Equal);

        // NUMBER IN, NUMBER OUT
        let stmt = session.prepare("
        BEGIN
            :VAL := NULL;
        END;
        ")?;
        let mut val = Some(Number::from_int(99, &session)?);
        let cnt = stmt.execute(&mut val)?;
        assert_eq!(cnt, 1);
        assert!(val.is_none());

        Ok(())
    }

    #[test]
    fn varchars() -> Result<()> {
        let session = sibyl::test_env::get_session()?;

        // NULL IN, VARCHAR OUT (kind of)
        let stmt = session.prepare("
            SELECT Nvl(:VAL, 'hello') FROM dual
        ")?;
        let arg : Option<Varchar> = None;
        let row = stmt.query_single(arg)?.unwrap();
        let res: Varchar = row.get(0)?;
        assert_eq!(res.as_str(), "hello");

        let stmt = session.prepare("
        BEGIN
            IF :VAL IS NULL THEN
                :VAL := 'text';
            ELSE
                :VAL := '<' || :VAL || '>';
            END IF;
        END;
        ")?;
        // NULL IN, VARCHAR OUT
        let mut val = Some(Varchar::with_capacity(8, &session)?);
        let cnt = stmt.execute(&mut val)?;
        assert_eq!(cnt, 1);
        assert!(val.is_some());
        let res = val.unwrap();
        assert_eq!(res.as_str(), "text");

        // VARCHAR IN, VARCHAR OUT
        let mut val = Some(res);
        let cnt = stmt.execute(&mut val)?;
        assert_eq!(cnt, 1);
        assert!(val.is_some());
        let res = val.unwrap();
        assert_eq!(res.as_str(), "<text>");

        // VARCHAR IN, NULL OUT
        let stmt = session.prepare("
        BEGIN
            :VAL := NULL;
        END;
        ")?;
        let mut val = Some(Varchar::from("text", &session)?);
        let cnt = stmt.execute(&mut val)?;
        assert_eq!(cnt, 1);
        assert!(val.is_none());

        Ok(())
    }

    #[test]
    fn raws() -> Result<()> {
        let session = sibyl::test_env::get_session()?;

        // NULL IN, RAW (kind of)
        let stmt = session.prepare("SELECT Nvl(:VAL,Utl_Raw.Cast_To_Raw('nil')) FROM dual")?;

        let arg : Option<Raw> = None;
        let row = stmt.query_single(arg)?.unwrap();
        let val : Raw = row.get(0)?;
        assert_eq!(val.as_bytes(), &[0x6e, 0x69, 0x6c]);

        let stmt = session.prepare("
        BEGIN
            IF :VAL IS NULL THEN
                :VAL := Utl_Raw.Cast_To_Raw('data');
            ELSE
                :VAL := Utl_Raw.Concat(
                    Utl_Raw.Cast_To_Raw('<'),
                    :VAL,
                    Utl_Raw.Cast_To_Raw('>')
                );
            END IF;
        END;
        ")?;
        // NULL IN, RAW OUT
        let mut val = Some(Raw::with_capacity(8, &session)?);
        let cnt = stmt.execute(&mut val)?;
        assert_eq!(cnt, 1);
        assert!(val.is_some());
        let bin = val.unwrap();
        assert_eq!(bin.as_bytes(), &[0x64, 0x61, 0x74, 0x61]);

        // RAW IN, RAW OUT
        let mut val = Some(bin);
        let cnt = stmt.execute(&mut val)?;
        assert_eq!(cnt, 1);
        assert!(val.is_some());
        assert_eq!(val.unwrap().as_bytes(), &[0x3c, 0x64, 0x61, 0x74, 0x61, 0x3e]);

        // RAW IN, NULL OUT
        let stmt = session.prepare("
        BEGIN
            :VAL := NULL;
        END;
        ")?;
        let mut val = Some(Raw::from_bytes(&[0x64, 0x61, 0x74, 0x61], &session)?);
        let cnt = stmt.execute(&mut val)?;
        assert_eq!(cnt, 1);
        assert!(val.is_none());

        Ok(())
    }

    #[test]
    fn rawids() -> Result<()> {
        let session = sibyl::test_env::get_session()?;

        let stmt = session.prepare("
        BEGIN
            IF :VAL IS NULL THEN
                SELECT rowid 
                  INTO :VAL 
                  FROM system.help 
                 WHERE topic='@' 
                   AND seq = 2;
            ELSE
                SELECT rowid 
                  INTO :VAL 
                  FROM system.help 
                 WHERE (topic, seq) IN (
                           SELECT topic, seq + 1
                             FROM system.help
                            WHERE rowid = :VAL );
            END IF;
        END;
        ")?;
        // NULL IN, RAWID OUT
        let mut val = Some(RowID::new(&session)?);
        let cnt = stmt.execute(&mut val)?;
        assert_eq!(cnt, 1);
        assert!(val.is_some());

        let assert_stmt = session.prepare("SELECT info FROM system.help WHERE rowid = :ROW_ID")?;
        let row = assert_stmt.query_single(&val)?.unwrap();
        let info : &str = row.get(0)?;
        assert_eq!(info, r#" @ ("at" sign)"#);

        // RAWID IN, RAWID OUT
        let cnt = stmt.execute(&mut val)?;
        assert_eq!(cnt, 2);
        assert!(val.is_some());

        let row = assert_stmt.query_single(&val)?.unwrap();
        let info : &str = row.get(0)?;
        assert_eq!(info, r#" -------------"#);

        // RAWID IN, NULL OUT
        let stmt = session.prepare("
        BEGIN
            :VAL := NULL;
        END;
        ")?;
        let cnt = stmt.execute(&mut val)?;
        assert_eq!(cnt, 1);
        assert!(val.is_none());

        Ok(())
    }
}
