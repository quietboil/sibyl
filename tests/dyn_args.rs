#[cfg(feature="blocking")]
mod tests {
    use sibyl::*;
    use sibyl::test_env::get_session;

    #[test]
    fn multitype() -> Result<()> {
        let session = get_session()?;

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
        let mut dept1 = "Marketing";
        let mut dept2 = "Purchasing";
        let mut dept3 = "Human Resources";
        let mut dept4 = "Shipping";
        let mut dept5 = "IT";
        let mut num_emp = 5;
        let mut date_from = Date::from_string("October   1, 2006", "MONTH DD, YYYY", &session)?;
        let mut date_thru = Date::from_string("December 31, 2006", "MONTH DD, YYYY", &session)?;

        let mut args = Vec::<&mut dyn ToSql>::new();
        args.push(&mut dept1); // :department_name
        args.push(&mut dept2); // :dn2
        args.push(&mut dept3); // :dn3
        args.push(&mut dept4); // :dn4
        args.push(&mut dept5); // :dn5
        args.push(&mut num_emp as &mut dyn ToSql); // :min_employees
        args.push(&mut date_from); // :from_date
        args.push(&mut date_thru); // :thru_date

        let row = stmt.query_single(args)?.expect("single row result");
        let first_name: &str = row.get(0)?;
        let last_name : &str = row.get(1)?;
        let dept_name : &str = row.get(2)?;
        let hire_date : Date = row.get(3)?;

        assert_eq!(first_name, "Guy");
        assert_eq!(last_name,  "Himuro");
        assert_eq!(dept_name,  "Purchasing");

        let expected_hire_date = Date::from_string("November 15, 2006", "MONTH DD, YYYY", &session)?;
        assert_eq!(hire_date.compare(&expected_hire_date)?, std::cmp::Ordering::Equal);
        
        Ok(())
    }
}
