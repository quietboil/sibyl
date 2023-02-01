#[cfg(feature="blocking")]
mod tests {
    use sibyl::*;
    use std::cmp::Ordering::Equal;

    #[test]
    fn monotype() -> Result<()> {
        let session = sibyl::test_env::get_session()?;

        let stmt = session.prepare("
            SELECT first_name, last_name, hire_date
              FROM hr.employees
             WHERE hire_date BETWEEN :from_date AND :thru_date
          ORDER BY hire_date
        ")?;
        let date_from = Date::from_string("September  1, 2006", "MONTH DD, YYYY", &session)?;
        let date_thru = Date::from_string("September 30, 2006", "MONTH DD, YYYY", &session)?;

        let rows = stmt.query([date_from, date_thru].as_slice())?;
        let row = rows.next()?.unwrap();
        let first_name: &str = row.get(0)?;
        let last_name:  &str = row.get(1)?;
        let hire_date:  Date = row.get(2)?;

        assert_eq!(first_name, "Irene");
        assert_eq!(last_name,  "Mikkilineni");

        let expected_hire_date = Date::from_string("September 28, 2006", "MONTH DD, YYYY", &session)?;
        assert_eq!(hire_date.compare(&expected_hire_date)?, Equal);

        Ok(())
    }

    #[test]
    fn named_monotype() -> Result<()> {
        let session = sibyl::test_env::get_session()?;

        let stmt = session.prepare("
            SELECT first_name, last_name, hire_date
              FROM hr.employees
             WHERE hire_date BETWEEN :hire_range AND :range_end
          ORDER BY hire_date
        ")?;
        let date_from = Date::from_string("September  1, 2006", "MONTH DD, YYYY", &session)?;
        let date_thru = Date::from_string("September 30, 2006", "MONTH DD, YYYY", &session)?;

        let rows = stmt.query(
            (":HIRE_RANGE", [date_from, date_thru].as_slice())
        )?;
        let row = rows.next()?.unwrap();
        let first_name: &str = row.get(0)?;
        let last_name:  &str = row.get(1)?;
        let hire_date:  Date = row.get(2)?;

        assert_eq!(first_name, "Irene");
        assert_eq!(last_name,  "Mikkilineni");

        let expected_hire_date = Date::from_string("September 28, 2006", "MONTH DD, YYYY", &session)?;
        assert_eq!(hire_date.compare(&expected_hire_date)?, Equal);

        Ok(())
    }

    #[test]
    fn mix() -> Result<()> {
        let session = sibyl::test_env::get_session()?;

        let stmt = session.prepare("
            SELECT first_name, last_name, department_name, hire_date
              FROM hr.employees e
              JOIN hr.departments d
                ON d.department_id = e.department_id
             WHERE d.department_name IN (:department_name, :dn2, :dn3, :dn4, :dn5)
               AND d.department_id IN (
                        SELECT department_id
                          FROM hr.employees
                      GROUP BY department_id
                        HAVING Count(*) >= :min_employees )
               AND hire_date BETWEEN :from_date AND :thru_date
          ORDER BY hire_date
        ")?;
        let date_from = Date::from_string("October   1, 2006", "MONTH DD, YYYY", &session)?;
        let date_thru = Date::from_string("December 31, 2006", "MONTH DD, YYYY", &session)?;

        let rows = stmt.query(
            (
                ["Marketing", "Purchasing", "Human Resources", "Shipping", "IT"].as_slice(),
                5,
                [date_from, date_thru].as_slice(),
            )
        )?;
        let row = rows.next()?.unwrap();
        let first_name: &str = row.get(0)?;
        let last_name:  &str = row.get(1)?;
        let dept_name:  &str = row.get(2)?;
        let hire_date: Date  = row.get(3)?;

        assert_eq!(first_name, "Guy");
        assert_eq!(last_name,  "Himuro");
        assert_eq!(dept_name,  "Purchasing");

        let expected_hire_date = Date::from_string("November 15, 2006", "MONTH DD, YYYY", &session)?;
        assert_eq!(hire_date.compare(&expected_hire_date)?, Equal);

        Ok(())
    }

    #[test]
    fn named_mix() -> Result<()> {
        let session = sibyl::test_env::get_session()?;

        let stmt = session.prepare("
            SELECT first_name, last_name, department_name, hire_date
              FROM hr.employees e
              JOIN hr.departments d
                ON d.department_id = e.department_id
             WHERE d.department_name IN (:departments, :2, :3, :4, :5)
               AND d.department_id IN (
                        SELECT department_id
                          FROM hr.employees
                      GROUP BY department_id
                        HAVING Count(*) >= :min_employees )
               AND hire_date BETWEEN :hire_range AND :8
          ORDER BY hire_date
        ")?;
        let date_from = Date::from_string("October   1, 2006", "MONTH DD, YYYY", &session)?;
        let date_thru = Date::from_string("December 31, 2006", "MONTH DD, YYYY", &session)?;

        let rows = stmt.query(
            (
                (":DEPARTMENTS", ["Marketing", "Purchasing", "Human Resources", "Shipping", "IT"].as_slice()),
                (":MIN_EMPLOYEES", 5),
                (":HIRE_RANGE", [date_from, date_thru].as_slice()),
            )
        )?;
        let row = rows.next()?.unwrap();
        let first_name: &str = row.get(0)?;
        let last_name:  &str = row.get(1)?;
        let dept_name:  &str = row.get(2)?;
        let hire_date: Date  = row.get(3)?;

        assert_eq!(first_name, "Guy");
        assert_eq!(last_name,  "Himuro");
        assert_eq!(dept_name,  "Purchasing");

        let expected_hire_date = Date::from_string("November 15, 2006", "MONTH DD, YYYY", &session)?;
        assert_eq!(hire_date.compare(&expected_hire_date)?, Equal);

        Ok(())
    }
}
