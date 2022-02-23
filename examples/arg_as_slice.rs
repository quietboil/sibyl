//! This example shows how to pass a collection of same type values as an argument.
//! This capability is intended to make passing arguments for IN parameters simple.
//! However, it can be "abused" :-) to pass multiple consecutive arguments of the
//! same type when convenient.
//!
#[cfg(feature="blocking")]
fn main() -> sibyl::Result<()> {
    use sibyl as oracle;
    use sibyl::Date;

    let oracle = oracle::env()?;
    let dbname = std::env::var("DBNAME").expect("database name");
    let dbuser = std::env::var("DBUSER").expect("user name");
    let dbpass = std::env::var("DBPASS").expect("password");
    let session = oracle.connect(&dbname, &dbuser, &dbpass)?;

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
      ORDER BY hire_date, department_name, last_name, first_name
    ")?;
    let date_from = Date::from_string("July      1, 2006", "MONTH DD, YYYY", &session)?;
    let date_thru = Date::from_string("December 31, 2006", "MONTH DD, YYYY", &session)?;

    let rows = stmt.query(
        (
            (":DEPARTMENTS",   ["Marketing", "Purchasing", "Human Resources", "Shipping", "IT"].as_slice()),
            (":MIN_EMPLOYEES", 5),
            (":HIRE_RANGE",    [date_from, date_thru].as_slice()),
        )
    )?;
    while let Some(row) = rows.next()? {
        let first_name: &str = row.get(0)?;
        let last_name:  &str = row.get(1)?;
        let dept_name:  &str = row.get(2)?;
        let hire_date:  Date = row.get(3)?;

        println!("{:17} {:15} {:11} {:9}",
            hire_date.to_string("Month DD, YYYY")?,
            dept_name, last_name, first_name
        );
    }

    Ok(())
}

#[cfg(feature="nonblocking")]
fn main() {}