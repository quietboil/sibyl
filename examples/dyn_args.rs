#[cfg(feature="blocking")]
fn main() -> sibyl::Result<()> {
    use sibyl::*;

    let dbname = std::env::var("DBNAME").expect("database name");
    let dbuser = std::env::var("DBUSER").expect("user name");
    let dbpass = std::env::var("DBPASS").expect("password");
    let oracle = Environment::new()?;
    let session = oracle.connect(&dbname, &dbuser, &dbpass)?;

    // Assume that this was assembled dynamically from bits and pieces
    let sql = String::from("
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
           AND hire_date BETWEEN To_Date(:from_date,'MONTH DD, YYYY')
                             AND To_Date(:thru_date,'MONTH DD, YYYY')
      ORDER BY hire_date
    ");
    // Assume that values for them arrived from elsewhere, and were collected
    // while the above SQL was being constructed
    let mut vals = Vec::new();
    vals.push("Marketing".to_string());
    vals.push("Purchasing".to_string());
    vals.push("Human Resources".to_string());
    vals.push("Shipping".to_string());
    vals.push("IT".to_string());
    vals.push(5.to_string());
    vals.push("October 1, 2006".to_string());
    vals.push("December 31, 2006".to_string());

    let stmt = session.prepare(&sql)?;

    // convert argument values into objects
    let args : Vec<&mut dyn ToSql> = vals.iter_mut().map(|v| v as &mut dyn ToSql).collect();

    let row = stmt.query_single(args)?.expect("single row result");
    let first_name: &str = row.get(0)?;
    let last_name : &str = row.get(1)?;
    let dept_name : &str = row.get(2)?;
    let hire_date : Date = row.get(3)?;

    println!("{first_name} {last_name} from {dept_name} was hired on {}", hire_date.to_string("fmMonth DD, YYYY")?);

    Ok(())
}
