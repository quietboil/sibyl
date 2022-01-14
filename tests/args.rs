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
        let rows = stmt.query(3333)?;
        let row = rows.next()?;
        assert!(row.is_some());
        let row = row.unwrap();
        let state_province : &str = row.get_not_null(0)?;
        assert_eq!(state_province, "N/A");
        let city : &str = row.get_not_null(1)?;
        assert_eq!(city, "N/A");
        let postal_code : &str = row.get_not_null(2)?;
        assert_eq!(postal_code, "00000");
        let street_address : &str = row.get_not_null(3)?;
        assert_eq!(street_address, "N/A");

        Ok(())
    }
}