#[cfg(feature="blocking")]
mod blocking {
    use sibyl::*;

    #[test]
    fn character_datatypes() -> Result<()> {
        let dbname = std::env::var("DBNAME").expect("database name");
        let dbuser = std::env::var("DBUSER").expect("user name");
        let dbpass = std::env::var("DBPASS").expect("password");
        let oracle = env()?;
        let session = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = session.prepare("
            DECLARE
                name_already_used EXCEPTION; PRAGMA EXCEPTION_INIT(name_already_used, -955);
            BEGIN
                EXECUTE IMMEDIATE '
                    CREATE TABLE test_character_data (
                        id      NUMBER GENERATED ALWAYS AS IDENTITY,
                        text    VARCHAR2(97),
                        ntext   NVARCHAR2(99)
                    )
                ';
            EXCEPTION
              WHEN name_already_used THEN NULL;
            END;
        ")?;
        stmt.execute(())?;

        let mut ids = Vec::with_capacity(3);
        let stmt = session.prepare("
            INSERT INTO test_character_data (text, ntext) VALUES (:TEXT, '> ' || :TEXT)
            RETURNING id, text, ntext INTO :ID, :TEXT_OUT, :NTXT_OUT
        ")?;
        let mut id = 0;
        let mut text_out = String::with_capacity(97);
        let mut ntxt_out = String::with_capacity(99);
        let count = stmt.execute(
            (
                (":TEXT",     "Two roads diverged in a yellow wood,"),
                (":ID",       &mut id),
                (":TEXT_OUT", &mut text_out),
                (":NTXT_OUT", &mut ntxt_out)
            )
        )?;
        assert_eq!(count, 1);
        assert_eq!(text_out, "Two roads diverged in a yellow wood,");
        assert_eq!(ntxt_out, "> Two roads diverged in a yellow wood,");
        assert!(id > 0);
        ids.push(id);

         let text = String::from("And sorry I could not travel both");
        let count = stmt.execute(
            (
                (":TEXT",     text.as_str()),
                (":ID",       &mut id),
                (":TEXT_OUT", &mut text_out),
                (":NTXT_OUT", &mut ntxt_out)
            )
        )?;
        assert_eq!(count, 1);
        assert_eq!(text_out, "And sorry I could not travel both");
        assert_eq!(ntxt_out, "> And sorry I could not travel both");
        assert!(id > 0);
        ids.push(id);

        let mut text_out = Varchar::with_capacity(97, &session)?;
        assert!(text_out.capacity()? >= 97, "text out capacity");
        let mut ntxt_out = Varchar::with_capacity(99, &session)?;
        assert!(ntxt_out.capacity()? >= 99, "Ntxt out capacity");
        let text = Varchar::from("And be one traveler, long I stood", &session)?;
        let count = stmt.execute(
            (
                (":TEXT",     text.as_str()),
                (":ID",       &mut id),
                (":TEXT_OUT", &mut text_out),
                (":NTXT_OUT", &mut ntxt_out)
            )
        )?;
        assert_eq!(count, 1);
        assert_eq!(text_out.as_str(), "And be one traveler, long I stood");
        assert_eq!(ntxt_out.as_str(), "> And be one traveler, long I stood");
        assert!(id > 0);
        ids.push(id);

        let stmt = session.prepare("SELECT text, ntext FROM test_character_data WHERE id = :ID")?;

        let row = stmt.query_single(ids[0])?.unwrap();
        let text : &str = row.get("TEXT")?;
        assert_eq!(text, "Two roads diverged in a yellow wood,");
        let text : &str = row.get("NTEXT")?;
        assert_eq!(text, "> Two roads diverged in a yellow wood,");

        if let Some(row) = stmt.query_single(ids[1])? {
            let text : String = row.get(0)?;
            assert_eq!(text.as_str(), "And sorry I could not travel both");
            let text : String = row.get(1)?;
            assert_eq!(text.as_str(), "> And sorry I could not travel both");
        }

        if let Some(row) = stmt.query_single(ids[2])? {
            let text : Varchar = row.get("TEXT")?;
            assert_eq!(text.as_str(), "And be one traveler, long I stood");
            let text : Varchar = row.get("NTEXT")?;
            assert_eq!(text.as_str(), "> And be one traveler, long I stood");
        }

        Ok(())
    }

    #[test]
    fn datetime_datatypes() -> Result<()> {
        use std::cmp::Ordering::Equal;

        let dbname = std::env::var("DBNAME").expect("database name");
        let dbuser = std::env::var("DBUSER").expect("user name");
        let dbpass = std::env::var("DBPASS").expect("password");
        let oracle = env()?;
        let session = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = session.prepare("
            DECLARE
                name_already_used EXCEPTION; PRAGMA EXCEPTION_INIT(name_already_used, -955);
            BEGIN
                EXECUTE IMMEDIATE '
                    CREATE TABLE test_datetime_data (
                        id      NUMBER GENERATED ALWAYS AS IDENTITY,
                        dt      DATE,
                        ts      TIMESTAMP(9),
                        tsz     TIMESTAMP(9) WITH TIME ZONE,
                        tsl     TIMESTAMP(9) WITH LOCAL TIME ZONE,
                        iym     INTERVAL YEAR(9) TO MONTH,
                        ids     INTERVAL DAY(8) TO SECOND(9)
                    )
                ';
            EXCEPTION
              WHEN name_already_used THEN NULL;
            END;
        ")?;
        stmt.execute(())?;

        let stmt = session.prepare("
            INSERT INTO test_datetime_data (dt, ts, tsz, tsl, iym, ids) VALUES (:DT, :TS, :TSZ, :TSL, :IYM, :IDS)
            RETURNING id, dt, ts, tsz, tsl, iym, ids INTO :ID, :ODT, :OTS, :OTSZ, :OTSL, :OIYM, :OIDS
        ")?;
        let mut id = 0u32;

        let dt  = Date::with_date_and_time(1969, 7, 24, 16, 50, 35, &session);
        let ts  = Timestamp::with_date_and_time(1969, 7, 24, 16, 50, 35, 1, "", &session)?;
        let tsz = TimestampTZ::with_date_and_time(1969, 7, 24, 16, 50, 35, 2, "UTC", &session)?;
        let tsl = TimestampLTZ::with_date_and_time(1969, 7, 24, 16, 50, 35, 3, "UTC", &session)?;
        let iym = IntervalYM::with_duration(123, 11, &session)?;
        let ids = IntervalDS::with_duration(256, 16, 15, 37, 123456789, &session)?;

        let mut dt_out  = Date::new(&session);
        let mut ts_out  = Timestamp::new(&session)?;
        let mut tsz_out = TimestampTZ::new(&session)?;
        let mut tsl_out = TimestampLTZ::new(&session)?;
        let mut iym_out = IntervalYM::new(&session)?;
        let mut ids_out = IntervalDS::new(&session)?;

        let count = stmt.execute((
            (":DT",  &dt),
            (":TS",  &ts),
            (":TSZ", &tsz),
            (":TSL", &tsl),
            (":IYM", &iym),
            (":IDS", &ids),
            (":ID",   &mut id),
            (":ODT",  &mut dt_out),
            (":OTS",  &mut ts_out),
            (":OTSZ", &mut tsz_out),
            (":OTSL", &mut tsl_out),
            (":OIYM", &mut iym_out),
            (":OIDS", &mut ids_out),
        ))?;
        assert_eq!(count, 1);
        assert!(id > 0);
        assert_eq!(dt_out.compare(&dt)?, Equal);
        assert_eq!(ts_out.compare(&ts)?, Equal);
        assert_eq!(tsz_out.compare(&tsz)?, Equal);
        assert_eq!(tsl_out.compare(&tsl)?, Equal);
        assert_eq!(iym_out.compare(&iym)?, Equal);
        assert_eq!(ids_out.compare(&ids)?, Equal);

        let count = stmt.execute((
            (":DT",  dt),
            (":TS",  ts),
            (":TSZ", tsz),
            (":TSL", tsl),
            (":IYM", iym),
            (":IDS", ids),
            (":ID",   &mut id),
            (":ODT",  &mut dt_out),
            (":OTS",  &mut ts_out),
            (":OTSZ", &mut tsz_out),
            (":OTSL", &mut tsl_out),
            (":OIYM", &mut iym_out),
            (":OIDS", &mut ids_out)
        ))?;
        assert_eq!(count, 1);
        assert!(id > 0);

        // IN arguments have just been moved. Re-create them for comparisons:
        let dt2  = Date::with_date_and_time(1969, 7, 24, 16, 50, 35, &session);
        let ts2  = Timestamp::with_date_and_time(1969, 7, 24, 16, 50, 35, 1, "", &session)?;
        let tsz2 = TimestampTZ::with_date_and_time(1969, 7, 24, 16, 50, 35, 2, "UTC", &session)?;
        let tsl2 = TimestampLTZ::with_date_and_time(1969, 7, 24, 16, 50, 35, 3, "UTC", &session)?;
        let iym2 = IntervalYM::with_duration(123, 11, &session)?;
        let ids2 = IntervalDS::with_duration(256, 16, 15, 37, 123456789, &session)?;

        assert_eq!(dt_out.compare(&dt2)?, Equal);
        assert_eq!(ts_out.compare(&ts2)?, Equal);
        assert_eq!(tsz_out.compare(&tsz2)?, Equal);
        assert_eq!(tsl_out.compare(&tsl2)?, Equal);
        assert_eq!(iym_out.compare(&iym2)?, Equal);
        assert_eq!(ids_out.compare(&ids2)?, Equal);


        let stmt = session.prepare("SELECT dt, ts, tsz, tsl, iym, ids FROM test_datetime_data WHERE id = :ID")?;
        let row = stmt.query_single(id)?.unwrap();
        let val : Date = row.get("DT")?;
        assert_eq!(val.compare(&dt2)?, Equal);
        let val : Timestamp = row.get("TS")?;
        assert_eq!(val.compare(&ts2)?, Equal);
        let val : TimestampTZ = row.get("TSZ")?;
        assert_eq!(val.compare(&tsz2)?, Equal);
        let val : TimestampLTZ = row.get("TSL")?;
        assert_eq!(val.compare(&tsl2)?, Equal);
        let val : IntervalYM = row.get("IYM")?;
        assert_eq!(val.compare(&iym2)?, Equal);
        let val : IntervalDS = row.get("IDS")?;
        assert_eq!(val.compare(&ids2)?, Equal);

        Ok(())
    }

    #[test]
    fn large_object_datatypes() -> Result<()> {
        let dbname = std::env::var("DBNAME").expect("database name");
        let dbuser = std::env::var("DBUSER").expect("user name");
        let dbpass = std::env::var("DBPASS").expect("password");
        let oracle = env()?;
        let session = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = session.prepare("
            DECLARE
                name_already_used EXCEPTION; PRAGMA EXCEPTION_INIT(name_already_used, -955);
            BEGIN
                EXECUTE IMMEDIATE '
                    CREATE TABLE test_large_object_data (
                        id      NUMBER GENERATED ALWAYS AS IDENTITY,
                        bin     BLOB,
                        text    CLOB,
                        ntxt    NCLOB,
                        fbin    BFILE
                    )
                ';
            EXCEPTION
              WHEN name_already_used THEN NULL;
            END;
        ")?;
        stmt.execute(())?;

        let stmt = session.prepare("
            INSERT INTO test_large_object_data (bin, text, ntxt, fbin)
            VALUES (Empty_Blob(), Empty_Clob(), Empty_Clob(), BFileName(:DIR,:NAME))
            RETURNING id INTO :ID
        ")?;
        let mut id = 0;
        let count = stmt.execute(("MEDIA_DIR", "hello_world.txt", &mut id))?;
        assert_eq!(count, 1);
        assert!(id > 0);

        // Content of `hello_world.txt`:
        let data = [0xfeu8, 0xff, 0x00, 0x48, 0x00, 0x65, 0x00, 0x6c, 0x00, 0x6c, 0x00, 0x6f, 0x00, 0x2c, 0x00, 0x20, 0x00, 0x57, 0x00, 0x6f, 0x00, 0x72, 0x00, 0x6c, 0x00, 0x64, 0x00, 0x21];

        // Can only read BFILEs
        let stmt = session.prepare("SELECT fbin FROM test_large_object_data WHERE id = :ID")?;
        let row = stmt.query_single(&id)?.unwrap();
        let lob : BFile = row.get("FBIN")?;

        assert!(lob.file_exists()?);
        let (dir, name) = lob.file_name()?; // if we forgot :-)
        assert_eq!(dir, "MEDIA_DIR");
        assert_eq!(name, "hello_world.txt");

        assert!(!lob.is_file_open()?);
        lob.open_file()?;
        let mut lob_data = Vec::new();
        lob.read(0, 28, &mut lob_data)?;
        lob.close_file()?;
        assert_eq!(lob_data, data);

        // Note: To modify a LOB column or attribute (write, copy, trim, and so forth), you must lock the row containing the LOB.
        // One way to do this is to use a SELECT...FOR UPDATE statement to select the locator before performing the operation.

        let stmt = session.prepare("SELECT bin FROM test_large_object_data WHERE id = :ID FOR UPDATE")?;
        let row = stmt.query_single(&id)?.unwrap();
        let lob : BLOB = row.get(0)?;

        lob.open()?;
        let count = lob.append(&data)?;
        assert_eq!(count, 28);
        lob.close()?;

        // Read it (in another transaction)

        let stmt = session.prepare("SELECT bin FROM test_large_object_data WHERE id = :ID")?;
        let row = stmt.query_single(&id)?.unwrap();
        let lob : BLOB = row.get(0)?;
        let mut lob_data = Vec::new();
        lob.read(0, 28, &mut lob_data)?;
        assert_eq!(lob_data, data);


        let stmt = session.prepare("SELECT text FROM test_large_object_data WHERE id = :ID FOR UPDATE")?;
        let row = stmt.query_single(&id)?.unwrap();
        let lob : CLOB = row.get(0)?;
        assert!(!lob.is_nclob()?);

        let text = "Two roads diverged in a yellow wood, And sorry I could not travel both And be one traveler, long I stood And looked down one as far as I could To where it bent in the undergrowth; Then took the other, as just as fair, And having perhaps the better claim, Because it was grassy and wanted wear; Though as for that the passing there Had worn them really about the same, And both that morning equally lay In leaves no step had trodden black. Oh, I kept the first for another day! Yet knowing how way leads on to way, I doubted if I should ever come back. I shall be telling this with a sigh Somewhere ages and ages hence: Two roads diverged in a wood, and I— I took the one less traveled by, And that has made all the difference.";

        lob.open()?;
        let count = lob.append(text)?;
        assert_eq!(count, 726);
        assert_eq!(lob.len()?, 726);
        lob.close()?;

        let stmt = session.prepare("SELECT text FROM test_large_object_data WHERE id = :ID")?;
        let row = stmt.query_single(&id)?.unwrap();
        let lob : CLOB = row.get(0)?;
        assert!(!lob.is_nclob()?);

        let mut lob_text = String::new();
        lob.read(0, 726, &mut lob_text)?;
        assert_eq!(lob_text, text);


        let stmt = session.prepare("SELECT ntxt FROM test_large_object_data WHERE id = :ID FOR UPDATE")?;
        let row = stmt.query_single(&id)?.unwrap();
        let lob : CLOB = row.get(0)?;
        assert!(lob.is_nclob()?);

        lob.open()?;
        let count = lob.append(text)?;
        assert_eq!(count, 726);
        assert_eq!(lob.len()?, 726);
        lob.close()?;

        let stmt = session.prepare("SELECT ntxt FROM test_large_object_data WHERE id = :ID")?;
        let row = stmt.query_single(&id)?.unwrap();
        let lob : CLOB = row.get(0)?;
        assert!(lob.is_nclob()?);

        let mut lob_text = String::new();
        lob.read(0, 726, &mut lob_text)?;
        assert_eq!(lob_text, text);

        Ok(())
    }

    #[test]
    fn long_and_raw_datatypes() -> Result<()> {
        let dbname = std::env::var("DBNAME").expect("database name");
        let dbuser = std::env::var("DBUSER").expect("user name");
        let dbpass = std::env::var("DBPASS").expect("password");
        let oracle = env()?;
        let session = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = session.prepare("
            DECLARE
                name_already_used EXCEPTION; PRAGMA EXCEPTION_INIT(name_already_used, -955);
            BEGIN
                EXECUTE IMMEDIATE '
                    CREATE TABLE long_and_raw_test_data (
                        id      NUMBER GENERATED ALWAYS AS IDENTITY,
                        bin     RAW(100),
                        text    LONG
                    )
                ';
            EXCEPTION
              WHEN name_already_used THEN NULL;
            END;
        ")?;
        stmt.execute(())?;

        // Cannot return LONG
        let stmt = session.prepare("
            INSERT INTO long_and_raw_test_data (bin, text) VALUES (:BIN, :TEXT)
            RETURNING id, bin INTO :ID, :OBIN
        ")?;
        let data = [0xfeu8, 0xff, 0x00, 0x48, 0x00, 0x65, 0x00, 0x6c, 0x00, 0x6c, 0x00, 0x6f, 0x00, 0x2c, 0x00, 0x20, 0x00, 0x57, 0x00, 0x6f, 0x00, 0x72, 0x00, 0x6c, 0x00, 0x64, 0x00, 0x21];
        let text = "When I have fears that I may cease to be Before my pen has gleaned my teeming brain, Before high-pilèd books, in charactery, Hold like rich garners the full ripened grain; When I behold, upon the night’s starred face, Huge cloudy symbols of a high romance, And think that I may never live to trace Their shadows with the magic hand of chance; And when I feel, fair creature of an hour, That I shall never look upon thee more, Never have relish in the faery power Of unreflecting love—then on the shore Of the wide world I stand alone, and think Till love and fame to nothingness do sink.";
        let mut id = 0;
        let mut data_out = Vec::with_capacity(30);
        let count = stmt.execute((data.as_ref(), text, &mut id, &mut data_out))?;
        assert_eq!(count, 1);
        assert!(id > 0);
        assert_eq!(data_out.as_slice(), data.as_ref());

        let stmt = session.prepare("SELECT bin, text FROM long_and_raw_test_data WHERE id = :ID")?;
        // without explicit resizing via `stmt.set_max_long_size` (before `stmt.query`) TEXT output is limited to 32768
        let row = stmt.query_single(&id)?.unwrap();
        let bin : Raw = row.get("BIN")?;
        let txt : &str = row.get("TEXT")?;
        assert_eq!(bin.as_bytes(), data.as_ref());
        assert_eq!(txt, text);

        Ok(())
    }

    #[test]
    fn long_raw_datatype() -> Result<()> {
        let dbname = std::env::var("DBNAME").expect("database name");
        let dbuser = std::env::var("DBUSER").expect("user name");
        let dbpass = std::env::var("DBPASS").expect("password");
        let oracle = env()?;
        let session = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = session.prepare("
            DECLARE
                name_already_used EXCEPTION; PRAGMA EXCEPTION_INIT(name_already_used, -955);
            BEGIN
                EXECUTE IMMEDIATE '
                    CREATE TABLE test_long_raw_data (
                        id      NUMBER GENERATED ALWAYS AS IDENTITY,
                        bin     LONG RAW
                    )
                ';
            EXCEPTION
              WHEN name_already_used THEN NULL;
            END;
        ")?;
        stmt.execute(())?;

        let stmt = session.prepare("
            INSERT INTO test_long_raw_data (bin) VALUES (:BIN)
            RETURNING id INTO :ID
        ")?;
        let data = [0xfeu8, 0xff, 0x00, 0x48, 0x00, 0x65, 0x00, 0x6c, 0x00, 0x6c, 0x00, 0x6f, 0x00, 0x2c, 0x00, 0x20, 0x00, 0x57, 0x00, 0x6f, 0x00, 0x72, 0x00, 0x6c, 0x00, 0x64, 0x00, 0x21];
        let mut id = 0;
        let count = stmt.execute((&data[..], &mut id, ()))?;
        assert_eq!(count, 1);
        assert!(id > 0);

        let stmt = session.prepare("SELECT bin FROM test_long_raw_data WHERE id = :ID")?;
        // without explicit resizing via `stmt.set_max_long_size` (before `stmt.query`) BIN output is limited to 32768
        let row = stmt.query_single(&id)?.unwrap();
        let bin : &[u8] = row.get(0)?;
        assert_eq!(bin, &data[..]);

        Ok(())
    }

    #[test]
    fn numeric_datatypes() -> Result<()> {
        use std::cmp::Ordering::Equal;

        let dbname = std::env::var("DBNAME").expect("database name");
        let dbuser = std::env::var("DBUSER").expect("user name");
        let dbpass = std::env::var("DBPASS").expect("password");
        let oracle = env()?;
        let session = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = session.prepare("
            DECLARE
                name_already_used EXCEPTION; PRAGMA EXCEPTION_INIT(name_already_used, -955);
            BEGIN
                EXECUTE IMMEDIATE '
                    CREATE TABLE test_numeric_data (
                        id      NUMBER GENERATED ALWAYS AS IDENTITY,
                        num     NUMBER,
                        flt     BINARY_FLOAT,
                        dbl     BINARY_DOUBLE
                    )
                ';
            EXCEPTION
              WHEN name_already_used THEN NULL;
            END;
        ")?;
        stmt.execute(())?;

        let stmt = session.prepare("
            INSERT INTO test_numeric_data (num, flt, dbl) VALUES (:NUM, :NUM, :NUM)
            RETURNING id, num, flt, dbl INTO :ID, :ONUM, :OFLT, :ODBL
        ")?;
        let src_num = Number::from_string("3.141592653589793238462643383279502884197", "9.999999999999999999999999999999999999999", &session)?;
        let mut id = 0;
        let mut num = Number::new(&session);
        let mut flt = 0f32;
        let mut dbl = 0f64;
        let count = stmt.execute(
            (
                (":NUM",  &src_num),
                (":ID",   &mut id),
                (":ONUM", &mut num),
                (":OFLT", &mut flt),
                (":ODBL", &mut dbl),
            )
        )?;
        assert_eq!(count, 1);
        assert!(id > 0);
        assert_eq!(num.compare(&src_num)?, Equal);
        assert!(3.141592653589792 < dbl && dbl < 3.141592653589794);
        assert!(3.1415926 < flt && flt < 3.1415929);

        let stmt = session.prepare("SELECT num, flt, dbl FROM test_numeric_data WHERE id = :ID")?;
        let row = stmt.query_single(&id)?.unwrap();
        let num : Number = row.get("NUM")?;
        let flt : f32 = row.get("FLT")?;
        let dbl : f64 = row.get("DBL")?;
        assert_eq!(num.compare(&src_num)?, Equal);
        assert!(3.141592653589792 < dbl && dbl < 3.141592653589794);
        assert!(3.1415926 < flt && flt < 3.1415929);
        assert_eq!(num.to_string("TM")?, "3.1415926535897932384626433832795028842");

        Ok(())
    }

    #[test]
    fn rowid_datatype() -> Result<()> {
        let dbname = std::env::var("DBNAME").expect("database name");
        let dbuser = std::env::var("DBUSER").expect("user name");
        let dbpass = std::env::var("DBPASS").expect("password");
        let oracle = env()?;
        let session = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = session.prepare("
            SELECT ROWID, manager_id
              FROM hr.employees
             WHERE employee_id = :ID
               FOR UPDATE
        ")?;
        let row = stmt.query_single(107)?.unwrap();
        let implicit_rowid = row.rowid()?;
        let str_rowid : String = row.get(0)?;
        assert_eq!(str_rowid, implicit_rowid.to_string(&session)?);
        let explicit_rowid : RowID = row.get(0)?;
        assert_eq!(explicit_rowid.to_string(&session)?, implicit_rowid.to_string(&session)?);
        let manager_id: u32 = row.get(1)?;
        assert_eq!(manager_id, 103, "employee ID of Alexander Hunold");

        let stmt = session.prepare("
            UPDATE hr.employees
               SET manager_id = :MID
             WHERE rowid = :RID
        ")?;
        let num_updated = stmt.execute(((":MID", 103), (":RID", &implicit_rowid)))?;
        assert_eq!(num_updated, 1);
        session.rollback()?;
        Ok(())
    }

    #[test]
    fn ref_cursor() -> Result<()> {
        use std::cmp::Ordering::Equal;

        let dbname = std::env::var("DBNAME").expect("database name");
        let dbuser = std::env::var("DBUSER").expect("user name");
        let dbpass = std::env::var("DBPASS").expect("password");
        let oracle = env()?;
        let session = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = session.prepare("
            BEGIN
                OPEN :lowest_payed_employee FOR
                    SELECT department_name, first_name, last_name, salary
                      FROM (
                            SELECT first_name, last_name, salary, department_id
                                 , ROW_NUMBER() OVER (ORDER BY salary) ord
                              FROM hr.employees
                           ) e
                      JOIN hr.departments d
                        ON d.department_id = e.department_id
                     WHERE ord = 1
                ;
                OPEN :median_salary_employees FOR
                    SELECT department_name, first_name, last_name, salary
                      FROM (
                            SELECT first_name, last_name, salary, department_id
                                 , MEDIAN(salary) OVER () median_salary
                              FROM hr.employees
                           ) e
                      JOIN hr.departments d
                        ON d.department_id = e.department_id
                     WHERE salary = median_salary
                  ORDER BY department_name, last_name, first_name
                ;
            END;
        ")?;

        let mut lowest_payed_employee   = Cursor::new(&stmt)?;
        let mut median_salary_employees = Cursor::new(&stmt)?;

        stmt.execute((
            (":LOWEST_PAYED_EMPLOYEE",   &mut lowest_payed_employee  ),
            (":MEDIAN_SALARY_EMPLOYEES", &mut median_salary_employees),
        ))?;

        let expected_lowest_salary = Number::from_int(2100, &session)?;
        let expected_median_salary = Number::from_int(6200, &session)?;

        let rows = lowest_payed_employee.rows()?;
        let row = rows.next()?.unwrap();

        let department_name : &str = row.get(0)?;
        let first_name : &str = row.get(1)?;
        let last_name : &str = row.get(2)?;
        let salary : Number = row.get(3)?;

        assert_eq!(department_name, "Shipping");
        assert_eq!(first_name, "TJ");
        assert_eq!(last_name, "Olson");
        assert_eq!(salary.compare(&expected_lowest_salary)?, Equal);

        let row = rows.next()?;
        assert!(row.is_none());

        let rows = median_salary_employees.rows()?;

        let row = rows.next()?.unwrap();
        let department_name : &str = row.get(0)?;
        let first_name : &str = row.get(1)?;
        let last_name : &str = row.get(2)?;
        let salary : Number = row.get(3)?;

        assert_eq!(department_name, "Sales");
        assert_eq!(first_name, "Amit");
        assert_eq!(last_name, "Banda");
        assert_eq!(salary.compare(&expected_median_salary)?, Equal);

        let row = rows.next()?.unwrap();

        let department_name : &str = row.get(0)?;
        let first_name : &str = row.get(1)?;
        let last_name : &str = row.get(2)?;
        let salary : Number = row.get(3)?;

        assert_eq!(department_name, "Sales");
        assert_eq!(first_name, "Charles");
        assert_eq!(last_name, "Johnson");
        assert_eq!(salary.compare(&expected_median_salary)?, Equal);

        let row = rows.next()?;
        assert!(row.is_none());

        Ok(())
    }

    #[test]
    fn ref_cursor_result() -> Result<()> {
        use std::cmp::Ordering::Equal;

        let dbname = std::env::var("DBNAME").expect("database name");
        let dbuser = std::env::var("DBUSER").expect("user name");
        let dbpass = std::env::var("DBPASS").expect("password");
        let oracle = env()?;
        let session = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = session.prepare("
            DECLARE
                c1 SYS_REFCURSOR;
                c2 SYS_REFCURSOR;
            BEGIN
                OPEN c1 FOR
                    SELECT department_name, first_name, last_name, salary
                      FROM (
                            SELECT first_name, last_name, salary, department_id
                                 , ROW_NUMBER() OVER (ORDER BY salary) ord
                              FROM hr.employees
                           ) e
                      JOIN hr.departments d
                        ON d.department_id = e.department_id
                     WHERE ord = 1
                ;
                OPEN c2 FOR
                    SELECT department_name, first_name, last_name, salary
                      FROM (
                            SELECT first_name, last_name, salary, department_id
                                 , MEDIAN(salary) OVER () median_salary
                              FROM hr.employees
                           ) e
                      JOIN hr.departments d
                        ON d.department_id = e.department_id
                     WHERE salary = median_salary
                  ORDER BY department_name, last_name, first_name
                ;
                DBMS_SQL.RETURN_RESULT(c1);
                DBMS_SQL.RETURN_RESULT(c2);
            END;
        ")?;

        let expected_lowest_salary = Number::from_int(2100, &session)?;
        let expected_median_salary = Number::from_int(6200, &session)?;

        stmt.execute(())?;

        let lowest_payed_employee = stmt.next_result()?.unwrap();

        let rows = lowest_payed_employee.rows()?;
        let row = rows.next()?.unwrap();

        let department_name : &str = row.get(0)?;
        let first_name : &str = row.get(1)?;
        let last_name : &str = row.get(2)?;
        let salary : Number = row.get(3)?;

        assert_eq!(department_name, "Shipping");
        assert_eq!(first_name, "TJ");
        assert_eq!(last_name, "Olson");
        assert_eq!(salary.compare(&expected_lowest_salary)?, Equal);

        let row = rows.next()?;
        assert!(row.is_none());

        let median_salary_employees = stmt.next_result()?.unwrap();

        let rows = median_salary_employees.rows()?;

        let row = rows.next()?.unwrap();
        let department_name : &str = row.get(0)?;
        let first_name : &str = row.get(1)?;
        let last_name : &str = row.get(2)?;
        let salary : Number = row.get(3)?;

        assert_eq!(department_name, "Sales");
        assert_eq!(first_name, "Amit");
        assert_eq!(last_name, "Banda");
        assert_eq!(salary.compare(&expected_median_salary)?, Equal);

        let row = rows.next()?.unwrap();

        let department_name : &str = row.get(0)?;
        let first_name : &str = row.get(1)?;
        let last_name : &str = row.get(2)?;
        let salary : Number = row.get(3)?;

        assert_eq!(department_name, "Sales");
        assert_eq!(first_name, "Charles");
        assert_eq!(last_name, "Johnson");
        assert_eq!(salary.compare(&expected_median_salary)?, Equal);

        let row = rows.next()?;
        assert!(row.is_none());

        assert!(stmt.next_result()?.is_none());

        Ok(())
    }

    #[test]
    fn ref_cursor_column() -> Result<()> {
        let dbname = std::env::var("DBNAME").expect("database name");
        let dbuser = std::env::var("DBUSER").expect("user name");
        let dbpass = std::env::var("DBPASS").expect("password");
        let oracle = env()?;
        let session = oracle.connect(&dbname, &dbuser, &dbpass)?;

        let stmt = session.prepare("
            SELECT last_name
                 , CURSOR(
                        SELECT department_name
                          FROM hr.departments
                         WHERE department_id IN (
                                    SELECT department_id
                                      FROM hr.employees
                                     WHERE last_name = e.last_name)
                      ORDER BY department_name
                   ) AS departments
              FROM (
                    SELECT distinct last_name
                      FROM hr.employees
                     WHERE last_name = :last_name
                   ) e
        ")?;
        let row = stmt.query_single("King")?.unwrap();
        let last_name : &str = row.get(0)?;
        assert_eq!(last_name, "King");

        let departments : Cursor = row.get(1)?;
        let dept_rows = departments.rows()?;
        let dept_row = dept_rows.next()?.unwrap();

        let department_name : &str = dept_row.get(0)?;
        assert_eq!(department_name, "Executive");

        let dept_row = dept_rows.next()?.unwrap();
        let department_name : &str = dept_row.get(0)?;
        assert_eq!(department_name, "Sales");

        assert!(dept_rows.next()?.is_none());

        Ok(())
    }
}

#[cfg(feature="nonblocking")]
mod nonblocking {
    use sibyl::*;

    #[test]
    fn character_datatypes() -> Result<()> {
        block_on(async {
            use once_cell::sync::OnceCell;

            static ORACLE : OnceCell<Environment> = OnceCell::new();
            let oracle = ORACLE.get_or_try_init(|| {
                sibyl::env()
            })?;

            let dbname = std::env::var("DBNAME").expect("database name");
            let dbuser = std::env::var("DBUSER").expect("user name");
            let dbpass = std::env::var("DBPASS").expect("password");

            let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;

            let stmt = session.prepare("
                DECLARE
                    name_already_used EXCEPTION; PRAGMA EXCEPTION_INIT(name_already_used, -955);
                BEGIN
                    EXECUTE IMMEDIATE '
                        CREATE TABLE test_character_data (
                            id      NUMBER GENERATED ALWAYS AS IDENTITY,
                            text    VARCHAR2(97),
                            ntext   NVARCHAR2(99)
                        )
                    ';
                EXCEPTION
                  WHEN name_already_used THEN NULL;
                END;
            ").await?;
            stmt.execute(()).await?;

            let stmt = session.prepare("
                INSERT INTO test_character_data (text, ntext) VALUES (:TEXT, '> ' || :TEXT)
                RETURNING id, text, ntext INTO :ID, :TEXT_OUT, :NTXT_OUT
            ").await?;

            let mut ids = Vec::with_capacity(3);
            let mut id = 0;

            let mut text_out = String::with_capacity(97);
            let mut ntxt_out = String::with_capacity(99);
            let count = stmt.execute(
                (
                    (":TEXT", "Two roads diverged in a yellow wood,"),
                    (":ID", &mut id),
                    (":TEXT_OUT", &mut text_out),
                    (":NTXT_OUT", &mut ntxt_out)
                )
            ).await?;
            assert_eq!(count, 1);
            assert_eq!(text_out, "Two roads diverged in a yellow wood,");
            assert_eq!(ntxt_out, "> Two roads diverged in a yellow wood,");
            assert!(id > 0);
            ids.push(id);

            let text = String::from("And sorry I could not travel both");
            let count = stmt.execute(
                (
                    (":TEXT", text.as_str()),
                    (":ID", &mut id),
                    (":TEXT_OUT", &mut text_out),
                    (":NTXT_OUT", &mut ntxt_out)
                )
            ).await?;
            assert_eq!(count, 1);
            assert_eq!(text_out, "And sorry I could not travel both");
            assert_eq!(ntxt_out, "> And sorry I could not travel both");
            assert!(id > 0);
            ids.push(id);

            let text = Varchar::from("And be one traveler, long I stood", &session)?;
            let mut text_out = Varchar::with_capacity(97, &session)?;
            let mut ntxt_out = Varchar::with_capacity(99, &session)?;
            let count = stmt.execute(
                (
                    (":TEXT", text.as_str()),
                    (":ID", &mut id),
                    (":TEXT_OUT", &mut text_out),
                    (":NTXT_OUT", &mut ntxt_out)
                )
            ).await?;
            assert_eq!(count, 1);
            assert_eq!(text_out.as_str(), "And be one traveler, long I stood");
            assert_eq!(ntxt_out.as_str(), "> And be one traveler, long I stood");
            ids.push(id);

            let stmt = session.prepare("SELECT text, ntext FROM test_character_data WHERE id = :ID").await?;

            let rows = stmt.query(ids[0]).await?;
            let row  = rows.next().await?.unwrap();
            let text : &str = row.get("TEXT")?;
            assert_eq!(text, "Two roads diverged in a yellow wood,");
            let text : &str = row.get("NTEXT")?;
            assert_eq!(text, "> Two roads diverged in a yellow wood,");
            assert!(rows.next().await?.is_none());

            let rows = stmt.query(ids[1]).await?;
            let row  = rows.next().await?.unwrap();
            let text : String = row.get(0)?;
            assert_eq!(text.as_str(), "And sorry I could not travel both");
            let text : String = row.get(1)?;
            assert_eq!(text.as_str(), "> And sorry I could not travel both");
            assert!(rows.next().await?.is_none());

            let rows = stmt.query(ids[2]).await?;
            let row  = rows.next().await?.unwrap();
            let text : Varchar = row.get("TEXT")?;
            assert_eq!(text.as_str(), "And be one traveler, long I stood");
            let text : Varchar = row.get("NTEXT")?;
            assert_eq!(text.as_str(), "> And be one traveler, long I stood");
            assert!(rows.next().await?.is_none());

            Ok(())
        })
    }

    #[test]
    fn datetime_datatypes() -> Result<()> {
        block_on(async {
            use std::cmp::Ordering::Equal;
            use once_cell::sync::OnceCell;

            static ORACLE : OnceCell<Environment> = OnceCell::new();
            let oracle = ORACLE.get_or_try_init(|| {
                sibyl::env()
            })?;

            let dbname = std::env::var("DBNAME").expect("database name");
            let dbuser = std::env::var("DBUSER").expect("user name");
            let dbpass = std::env::var("DBPASS").expect("password");

            let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;

            let stmt = session.prepare("
                DECLARE
                    name_already_used EXCEPTION; PRAGMA EXCEPTION_INIT(name_already_used, -955);
                BEGIN
                    EXECUTE IMMEDIATE '
                        CREATE TABLE test_datetime_data (
                            id      NUMBER GENERATED ALWAYS AS IDENTITY,
                            dt      DATE,
                            ts      TIMESTAMP(9),
                            tsz     TIMESTAMP(9) WITH TIME ZONE,
                            tsl     TIMESTAMP(9) WITH LOCAL TIME ZONE,
                            iym     INTERVAL YEAR(9) TO MONTH,
                            ids     INTERVAL DAY(8) TO SECOND(9)
                        )
                    ';
                EXCEPTION
                WHEN name_already_used THEN NULL;
                END;
            ").await?;
            stmt.execute(()).await?;

            let stmt = session.prepare("
                INSERT INTO test_datetime_data (dt, ts, tsz, tsl, iym, ids) VALUES (:DT, :TS, :TSZ, :TSL, :IYM, :IDS)
                RETURNING id, dt, ts, tsz, tsl, iym, ids INTO :ID, :ODT, :OTS, :OTSZ, :OTSL, :OIYM, :OIDS
            ").await?;

            let mut id = 0;

            let dt  = Date::with_date_and_time(1969, 7, 24, 16, 50, 35, &session);
            let ts  = Timestamp::with_date_and_time(1969, 7, 24, 16, 50, 35, 1, "", &session)?;
            let tsz = TimestampTZ::with_date_and_time(1969, 7, 24, 16, 50, 35, 2, "UTC", &session)?;
            let tsl = TimestampLTZ::with_date_and_time(1969, 7, 24, 16, 50, 35, 3, "UTC", &session)?;
            let iym = IntervalYM::with_duration(123, 11, &session)?;
            let ids = IntervalDS::with_duration(256, 16, 15, 37, 123456789, &session)?;

            let mut dt_out  = Date::new(&session);
            let mut ts_out  = Timestamp::new(&session)?;
            let mut tsz_out = TimestampTZ::new(&session)?;
            let mut tsl_out = TimestampLTZ::new(&session)?;
            let mut iym_out = IntervalYM::new(&session)?;
            let mut ids_out = IntervalDS::new(&session)?;

            let count = stmt.execute((
                (":DT",  &dt),
                (":TS",  &ts),
                (":TSZ", &tsz),
                (":TSL", &tsl),
                (":IYM", &iym),
                (":IDS", &ids),
                (":ID",   &mut id),
                (":ODT",  &mut dt_out),
                (":OTS",  &mut ts_out),
                (":OTSZ", &mut tsz_out),
                (":OTSL", &mut tsl_out),
                (":OIYM", &mut iym_out),
                (":OIDS", &mut ids_out)
            )).await?;
            assert_eq!(count, 1);
            assert!(id > 0);

            assert_eq!(dt_out.compare(&dt)?, Equal);
            assert_eq!(ts_out.compare(&ts)?, Equal);
            assert_eq!(tsz_out.compare(&tsz)?, Equal);
            assert_eq!(tsl_out.compare(&tsl)?, Equal);
            assert_eq!(iym_out.compare(&iym)?, Equal);
            assert_eq!(ids_out.compare(&ids)?, Equal);

            let count = stmt.execute((
                (":DT",  dt),
                (":TS",  ts),
                (":TSZ", tsz),
                (":TSL", tsl),
                (":IYM", iym),
                (":IDS", ids),
                (":ID",   &mut id),
                (":ODT",  &mut dt_out),
                (":OTS",  &mut ts_out),
                (":OTSZ", &mut tsz_out),
                (":OTSL", &mut tsl_out),
                (":OIYM", &mut iym_out),
                (":OIDS", &mut ids_out)
            )).await?;
            assert_eq!(count, 1);
            assert!(id > 0);

            // IN arguments have just been moved. Re-create them for comparisons:
            let dt2  = Date::with_date_and_time(1969, 7, 24, 16, 50, 35, &session);
            let ts2  = Timestamp::with_date_and_time(1969, 7, 24, 16, 50, 35, 1, "", &session)?;
            let tsz2 = TimestampTZ::with_date_and_time(1969, 7, 24, 16, 50, 35, 2, "UTC", &session)?;
            let tsl2 = TimestampLTZ::with_date_and_time(1969, 7, 24, 16, 50, 35, 3, "UTC", &session)?;
            let iym2 = IntervalYM::with_duration(123, 11, &session)?;
            let ids2 = IntervalDS::with_duration(256, 16, 15, 37, 123456789, &session)?;

            assert_eq!(dt_out.compare(&dt2)?, Equal);
            assert_eq!(ts_out.compare(&ts2)?, Equal);
            assert_eq!(tsz_out.compare(&tsz2)?, Equal);
            assert_eq!(tsl_out.compare(&tsl2)?, Equal);
            assert_eq!(iym_out.compare(&iym2)?, Equal);
            assert_eq!(ids_out.compare(&ids2)?, Equal);

            let stmt = session.prepare("SELECT dt, ts, tsz, tsl, iym, ids FROM test_datetime_data WHERE id = :ID").await?;
            let rows = stmt.query(id).await?;
            let row  = rows.next().await?.unwrap();
            let val : Date = row.get("DT")?;
            assert_eq!(val.compare(&dt2)?, Equal);
            let val : Timestamp = row.get("TS")?;
            assert_eq!(val.compare(&ts2)?, Equal);
            let val : TimestampTZ = row.get("TSZ")?;
            assert_eq!(val.compare(&tsz2)?, Equal);
            let val : TimestampLTZ = row.get("TSL")?;
            assert_eq!(val.compare(&tsl2)?, Equal);
            let val : IntervalYM = row.get("IYM")?;
            assert_eq!(val.compare(&iym2)?, Equal);
            let val : IntervalDS = row.get("IDS")?;
            assert_eq!(val.compare(&ids2)?, Equal);

            assert!(rows.next().await?.is_none());

            Ok(())
        })
    }

    #[test]
    fn long_and_raw_datatypes() -> Result<()> {
        block_on(async {
            use once_cell::sync::OnceCell;

            static ORACLE : OnceCell<Environment> = OnceCell::new();
            let oracle = ORACLE.get_or_try_init(|| {
                sibyl::env()
            })?;

            let dbname = std::env::var("DBNAME").expect("database name");
            let dbuser = std::env::var("DBUSER").expect("user name");
            let dbpass = std::env::var("DBPASS").expect("password");

            let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;

            let stmt = session.prepare("
                DECLARE
                    name_already_used EXCEPTION; PRAGMA EXCEPTION_INIT(name_already_used, -955);
                BEGIN
                    EXECUTE IMMEDIATE '
                        CREATE TABLE long_and_raw_test_data (
                            id      NUMBER GENERATED ALWAYS AS IDENTITY,
                            bin     RAW(100),
                            text    LONG
                        )
                    ';
                EXCEPTION
                WHEN name_already_used THEN NULL;
                END;
            ").await?;
            stmt.execute(()).await?;

            // Cannot return LONG
            let stmt = session.prepare("
                INSERT INTO long_and_raw_test_data (bin, text) VALUES (:BIN, :TEXT)
                RETURNING id, bin INTO :ID, :OBIN
            ").await?;

            let data = [0xfeu8, 0xff, 0x00, 0x48, 0x00, 0x65, 0x00, 0x6c, 0x00, 0x6c, 0x00, 0x6f, 0x00, 0x2c, 0x00, 0x20, 0x00, 0x57, 0x00, 0x6f, 0x00, 0x72, 0x00, 0x6c, 0x00, 0x64, 0x00, 0x21];
            let text = "When I have fears that I may cease to be Before my pen has gleaned my teeming brain, Before high-pilèd books, in charactery, Hold like rich garners the full ripened grain; When I behold, upon the night’s starred face, Huge cloudy symbols of a high romance, And think that I may never live to trace Their shadows with the magic hand of chance; And when I feel, fair creature of an hour, That I shall never look upon thee more, Never have relish in the faery power Of unreflecting love—then on the shore Of the wide world I stand alone, and think Till love and fame to nothingness do sink.";
            let mut id = 0;
            let mut data_out = Vec::with_capacity(30);
            let count = stmt.execute(
                (
                    (":BIN", &data[..]),
                    (":TEXT", text),
                    (":ID", &mut id),
                    (":OBIN", &mut data_out)
                )
            ).await?;
            assert_eq!(count, 1);
            assert!(id > 0);
            assert_eq!(data_out.as_slice(), &data[..]);

            let stmt = session.prepare("SELECT bin, text FROM long_and_raw_test_data WHERE id = :ID").await?;
            // without explicit resizing via `stmt.set_max_long_size` (before `stmt.query`) TEXT output is limited to 32768
            let row = stmt.query_single(&id).await?.unwrap();
            let bin : &[u8] = row.get("BIN")?;
            let txt : &str = row.get("TEXT")?;
            assert_eq!(bin, &data[..]);
            assert_eq!(txt, text);

            Ok(())
        })
    }

    #[test]
    fn long_raw_datatype() -> Result<()> {
        block_on(async {
            use once_cell::sync::OnceCell;

            static ORACLE : OnceCell<Environment> = OnceCell::new();
            let oracle = ORACLE.get_or_try_init(|| {
                sibyl::env()
            })?;

            let dbname = std::env::var("DBNAME").expect("database name");
            let dbuser = std::env::var("DBUSER").expect("user name");
            let dbpass = std::env::var("DBPASS").expect("password");

            let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;

            let stmt = session.prepare("
                DECLARE
                    name_already_used EXCEPTION; PRAGMA EXCEPTION_INIT(name_already_used, -955);
                BEGIN
                    EXECUTE IMMEDIATE '
                        CREATE TABLE test_long_raw_data (
                            id      NUMBER GENERATED ALWAYS AS IDENTITY,
                            bin     LONG RAW
                        )
                    ';
                EXCEPTION
                WHEN name_already_used THEN NULL;
                END;
            ").await?;
            stmt.execute(()).await?;

            let stmt = session.prepare("
                INSERT INTO test_long_raw_data (bin) VALUES (:BIN)
                RETURNING id INTO :ID
            ").await?;
            let data = [0xfeu8, 0xff, 0x00, 0x48, 0x00, 0x65, 0x00, 0x6c, 0x00, 0x6c, 0x00, 0x6f, 0x00, 0x2c, 0x00, 0x20, 0x00, 0x57, 0x00, 0x6f, 0x00, 0x72, 0x00, 0x6c, 0x00, 0x64, 0x00, 0x21];
            let mut id = 0;
            let count = stmt.execute(((":BIN", &data[..]), (":ID", &mut id))).await?;
            assert_eq!(count, 1);
            assert!(id > 0);

            let stmt = session.prepare("SELECT bin FROM test_long_raw_data WHERE id = :ID").await?;
            // without explicit resizing via `stmt.set_max_long_size` (before `stmt.query`) BIN output is limited to 32768
            let row = stmt.query_single(&id).await?.unwrap();
            let bin : &[u8] = row.get(0)?;
            assert_eq!(bin, &data[..]);

            Ok(())
        })
    }

    #[test]
    fn numeric_datatypes() -> Result<()> {
        block_on(async {
            use std::cmp::Ordering::Equal;
            use once_cell::sync::OnceCell;

            static ORACLE : OnceCell<Environment> = OnceCell::new();
            let oracle = ORACLE.get_or_try_init(|| {
                sibyl::env()
            })?;

            let dbname = std::env::var("DBNAME").expect("database name");
            let dbuser = std::env::var("DBUSER").expect("user name");
            let dbpass = std::env::var("DBPASS").expect("password");

            let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;

            let stmt = session.prepare("
                DECLARE
                    name_already_used EXCEPTION; PRAGMA EXCEPTION_INIT(name_already_used, -955);
                BEGIN
                    EXECUTE IMMEDIATE '
                        CREATE TABLE test_numeric_data (
                            id      NUMBER GENERATED ALWAYS AS IDENTITY,
                            num     NUMBER,
                            flt     BINARY_FLOAT,
                            dbl     BINARY_DOUBLE
                        )
                    ';
                EXCEPTION
                WHEN name_already_used THEN NULL;
                END;
            ").await?;
            stmt.execute(()).await?;

            let stmt = session.prepare("
                INSERT INTO test_numeric_data (num, flt, dbl) VALUES (:NUM, :NUM, :NUM)
                RETURNING id, num, flt, dbl INTO :ID, :ONUM, :OFLT, :ODBL
            ").await?;
            let src_num = Number::from_string("3.141592653589793238462643383279502884197", "9.999999999999999999999999999999999999999", &session)?;
            let mut id = 0;
            let mut num = Number::new(&session);
            let mut flt = 0f32;
            let mut dbl = 0f64;
            let count = stmt.execute(
                (
                    (":NUM",  &src_num),
                    (":ID",   &mut id),
                    (":ONUM", &mut num),
                    (":OFLT", &mut flt),
                    (":ODBL", &mut dbl),
                )
            ).await?;
            assert_eq!(count, 1);
            assert!(id > 0);
            assert_eq!(num.compare(&src_num)?, Equal);
            assert!(3.141592653589792 < dbl && dbl < 3.141592653589794);
            assert!(3.1415926 < flt && flt < 3.1415929);

            let stmt = session.prepare("SELECT num, flt, dbl FROM test_numeric_data WHERE id = :ID").await?;
            let row = stmt.query_single(&id).await?.unwrap();
            let num : Number = row.get("NUM")?;
            let flt : f32 = row.get("FLT")?;
            let dbl : f64 = row.get("DBL")?;
            assert_eq!(num.compare(&src_num)?, Equal);
            assert!(3.141592653589792 < dbl && dbl < 3.141592653589794);
            assert!(3.1415926 < flt && flt < 3.1415929);
            assert_eq!(num.to_string("TM")?, "3.1415926535897932384626433832795028842");

            Ok(())
        })
    }

    #[test]
    fn rowid_datatype() -> Result<()> {
        block_on(async {
            use once_cell::sync::OnceCell;

            static ORACLE : OnceCell<Environment> = OnceCell::new();
            let oracle = ORACLE.get_or_try_init(|| {
                sibyl::env()
            })?;

            let dbname = std::env::var("DBNAME").expect("database name");
            let dbuser = std::env::var("DBUSER").expect("user name");
            let dbpass = std::env::var("DBPASS").expect("password");

            let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;

            let stmt = session.prepare("
                SELECT ROWID, manager_id
                  FROM hr.employees
                 WHERE employee_id = :ID
                   FOR UPDATE
            ").await?;
            let row = stmt.query_single(107).await?.expect("selected row");

            let implicit_rowid = row.rowid()?;
            let str_rowid : String = row.get(0)?;
            assert_eq!(str_rowid, implicit_rowid.to_string(&session)?);

            let explicit_rowid : RowID = row.get(0)?;
            assert_eq!(explicit_rowid.to_string(&session)?, implicit_rowid.to_string(&session)?);

            let manager_id: u32 = row.get(1)?;
            assert_eq!(manager_id, 103, "employee ID of Alexander Hunold");

            let stmt = session.prepare("
                UPDATE hr.employees
                SET manager_id = :MID
                WHERE rowid = :RID
            ").await?;
            let num_updated = stmt.execute((
                (":MID", 103 ),
                (":RID", &implicit_rowid),
            )).await?;
            assert_eq!(num_updated, 1);
            session.rollback().await?;

            Ok(())
        })
    }

    #[test]
    fn ref_cursor() -> Result<()> {
        block_on(async {
            use std::cmp::Ordering::Equal;
            use once_cell::sync::OnceCell;

            static ORACLE : OnceCell<Environment> = OnceCell::new();
            let oracle = ORACLE.get_or_try_init(|| {
                sibyl::env()
            })?;

            let dbname = std::env::var("DBNAME").expect("database name");
            let dbuser = std::env::var("DBUSER").expect("user name");
            let dbpass = std::env::var("DBPASS").expect("password");

            let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
            let stmt = session.prepare("
                BEGIN
                    OPEN :lowest_payed_employee FOR
                        SELECT department_name, first_name, last_name, salary
                        FROM (
                                SELECT first_name, last_name, salary, department_id
                                    , ROW_NUMBER() OVER (ORDER BY salary) ord
                                FROM hr.employees
                            ) e
                        JOIN hr.departments d
                            ON d.department_id = e.department_id
                        WHERE ord = 1
                    ;
                    OPEN :median_salary_employees FOR
                        SELECT department_name, first_name, last_name, salary
                        FROM (
                                SELECT first_name, last_name, salary, department_id
                                    , MEDIAN(salary) OVER () median_salary
                                FROM hr.employees
                            ) e
                        JOIN hr.departments d
                            ON d.department_id = e.department_id
                        WHERE salary = median_salary
                    ORDER BY department_name, last_name, first_name
                    ;
                END;
            ").await?;

            let mut lowest_payed_employee   = Cursor::new(&stmt)?;
            let mut median_salary_employees = Cursor::new(&stmt)?;

            stmt.execute((
                ( ":LOWEST_PAYED_EMPLOYEE",   &mut lowest_payed_employee   ),
                ( ":MEDIAN_SALARY_EMPLOYEES", &mut median_salary_employees ),
            )).await?;

            let expected_lowest_salary = Number::from_int(2100, &session)?;
            let expected_median_salary = Number::from_int(6200, &session)?;

            let rows = lowest_payed_employee.rows().await?;
            let row = rows.next().await?.unwrap();

            let department_name : &str = row.get(0)?;
            let first_name : &str = row.get(1)?;
            let last_name : &str = row.get(2)?;
            let salary : Number = row.get(3)?;

            assert_eq!(department_name, "Shipping");
            assert_eq!(first_name, "TJ");
            assert_eq!(last_name, "Olson");
            assert_eq!(salary.compare(&expected_lowest_salary)?, Equal);

            let row = rows.next().await?;
            assert!(row.is_none());

            let rows = median_salary_employees.rows().await?;

            let row = rows.next().await?.unwrap();
            let department_name : &str = row.get(0)?;
            let first_name : &str = row.get(1)?;
            let last_name : &str = row.get(2)?;
            let salary : Number = row.get(3)?;

            assert_eq!(department_name, "Sales");
            assert_eq!(first_name, "Amit");
            assert_eq!(last_name, "Banda");
            assert_eq!(salary.compare(&expected_median_salary)?, Equal);

            let row = rows.next().await?.unwrap();

            let department_name : &str = row.get(0)?;
            let first_name : &str = row.get(1)?;
            let last_name : &str = row.get(2)?;
            let salary : Number = row.get(3)?;

            assert_eq!(department_name, "Sales");
            assert_eq!(first_name, "Charles");
            assert_eq!(last_name, "Johnson");
            assert_eq!(salary.compare(&expected_median_salary)?, Equal);

            let row = rows.next().await?;
            assert!(row.is_none());

            Ok(())
        })
    }

    #[test]
    fn ref_cursor_result() -> Result<()> {
        block_on(async {
            use std::cmp::Ordering::Equal;
            use once_cell::sync::OnceCell;

            static ORACLE : OnceCell<Environment> = OnceCell::new();
            let oracle = ORACLE.get_or_try_init(|| {
                sibyl::env()
            })?;

            let dbname = std::env::var("DBNAME").expect("database name");
            let dbuser = std::env::var("DBUSER").expect("user name");
            let dbpass = std::env::var("DBPASS").expect("password");

            let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
            let stmt = session.prepare("
                DECLARE
                    c1 SYS_REFCURSOR;
                    c2 SYS_REFCURSOR;
                BEGIN
                    OPEN c1 FOR
                        SELECT department_name, first_name, last_name, salary
                        FROM (
                                SELECT first_name, last_name, salary, department_id
                                    , ROW_NUMBER() OVER (ORDER BY salary) ord
                                FROM hr.employees
                            ) e
                        JOIN hr.departments d
                            ON d.department_id = e.department_id
                        WHERE ord = 1
                    ;
                    OPEN c2 FOR
                        SELECT department_name, first_name, last_name, salary
                        FROM (
                                SELECT first_name, last_name, salary, department_id
                                    , MEDIAN(salary) OVER () median_salary
                                FROM hr.employees
                            ) e
                        JOIN hr.departments d
                            ON d.department_id = e.department_id
                        WHERE salary = median_salary
                    ORDER BY department_name, last_name, first_name
                    ;
                    DBMS_SQL.RETURN_RESULT(c1);
                    DBMS_SQL.RETURN_RESULT(c2);
                END;
            ").await?;

            let expected_lowest_salary = Number::from_int(2100, &session)?;
            let expected_median_salary = Number::from_int(6200, &session)?;

            stmt.execute(()).await?;

            let lowest_payed_employee = stmt.next_result().await?.unwrap();

            let rows = lowest_payed_employee.rows().await?;
            let row = rows.next().await?.unwrap();

            let department_name : &str = row.get(0)?;
            let first_name : &str = row.get(1)?;
            let last_name : &str = row.get(2)?;
            let salary : Number = row.get(3)?;

            assert_eq!(department_name, "Shipping");
            assert_eq!(first_name, "TJ");
            assert_eq!(last_name, "Olson");
            assert_eq!(salary.compare(&expected_lowest_salary)?, Equal);

            let row = rows.next().await?;
            assert!(row.is_none());

            let median_salary_employees = stmt.next_result().await?.unwrap();

            let rows = median_salary_employees.rows().await?;

            let row = rows.next().await?.unwrap();
            let department_name : &str = row.get(0)?;
            let first_name : &str = row.get(1)?;
            let last_name : &str = row.get(2)?;
            let salary : Number = row.get(3)?;

            assert_eq!(department_name, "Sales");
            assert_eq!(first_name, "Amit");
            assert_eq!(last_name, "Banda");
            assert_eq!(salary.compare(&expected_median_salary)?, Equal);

            let row = rows.next().await?.unwrap();

            let department_name : &str = row.get(0)?;
            let first_name : &str = row.get(1)?;
            let last_name : &str = row.get(2)?;
            let salary : Number = row.get(3)?;

            assert_eq!(department_name, "Sales");
            assert_eq!(first_name, "Charles");
            assert_eq!(last_name, "Johnson");
            assert_eq!(salary.compare(&expected_median_salary)?, Equal);

            let row = rows.next().await?;
            assert!(row.is_none());

            assert!(stmt.next_result().await?.is_none());

            Ok(())
        })
    }

    #[test]
    fn ref_cursor_column() -> Result<()> {
        block_on(async {
            use once_cell::sync::OnceCell;

            static ORACLE : OnceCell<Environment> = OnceCell::new();
            let oracle = ORACLE.get_or_try_init(|| {
                sibyl::env()
            })?;

            let dbname = std::env::var("DBNAME").expect("database name");
            let dbuser = std::env::var("DBUSER").expect("user name");
            let dbpass = std::env::var("DBPASS").expect("password");

            let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
            let stmt = session.prepare("
                SELECT last_name
                    , CURSOR(
                            SELECT department_name
                            FROM hr.departments
                            WHERE department_id IN (
                                        SELECT department_id
                                        FROM hr.employees
                                        WHERE last_name = e.last_name)
                        ORDER BY department_name
                    ) AS departments
                FROM (
                        SELECT distinct last_name
                        FROM hr.employees
                        WHERE last_name = :last_name
                    ) e
            ").await?;
            let row = stmt.query_single("King").await?.unwrap();
            let last_name : &str = row.get(0)?;
            assert_eq!(last_name, "King");

            let departments : Cursor = row.get(1)?;
            let dept_rows = departments.rows().await?;
            let dept_row = dept_rows.next().await?.unwrap();

            let department_name : &str = dept_row.get(0)?;
            assert_eq!(department_name, "Executive");

            let dept_row = dept_rows.next().await?.unwrap();
            let department_name : &str = dept_row.get(0)?;
            assert_eq!(department_name, "Sales");

            assert!(dept_rows.next().await?.is_none());

            Ok(())
        })
    }

    #[test]
    fn large_object_datatypes() -> Result<()> {
        block_on(async {
            use once_cell::sync::OnceCell;

            static ORACLE : OnceCell<Environment> = OnceCell::new();
            let oracle = ORACLE.get_or_try_init(|| {
                sibyl::env()
            })?;

            let dbname = std::env::var("DBNAME").expect("database name");
            let dbuser = std::env::var("DBUSER").expect("user name");
            let dbpass = std::env::var("DBPASS").expect("password");

            let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
            let stmt = session.prepare("
                DECLARE
                    name_already_used EXCEPTION; PRAGMA EXCEPTION_INIT(name_already_used, -955);
                BEGIN
                    EXECUTE IMMEDIATE '
                        CREATE TABLE test_large_object_data (
                            id      NUMBER GENERATED ALWAYS AS IDENTITY,
                            bin     BLOB,
                            text    CLOB,
                            ntxt    NCLOB,
                            fbin    BFILE
                        )
                    ';
                EXCEPTION
                WHEN name_already_used THEN NULL;
                END;
            ").await?;
            stmt.execute(()).await?;

            let stmt = session.prepare("
                INSERT INTO test_large_object_data (bin, text, ntxt, fbin)
                VALUES (Empty_Blob(), Empty_Clob(), Empty_Clob(), BFileName(:DIR,:NAME))
                RETURNING id INTO :ID
            ").await?;
            let mut id = 0;
            let count = stmt.execute(((":DIR", "MEDIA_DIR"), (":NAME", "hello_world.txt"), (":ID", &mut id))).await?;
            assert_eq!(count, 1);
            assert!(id > 0);

            // session.commit().await?;

            // Content of `hello_world.txt`:
            let data = [0xfeu8, 0xff, 0x00, 0x48, 0x00, 0x65, 0x00, 0x6c, 0x00, 0x6c, 0x00, 0x6f, 0x00, 0x2c, 0x00, 0x20, 0x00, 0x57, 0x00, 0x6f, 0x00, 0x72, 0x00, 0x6c, 0x00, 0x64, 0x00, 0x21];

            // Can only read BFILEs
            let stmt = session.prepare("SELECT fbin FROM test_large_object_data WHERE id = :ID").await?;
            let row = stmt.query_single(&id).await?.expect("a row from the result set");
            let lob : BFile = row.get("FBIN")?;

            assert!(lob.file_exists().await?);
            let (dir, name) = lob.file_name()?; // if we forgot :-)
            assert_eq!(dir, "MEDIA_DIR");
            assert_eq!(name, "hello_world.txt");

            assert!(!lob.is_file_open().await?);
            lob.open_file().await?;
            let mut lob_data = Vec::new();
            lob.read(0, 28, &mut lob_data).await?;
            lob.close_file().await?;
            assert_eq!(lob_data, data);

            // Note: To modify a LOB column or attribute (write, copy, trim, and so forth), you must lock the row containing the LOB.
            // One way to do this is to use a SELECT...FOR UPDATE statement to select the locator before performing the operation.

            let stmt = session.prepare("SELECT bin FROM test_large_object_data WHERE id = :ID FOR UPDATE").await?;
            let row = stmt.query_single(&id).await?.expect("a row from the result set");
            let lob : BLOB = row.get(0)?;

            lob.open().await?;
            let count = lob.append(&data).await?;
            assert_eq!(count, 28);
            lob.close().await?;

            // session.commit().await?;

            // Read it (in another transaction)

            let stmt = session.prepare("SELECT bin FROM test_large_object_data WHERE id = :ID").await?;
            let row = stmt.query_single(&id).await?.expect("a row from the result set");
            let lob : BLOB = row.get(0)?;
            let mut lob_data = Vec::new();
            let num_read = lob.read(0, 100, &mut lob_data).await?;
            assert_eq!(num_read, 28);
            assert_eq!(lob_data, data);


            let stmt = session.prepare("SELECT text FROM test_large_object_data WHERE id = :ID FOR UPDATE").await?;
            let row = stmt.query_single(&id).await?.expect("a row from the result set");
            let lob : CLOB = row.get(0)?;
            assert!(!lob.is_nclob()?);

            let text = "Two roads diverged in a yellow wood, And sorry I could not travel both And be one traveler, long I stood And looked down one as far as I could To where it bent in the undergrowth; Then took the other, as just as fair, And having perhaps the better claim, Because it was grassy and wanted wear; Though as for that the passing there Had worn them really about the same, And both that morning equally lay In leaves no step had trodden black. Oh, I kept the first for another day! Yet knowing how way leads on to way, I doubted if I should ever come back. I shall be telling this with a sigh Somewhere ages and ages hence: Two roads diverged in a wood, and I— I took the one less traveled by, And that has made all the difference.";

            lob.open().await?;
            let count = lob.append(text).await?;
            assert_eq!(count, 726); // characters
            lob.close().await?;

            // session.commit().await?;

            // Read it (in another transaction)

            let stmt = session.prepare("SELECT text FROM test_large_object_data WHERE id = :ID").await?;
            let row = stmt.query_single(&id).await?.expect("a row from the result set");
            let lob : CLOB = row.get(0)?;
            let mut lob_text = String::new();
            let num_read = lob.read(0, 800, &mut lob_text).await?;
            assert_eq!(num_read, 726);
            assert_eq!(lob_text, text);

            Ok(())
        })
    }
}