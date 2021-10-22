use std::cmp::Ordering::Equal;
use sibyl::*;

#[test]
fn character_datatypes() -> Result<()> {
    let dbname = std::env::var("DBNAME").expect("database name");
    let dbuser = std::env::var("DBUSER").expect("schema name");
    let dbpass = std::env::var("DBPASS").expect("password");
    let oracle = env()?;
    let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    let stmt = conn.prepare("
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
          WHEN name_already_used THEN
            EXECUTE IMMEDIATE '
                TRUNCATE TABLE test_character_data
            ';
        END;
    ")?;
    stmt.execute(&[])?;

    let mut ids = Vec::with_capacity(3);
    let stmt = conn.prepare("
        INSERT INTO test_character_data (text, ntext) VALUES (:TEXT, '> ' || :TEXT)
        RETURNING id, text, ntext INTO :ID, :TEXT_OUT, :NTXT_OUT
    ")?;
    let mut id = 0;
    let mut text_out = String::with_capacity(97);
    let mut ntxt_out = String::with_capacity(99);
    let count = stmt.execute_into(
        &[
            &(":TEXT", "Two roads diverged in a yellow wood,")
        ], &mut [
            &mut (":ID", &mut id),
            &mut (":TEXT_OUT", &mut text_out),
            &mut (":NTXT_OUT", &mut ntxt_out)
        ]
    )?;
    assert_eq!(count, 1);
    assert_eq!(text_out, "Two roads diverged in a yellow wood,");
    assert_eq!(ntxt_out, "> Two roads diverged in a yellow wood,");
    assert!(id > 0);
    ids.push(id);

    let text = String::from("And sorry I could not travel both");
    let count = stmt.execute_into(
        &[
            &(":TEXT", text.as_str())
        ], &mut[
            &mut (":ID", &mut id),
            &mut (":TEXT_OUT", &mut text_out),
            &mut (":NTXT_OUT", &mut ntxt_out)
        ]
    )?;
    assert_eq!(count, 1);
    assert_eq!(text_out, "And sorry I could not travel both");
    assert_eq!(ntxt_out, "> And sorry I could not travel both");
    assert!(id > 0);
    ids.push(id);

    let mut text_out = Varchar::with_capacity(97, &conn)?;
    let mut ntxt_out = Varchar::with_capacity(99, &conn)?;
    let text = Varchar::from("And be one traveler, long I stood", &conn)?;
    let count = stmt.execute_into(
        &[
            &(":TEXT", text.as_str())
        ], &mut [
            &mut (":ID", &mut id),
            &mut (":TEXT_OUT", &mut text_out),
            &mut (":NTXT_OUT", &mut ntxt_out)
        ]
    )?;
    assert_eq!(count, 1);
    assert_eq!(text_out.as_str(), "And be one traveler, long I stood");
    assert_eq!(ntxt_out.as_str(), "> And be one traveler, long I stood");
    assert!(id > 0);
    ids.push(id);

    let stmt = conn.prepare("SELECT text, ntext FROM test_character_data WHERE id = :ID")?;

    let mut rows = stmt.query(&[ &(":ID", ids[0]) ])?;
    let row  = rows.next()?.unwrap();
    let text : &str = row.get("TEXT")?.unwrap();
    assert_eq!(text, "Two roads diverged in a yellow wood,");
    let text : &str = row.get("NTEXT")?.unwrap();
    assert_eq!(text, "> Two roads diverged in a yellow wood,");
    assert!(rows.next()?.is_none());

    let mut rows = stmt.query(&[ &(":ID", ids[1]) ])?;
    let row  = rows.next()?.unwrap();
    let text : String = row.get(0)?.unwrap();
    assert_eq!(text.as_str(), "And sorry I could not travel both");
    let text : String = row.get(1)?.unwrap();
    assert_eq!(text.as_str(), "> And sorry I could not travel both");
    assert!(rows.next()?.is_none());

    let mut rows = stmt.query(&[ &(":ID", ids[2]) ])?;
    let row  = rows.next()?.unwrap();
    let text : Varchar = row.get("TEXT")?.unwrap();
    assert_eq!(text.as_str(), "And be one traveler, long I stood");
    let text : Varchar = row.get("NTEXT")?.unwrap();
    assert_eq!(text.as_str(), "> And be one traveler, long I stood");
    assert!(rows.next()?.is_none());

    Ok(())
}

#[test]
fn datetime_datatypes() -> Result<()> {
    let dbname = std::env::var("DBNAME").expect("database name");
    let dbuser = std::env::var("DBUSER").expect("schema name");
    let dbpass = std::env::var("DBPASS").expect("password");
    let oracle = env()?;
    let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    let stmt = conn.prepare("
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
          WHEN name_already_used THEN
            EXECUTE IMMEDIATE '
                TRUNCATE TABLE test_datetime_data
            ';
        END;
    ")?;
    stmt.execute(&[])?;

    let stmt = conn.prepare("
        INSERT INTO test_datetime_data (dt, ts, tsz, tsl, iym, ids) VALUES (:DT, :TS, :TSZ, :TSL, :IYM, :IDS)
        RETURNING id, dt, ts, tsz, tsl, iym, ids INTO :ID, :ODT, :OTS, :OTSZ, :OTSL, :OIYM, :OIDS
    ")?;
    let mut id = 0;

    let dt  = Date::with_datetime(1969, 7, 24, 16, 50, 35, &conn)?;
    let ts  = Timestamp::with_datetime(1969, 7, 24, 16, 50, 35, 1, "", &conn)?;
    let tsz = TimestampTZ::with_datetime(1969, 7, 24, 16, 50, 35, 2, "UTC", &conn)?;
    let tsl = TimestampLTZ::with_datetime(1969, 7, 24, 16, 50, 35, 3, "UTC", &conn)?;
    let iym = IntervalYM::with_duration(123, 11, &conn)?;
    let ids = IntervalDS::with_duration(256, 16, 15, 37, 123456789, &conn)?;

    let mut dt_out  = Date::new(&conn);
    let mut ts_out  = Timestamp::new(&conn)?;
    let mut tsz_out = TimestampTZ::new(&conn)?;
    let mut tsl_out = TimestampLTZ::new(&conn)?;
    let mut iym_out = IntervalYM::new(&conn)?;
    let mut ids_out = IntervalDS::new(&conn)?;

    let count = stmt.execute_into(
        &[
            &(":DT",  &dt),
            &(":TS",  &ts),
            &(":TSZ", &tsz),
            &(":TSL", &tsl),
            &(":IYM", &iym),
            &(":IDS", &ids)
        ], &mut [
            &mut (":ID",   &mut id),
            &mut (":ODT",  &mut dt_out),
            &mut (":OTS",  &mut ts_out),
            &mut (":OTSZ", &mut tsz_out),
            &mut (":OTSL", &mut tsl_out),
            &mut (":OIYM", &mut iym_out),
            &mut (":OIDS", &mut ids_out)
        ]
    )?;
    assert_eq!(count, 1);
    assert!(id > 0);
    assert_eq!(dt_out.compare(&dt)?, Equal);
    assert_eq!(ts_out.compare(&ts)?, Equal);
    assert_eq!(tsz_out.compare(&tsz)?, Equal);
    assert_eq!(tsl_out.compare(&tsl)?, Equal);
    assert_eq!(iym_out.compare(&iym)?, Equal);
    assert_eq!(ids_out.compare(&ids)?, Equal);

    let count = stmt.execute_into(
        &[
            &(":DT",  dt),
            &(":TS",  ts),
            &(":TSZ", tsz),
            &(":TSL", tsl),
            &(":IYM", iym),
            &(":IDS", ids)
        ],
        &mut [
            &mut (":ID",   &mut id),
            &mut (":ODT",  &mut dt_out),
            &mut (":OTS",  &mut ts_out),
            &mut (":OTSZ", &mut tsz_out),
            &mut (":OTSL", &mut tsl_out),
            &mut (":OIYM", &mut iym_out),
            &mut (":OIDS", &mut ids_out)
        ]
    )?;
    assert_eq!(count, 1);
    assert!(id > 0);

    // IN arguments have just been moved. Re-create them for comparisons:
    let dt2  = Date::with_datetime(1969, 7, 24, 16, 50, 35, &conn)?;
    let ts2  = Timestamp::with_datetime(1969, 7, 24, 16, 50, 35, 1, "", &conn)?;
    let tsz2 = TimestampTZ::with_datetime(1969, 7, 24, 16, 50, 35, 2, "UTC", &conn)?;
    let tsl2 = TimestampLTZ::with_datetime(1969, 7, 24, 16, 50, 35, 3, "UTC", &conn)?;
    let iym2 = IntervalYM::with_duration(123, 11, &conn)?;
    let ids2 = IntervalDS::with_duration(256, 16, 15, 37, 123456789, &conn)?;

    assert_eq!(dt_out.compare(&dt2)?, Equal);
    assert_eq!(ts_out.compare(&ts2)?, Equal);
    assert_eq!(tsz_out.compare(&tsz2)?, Equal);
    assert_eq!(tsl_out.compare(&tsl2)?, Equal);
    assert_eq!(iym_out.compare(&iym2)?, Equal);
    assert_eq!(ids_out.compare(&ids2)?, Equal);


    let stmt = conn.prepare("SELECT dt, ts, tsz, tsl, iym, ids FROM test_datetime_data WHERE id = :ID")?;
    let mut rows = stmt.query(&[ &(":ID", id) ])?;
    let row  = rows.next()?.unwrap();
    let val : Date = row.get("DT")?.unwrap();
    assert_eq!(val.compare(&dt2)?, Equal);
    let val : Timestamp = row.get("TS")?.unwrap();
    assert_eq!(val.compare(&ts2)?, Equal);
    let val : TimestampTZ = row.get("TSZ")?.unwrap();
    assert_eq!(val.compare(&tsz2)?, Equal);
    let val : TimestampLTZ = row.get("TSL")?.unwrap();
    assert_eq!(val.compare(&tsl2)?, Equal);
    let val : IntervalYM = row.get("IYM")?.unwrap();
    assert_eq!(val.compare(&iym2)?, Equal);
    let val : IntervalDS = row.get("IDS")?.unwrap();
    assert_eq!(val.compare(&ids2)?, Equal);

    assert!(rows.next()?.is_none());

    Ok(())
}

#[test]
fn large_object_datatypes() -> Result<()> {
    let dbname = std::env::var("DBNAME").expect("database name");
    let dbuser = std::env::var("DBUSER").expect("schema name");
    let dbpass = std::env::var("DBPASS").expect("password");
    let oracle = env()?;
    let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    let stmt = conn.prepare("
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
          WHEN name_already_used THEN
            EXECUTE IMMEDIATE '
                TRUNCATE TABLE test_large_object_data
            ';
        END;
    ")?;
    stmt.execute(&[])?;

    let stmt = conn.prepare("
        INSERT INTO test_large_object_data (bin, text, ntxt, fbin)
        VALUES (Empty_Blob(), Empty_Clob(), Empty_Clob(), BFileName(:DIR,:NAME))
        RETURNING id INTO :ID
    ")?;
    let mut id = 0;
    let count = stmt.execute_into(
        &[
            &(":DIR", "MEDIA_DIR"),
            &(":NAME", "hello_world.txt")
        ], &mut [
            &mut (":ID", &mut id)
        ]
    )?;
    assert_eq!(count, 1);
    assert!(id > 0);

    // Content of `hello_world.txt`:
    let data = [0xfeu8, 0xff, 0x00, 0x48, 0x00, 0x65, 0x00, 0x6c, 0x00, 0x6c, 0x00, 0x6f, 0x00, 0x2c, 0x00, 0x20, 0x00, 0x57, 0x00, 0x6f, 0x00, 0x72, 0x00, 0x6c, 0x00, 0x64, 0x00, 0x21];

    // Can only read BFILEs
    let stmt = conn.prepare("SELECT fbin FROM test_large_object_data WHERE id = :ID")?;
    let mut rows = stmt.query(&[ &(":ID", &id) ])?;
    let row  = rows.next()?.expect("a row from the result set");
    let lob : BFile = row.get("FBIN")?.expect("BFILE locator");

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

    let stmt = conn.prepare("SELECT bin FROM test_large_object_data WHERE id = :ID FOR UPDATE")?;
    let mut rows = stmt.query(&[ &(":ID", &id) ])?;
    let row  = rows.next()?.expect("a row from the result set");
    let lob : BLOB = row.get(0)?.expect("BLOB locator");

    lob.open()?;
    let count = lob.append(&data)?;
    assert_eq!(count, 28);
    lob.close()?;

    // Read it (in another transaction)

    let stmt = conn.prepare("SELECT bin FROM test_large_object_data WHERE id = :ID")?;
    let mut rows = stmt.query(&[ &(":ID", &id) ])?;
    let row  = rows.next()?.expect("a row from the result set");
    let lob : BLOB = row.get(0)?.expect("BLOB locator");
    let mut lob_data = Vec::new();
    lob.read(0, 28, &mut lob_data)?;
    assert_eq!(lob_data, data);


    let stmt = conn.prepare("SELECT text FROM test_large_object_data WHERE id = :ID FOR UPDATE")?;
    let mut rows = stmt.query(&[ &(":ID", &id) ])?;
    let row  = rows.next()?.expect("a row from the result set");
    let lob : CLOB = row.get(0)?.expect("BLOB locator");
    assert!(!lob.is_nclob()?);

    let text = "Two roads diverged in a yellow wood, And sorry I could not travel both And be one traveler, long I stood And looked down one as far as I could To where it bent in the undergrowth; Then took the other, as just as fair, And having perhaps the better claim, Because it was grassy and wanted wear; Though as for that the passing there Had worn them really about the same, And both that morning equally lay In leaves no step had trodden black. Oh, I kept the first for another day! Yet knowing how way leads on to way, I doubted if I should ever come back. I shall be telling this with a sigh Somewhere ages and ages hence: Two roads diverged in a wood, and I— I took the one less traveled by, And that has made all the difference.";

    lob.open()?;
    let count = lob.append(text)?;
    assert_eq!(count, 728); // bytes
    assert_eq!(lob.len()?, 726); // characters
    lob.close()?;

    let stmt = conn.prepare("SELECT text FROM test_large_object_data WHERE id = :ID")?;
    let mut rows = stmt.query(&[ &(":ID", &id) ])?;
    let row  = rows.next()?.expect("a row from the result set");
    let lob : CLOB = row.get(0)?.expect("CLOB locator");
    assert!(!lob.is_nclob()?);
    let mut lob_text = String::new();
    lob.read(0, 726, &mut lob_text)?;
    assert_eq!(lob_text, text);


    let stmt = conn.prepare("SELECT ntxt FROM test_large_object_data WHERE id = :ID FOR UPDATE")?;
    let mut rows = stmt.query(&[ &(":ID", &id) ])?;
    let row  = rows.next()?.expect("a row from the result set");
    let lob : CLOB = row.get(0)?.expect("CLOB locator");
    assert!(lob.is_nclob()?);

    lob.open()?;
    let count = lob.append(text)?;
    assert_eq!(count, 728); // bytes
    assert_eq!(lob.len()?, 726); // characters
    lob.close()?;

    let stmt = conn.prepare("SELECT ntxt FROM test_large_object_data WHERE id = :ID")?;
    let mut rows = stmt.query(&[ &(":ID", &id) ])?;
    let row  = rows.next()?.expect("a row from the result set");
    let lob : CLOB = row.get(0)?.expect("CLOB locator");
    assert!(lob.is_nclob()?);
    let mut lob_text = String::new();
    lob.read(0, 726, &mut lob_text)?;
    assert_eq!(lob_text, text);

    Ok(())
}

#[test]
fn long_and_raw_datatypes() -> Result<()> {
    let dbname = std::env::var("DBNAME").expect("database name");
    let dbuser = std::env::var("DBUSER").expect("schema name");
    let dbpass = std::env::var("DBPASS").expect("password");
    let oracle = env()?;
    let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    let stmt = conn.prepare("
        DECLARE
            name_already_used EXCEPTION; PRAGMA EXCEPTION_INIT(name_already_used, -955);
        BEGIN
            EXECUTE IMMEDIATE '
                CREATE TABLE test_long_and_raw_data (
                    id      NUMBER GENERATED ALWAYS AS IDENTITY,
                    bin     RAW(100),
                    text    LONG
                )
            ';
        EXCEPTION
          WHEN name_already_used THEN
            EXECUTE IMMEDIATE '
                TRUNCATE TABLE test_long_and_raw_data
            ';
        END;
    ")?;
    stmt.execute(&[])?;

    // Cannot return LONG
    let stmt = conn.prepare("
        INSERT INTO test_long_and_raw_data (bin, text) VALUES (:BIN, :TEXT)
        RETURNING id, bin INTO :ID, :OBIN
    ")?;
    let data = [0xfeu8, 0xff, 0x00, 0x48, 0x00, 0x65, 0x00, 0x6c, 0x00, 0x6c, 0x00, 0x6f, 0x00, 0x2c, 0x00, 0x20, 0x00, 0x57, 0x00, 0x6f, 0x00, 0x72, 0x00, 0x6c, 0x00, 0x64, 0x00, 0x21];
    let text = "When I have fears that I may cease to be Before my pen has gleaned my teeming brain, Before high-pilèd books, in charactery, Hold like rich garners the full ripened grain; When I behold, upon the night’s starred face, Huge cloudy symbols of a high romance, And think that I may never live to trace Their shadows with the magic hand of chance; And when I feel, fair creature of an hour, That I shall never look upon thee more, Never have relish in the faery power Of unreflecting love—then on the shore Of the wide world I stand alone, and think Till love and fame to nothingness do sink.";
    let mut id = 0;
    let mut data_out = Vec::with_capacity(30);
    let count = stmt.execute_into(
        &[
            &(":BIN", &data[..]),
            &(":TEXT", text)
        ], &mut [
            &mut (":ID", &mut id),
            &mut (":OBIN", &mut data_out)
        ]
    )?;
    assert_eq!(count, 1);
    assert!(id > 0);
    assert_eq!(data_out.as_slice(), &data[..]);

    let stmt = conn.prepare("SELECT bin, text FROM test_long_and_raw_data WHERE id = :ID")?;
    // without explicit resizing via `stmt.set_column_size` (before `stmt.query`) TEXT output is limited to 32768
    let mut rows = stmt.query(&[ &(":ID", &id) ])?;
    let row  = rows.next()?.unwrap();
    let bin : &[u8] = row.get("BIN")?.unwrap();
    let txt : &str = row.get("TEXT")?.unwrap();
    assert_eq!(bin, &data[..]);
    assert_eq!(txt, text);

    Ok(())
}

#[test]
fn long_raw_datatype() -> Result<()> {
    let dbname = std::env::var("DBNAME").expect("database name");
    let dbuser = std::env::var("DBUSER").expect("schema name");
    let dbpass = std::env::var("DBPASS").expect("password");
    let oracle = env()?;
    let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    let stmt = conn.prepare("
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
          WHEN name_already_used THEN
            EXECUTE IMMEDIATE '
                TRUNCATE TABLE test_long_raw_data
            ';
        END;
    ")?;
    stmt.execute(&[])?;

    let stmt = conn.prepare("
        INSERT INTO test_long_raw_data (bin) VALUES (:BIN)
        RETURNING id INTO :ID
    ")?;
    let data = [0xfeu8, 0xff, 0x00, 0x48, 0x00, 0x65, 0x00, 0x6c, 0x00, 0x6c, 0x00, 0x6f, 0x00, 0x2c, 0x00, 0x20, 0x00, 0x57, 0x00, 0x6f, 0x00, 0x72, 0x00, 0x6c, 0x00, 0x64, 0x00, 0x21];
    let mut id = 0;
    let count = stmt.execute_into(
        &[
            &(":BIN", &data[..])
        ], &mut [
            &mut (":ID", &mut id)
        ]
    )?;
    assert_eq!(count, 1);
    assert!(id > 0);

    let stmt = conn.prepare("SELECT bin FROM test_long_raw_data WHERE id = :ID")?;
    // without explicit resizing via `stmt.set_column_size` (before `stmt.query`) BIN output is limited to 32768
    let mut rows = stmt.query(&[ &(":ID", &id) ])?;
    let row  = rows.next()?.unwrap();
    let bin : &[u8] = row.get(0)?.unwrap();
    assert_eq!(bin, &data[..]);

    Ok(())
}

#[test]
fn numeric_datatypes() -> Result<()> {
    let dbname = std::env::var("DBNAME").expect("database name");
    let dbuser = std::env::var("DBUSER").expect("schema name");
    let dbpass = std::env::var("DBPASS").expect("password");
    let oracle = env()?;
    let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    let stmt = conn.prepare("
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
          WHEN name_already_used THEN
            EXECUTE IMMEDIATE '
                TRUNCATE TABLE test_numeric_data
            ';
        END;
    ")?;
    stmt.execute(&[])?;

    let stmt = conn.prepare("
        INSERT INTO test_numeric_data (num, flt, dbl) VALUES (:NUM, :NUM, :NUM)
        RETURNING id, num, flt, dbl INTO :ID, :ONUM, :OFLT, :ODBL
    ")?;
    let src_num = Number::from_string("3.141592653589793238462643383279502884197", "9.999999999999999999999999999999999999999", &conn)?;
    let mut id = 0;
    let mut num = Number::new(&conn);
    let mut flt = 0f32;
    let mut dbl = 0f64;
    let count = stmt.execute_into(
        &[
            &(":NUM", &src_num)
        ], &mut [
            &mut (":ID", &mut id),
            &mut (":ONUM", &mut num),
            &mut (":OFLT", &mut flt),
            &mut (":ODBL", &mut dbl)
        ]
    )?;
    assert_eq!(count, 1);
    assert!(id > 0);
    assert_eq!(num.compare(&src_num)?, Equal);
    assert!(3.141592653589792 < dbl && dbl < 3.141592653589794);
    assert!(3.1415926 < flt && flt < 3.1415929);

    let stmt = conn.prepare("SELECT num, flt, dbl FROM test_numeric_data WHERE id = :ID")?;
    let mut rows = stmt.query(&[ &(":ID", &id) ])?;
    let row  = rows.next()?.unwrap();
    let num : Number = row.get("NUM")?.expect("test_numeric_data.num");
    let flt : f32 = row.get("FLT")?.expect("test_numeric_data.flt");
    let dbl : f64 = row.get("DBL")?.expect("test_numeric_data.dbl");
    assert_eq!(num.compare(&src_num)?, Equal);
    assert!(3.141592653589792 < dbl && dbl < 3.141592653589794);
    assert!(3.1415926 < flt && flt < 3.1415929);
    assert_eq!(num.to_string("TM")?, "3.1415926535897932384626433832795028842");

    Ok(())
}

#[test]
fn rowid_datatype() -> Result<()> {
    let dbname = std::env::var("DBNAME").expect("database name");
    let dbuser = std::env::var("DBUSER").expect("schema name");
    let dbpass = std::env::var("DBPASS").expect("password");
    let oracle = env()?;
    let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    let stmt = conn.prepare("
        SELECT ROWID, manager_id
          FROM hr.employees
         WHERE employee_id = :ID
           FOR UPDATE
    ")?;
    let mut rows = stmt.query(&[ &(":ID", 107) ])?;
    let row = rows.next()?.expect("selected row");
    let implicit_rowid = row.get_rowid()?;
    let str_rowid : String = row.get(0)?.expect("ROWID as text");
    assert_eq!(str_rowid, implicit_rowid.to_string(&conn)?);
    let explicit_rowid : RowID = row.get(0)?.expect("ROWID pseudo-column");
    assert_eq!(explicit_rowid.to_string(&conn)?, implicit_rowid.to_string(&conn)?);
    let manager_id: u32 = row.get(1)?.expect("menager ID");
    assert_eq!(manager_id, 102);

    let stmt = conn.prepare("
        UPDATE hr.employees
           SET manager_id = :MID
         WHERE rowid = :RID
    ")?;
    let num_updated = stmt.execute(&[
        &( ":MID", 102 ),
        &( ":RID", &implicit_rowid ),
    ])?;
    assert_eq!(num_updated, 1);
    conn.rollback()?;
    Ok(())
}

#[test]
fn ref_cursor() -> Result<()> {
    let dbname = std::env::var("DBNAME").expect("database name");
    let dbuser = std::env::var("DBUSER").expect("schema name");
    let dbpass = std::env::var("DBPASS").expect("password");
    let oracle = env()?;
    let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    let stmt = conn.prepare("
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

    stmt.execute_into(&[], &mut [
        &mut ( ":LOWEST_PAYED_EMPLOYEE",   &mut lowest_payed_employee   ),
        &mut ( ":MEDIAN_SALARY_EMPLOYEES", &mut median_salary_employees ),
    ])?;

    let expected_lowest_salary = Number::from_int(2100, &conn)?;
    let expected_median_salary = Number::from_int(6200, &conn)?;

    let mut rows = lowest_payed_employee.rows()?;
    let row = rows.next()?.unwrap();

    let department_name : &str = row.get(0)?.unwrap();
    let first_name : &str = row.get(1)?.unwrap();
    let last_name : &str = row.get(2)?.unwrap();
    let salary : Number = row.get(3)?.unwrap();

    assert_eq!(department_name, "Shipping");
    assert_eq!(first_name, "TJ");
    assert_eq!(last_name, "Olson");
    assert_eq!(salary.compare(&expected_lowest_salary)?, Equal);

    let row = rows.next()?;
    assert!(row.is_none());

    let mut rows = median_salary_employees.rows()?;

    let row = rows.next()?.unwrap();
    let department_name : &str = row.get(0)?.unwrap();
    let first_name : &str = row.get(1)?.unwrap();
    let last_name : &str = row.get(2)?.unwrap();
    let salary : Number = row.get(3)?.unwrap();

    assert_eq!(department_name, "Sales");
    assert_eq!(first_name, "Amit");
    assert_eq!(last_name, "Banda");
    assert_eq!(salary.compare(&expected_median_salary)?, Equal);

    let row = rows.next()?.unwrap();

    let department_name : &str = row.get(0)?.unwrap();
    let first_name : &str = row.get(1)?.unwrap();
    let last_name : &str = row.get(2)?.unwrap();
    let salary : Number = row.get(3)?.unwrap();

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
    let dbname = std::env::var("DBNAME").expect("database name");
    let dbuser = std::env::var("DBUSER").expect("schema name");
    let dbpass = std::env::var("DBPASS").expect("password");
    let oracle = env()?;
    let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    let stmt = conn.prepare("
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

    let expected_lowest_salary = Number::from_int(2100, &conn)?;
    let expected_median_salary = Number::from_int(6200, &conn)?;

    stmt.execute(&[])?;

    let lowest_payed_employee = stmt.next_result()?.unwrap();

    let mut rows = lowest_payed_employee.rows()?;
    let row = rows.next()?.unwrap();

    let department_name : &str = row.get(0)?.unwrap();
    let first_name : &str = row.get(1)?.unwrap();
    let last_name : &str = row.get(2)?.unwrap();
    let salary : Number = row.get(3)?.unwrap();

    assert_eq!(department_name, "Shipping");
    assert_eq!(first_name, "TJ");
    assert_eq!(last_name, "Olson");
    assert_eq!(salary.compare(&expected_lowest_salary)?, Equal);

    let row = rows.next()?;
    assert!(row.is_none());

    let median_salary_employees = stmt.next_result()?.unwrap();

    let mut rows = median_salary_employees.rows()?;

    let row = rows.next()?.unwrap();
    let department_name : &str = row.get(0)?.unwrap();
    let first_name : &str = row.get(1)?.unwrap();
    let last_name : &str = row.get(2)?.unwrap();
    let salary : Number = row.get(3)?.unwrap();

    assert_eq!(department_name, "Sales");
    assert_eq!(first_name, "Amit");
    assert_eq!(last_name, "Banda");
    assert_eq!(salary.compare(&expected_median_salary)?, Equal);

    let row = rows.next()?.unwrap();

    let department_name : &str = row.get(0)?.unwrap();
    let first_name : &str = row.get(1)?.unwrap();
    let last_name : &str = row.get(2)?.unwrap();
    let salary : Number = row.get(3)?.unwrap();

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
    let dbuser = std::env::var("DBUSER").expect("schema name");
    let dbpass = std::env::var("DBPASS").expect("password");
    let oracle = env()?;
    let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;

    let stmt = conn.prepare("
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
    let mut rows = stmt.query(&[ &"King" ])?;

    let row = rows.next()?.unwrap();
    let last_name : &str = row.get(0)?.unwrap();
    assert_eq!(last_name, "King");

    let departments : Cursor = row.get(1)?.unwrap();
    let mut dept_rows = departments.rows()?;
    let dept_row = dept_rows.next()?.unwrap();

    let department_name : &str = dept_row.get(0)?.unwrap();
    assert_eq!(department_name, "Executive");

    let dept_row = dept_rows.next()?.unwrap();
    let department_name : &str = dept_row.get(0)?.unwrap();
    assert_eq!(department_name, "Sales");

    assert!(dept_rows.next()?.is_none());
    assert!(rows.next()?.is_none());

    Ok(())
}
