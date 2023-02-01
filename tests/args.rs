#[cfg(feature="blocking")]
mod tests {
    use sibyl::*;

    #[test]
    fn dup_args() -> Result<()> {
        let session = sibyl::test_env::get_session()?;

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
        let session = sibyl::test_env::get_session()?;

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
        let session = sibyl::test_env::get_session()?;

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
}