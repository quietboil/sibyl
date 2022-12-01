#[cfg(feature="blocking")]
mod tests {
    use sibyl::*;

    #[test]
    fn dup_args() -> Result<()> {
        let oracle = env()?;
        let dbname = std::env::var("DBNAME").expect("database name");
        let dbuser = std::env::var("DBUSER").expect("user name");
        let dbpass = std::env::var("DBPASS").expect("password");
        let session = oracle.connect(&dbname, &dbuser, &dbpass)?;

        let stmt = session.prepare("
            INSERT INTO hr.locations (location_id, state_province, city, postal_code, street_address)
            VALUES (:id, :na, :na, :code, :na)
        ")?;
        let num_rows = stmt.execute((3333, "N/A", (), "00000", ()))?;
        assert_eq!(num_rows, 1);

        let stmt = session.prepare("
            SELECT state_province, city, postal_code, street_address
              FROM hr.locations
             WHERE location_id = :id
        ")?;
        let row = stmt.query_single(3333)?;
        assert!(row.is_some());
        let row = row.unwrap();
        let state_province : &str = row.get(0)?;
        assert_eq!(state_province, "N/A");
        let city : &str = row.get(1)?;
        assert_eq!(city, "N/A");
        let postal_code : &str = row.get(2)?;
        assert_eq!(postal_code, "00000");
        let street_address : &str = row.get(3)?;
        assert_eq!(street_address, "N/A");

        Ok(())
    }

    #[test]
    fn num_args() -> Result<()> {
        let oracle = env()?;
        let dbname = std::env::var("DBNAME").expect("database name");
        let dbuser = std::env::var("DBUSER").expect("user name");
        let dbpass = std::env::var("DBPASS").expect("password");
        let session = oracle.connect(&dbname, &dbuser, &dbpass)?;

        let stmt = session.prepare("
            INSERT INTO hr.locations (location_id, state_province, city, postal_code, street_address)
            VALUES (:1, :2, :2, :3, :2)
        ")?;
        let num_rows = stmt.execute((3333, "N/A", (), "00000", ()))?;
        assert_eq!(num_rows, 1);

        let stmt = session.prepare("
            SELECT state_province, city, postal_code, street_address
              FROM hr.locations
             WHERE location_id = :id
        ")?;
        let row = stmt.query_single(3333)?;
        assert!(row.is_some());
        let row = row.unwrap();
        let state_province : &str = row.get(0)?;
        assert_eq!(state_province, "N/A");
        let city : &str = row.get(1)?;
        assert_eq!(city, "N/A");
        let postal_code : &str = row.get(2)?;
        assert_eq!(postal_code, "00000");
        let street_address : &str = row.get(3)?;
        assert_eq!(street_address, "N/A");

        Ok(())
    }

    #[test]
    fn slices() -> std::result::Result<(),Box<dyn std::error::Error>> {
        let oracle = env()?;
        let dbname = std::env::var("DBNAME").expect("database name");
        let dbuser = std::env::var("DBUSER").expect("user name");
        let dbpass = std::env::var("DBPASS").expect("password");
        let session = oracle.connect(&dbname, &dbuser, &dbpass)?;

        let stmt = session.prepare("
            SELECT location_id, state_province, city, postal_code, street_address
              FROM hr.locations
             WHERE country_id = :country
               AND (location_id = :id OR location_id = :id2)
               AND postal_code IN (:code, :code2)
             ORDER BY location_id
        ")?;
        let rows = stmt.query((
            ("COUNTRY", "UK"),
            ("ID", [2500, 2600].as_ref()),
            ("CODE", ["OX9 9ZB", "09629850293"].as_ref())
        ))?;
        while let Some(row) = rows.next()? {
            let location_id    : i32  = row.get(0)?;
            let state_province : &str = row.get(1)?;
            let city           : &str = row.get(2)?;
            let postal_code    : &str = row.get(3)?;
            let street_address : &str = row.get(4)?;
            match location_id {
                2500 => {
                    assert_eq!(state_province, "Oxford");
                    assert_eq!(city, "Oxford");
                    assert_eq!(postal_code, "OX9 9ZB");
                    assert_eq!(street_address, "Magdalen Centre, The Oxford Science Park");
                },
                2600 => {
                    assert_eq!(state_province, "Manchester");
                    assert_eq!(city, "Stretford");
                    assert_eq!(postal_code, "09629850293");
                    assert_eq!(street_address, "9702 Chester Road");
                },
                _ => {
                    panic!("unexpected location");
                }
            }
        }
        let num_rows = stmt.row_count()?;
        assert_eq!(num_rows, 2);

        Ok(())
    }

    #[test]
    fn option_num() -> std::result::Result<(),Box<dyn std::error::Error>> {
        let dbname = std::env::var("DBNAME")?;
        let dbuser = std::env::var("DBUSER")?;
        let dbpass = std::env::var("DBPASS")?;
        let oracle = Environment::new()?;
        let session = oracle.connect(&dbname, &dbuser, &dbpass)?;

        let stmt = session.prepare("SELECT Nvl(:val,42) FROM dual")?;
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

        let arg = Some(99i32);
        let row = stmt.query_single(arg)?.unwrap();
        let val : i32 = row.get(0)?;
        assert_eq!(val, 99);

        let arg = Some(99i32);
        let row = stmt.query_single(&arg)?.unwrap();
        let val : i32 = row.get(0)?;
        assert_eq!(val, 99);

        let num : i32 = 99;
        let arg = Some(&num);
        let row = stmt.query_single(arg)?.unwrap();
        let val : i32 = row.get(0)?;
        assert_eq!(val, 99);

        let arg = Some(&num);
        let row = stmt.query_single(&arg)?.unwrap();
        let val : i32 = row.get(0)?;
        assert_eq!(val, 99);

        Ok(())
    }

    #[test]
    fn option_str() -> std::result::Result<(),Box<dyn std::error::Error>> {
        let dbname = std::env::var("DBNAME")?;
        let dbuser = std::env::var("DBUSER")?;
        let dbpass = std::env::var("DBPASS")?;
        let oracle = Environment::new()?;
        let session = oracle.connect(&dbname, &dbuser, &dbpass)?;

        let stmt = session.prepare("SELECT Nvl(:val,'None') FROM dual")?;
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

        Ok(())
    }


    #[test]
    fn option_string() -> std::result::Result<(),Box<dyn std::error::Error>> {
        let dbname = std::env::var("DBNAME")?;
        let dbuser = std::env::var("DBUSER")?;
        let dbpass = std::env::var("DBPASS")?;
        let oracle = Environment::new()?;
        let session = oracle.connect(&dbname, &dbuser, &dbpass)?;

        let stmt = session.prepare("SELECT Nvl(:val,'None') FROM dual")?;
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
        let arg : Option<&String> = Some(&txt);
        let row = stmt.query_single(arg)?.unwrap();
        let val : &str = row.get(0)?;
        assert_eq!(val, "Text");

        let arg : Option<&String> = Some(&txt);
        let row = stmt.query_single(&arg)?.unwrap();
        let val : &str = row.get(0)?;
        assert_eq!(val, "Text");

        Ok(())
    }
}