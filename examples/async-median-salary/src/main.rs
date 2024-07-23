#[tokio::main]
async fn main() -> sibyl::Result<()> {
    let oracle = sibyl::env()?;

    let dbname = std::env::var("DBNAME").expect("database name");
    let dbuser = std::env::var("DBUSER").expect("user name");
    let dbpass = std::env::var("DBPASS").expect("password");
    let region = std::env::var("REGION").expect("HR region");

    let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;

    let stmt = session.prepare("
        SELECT c.country_name, Median(e.salary)
          FROM hr.employees e
          JOIN hr.departments d ON d.department_id = e.department_id
          JOIN hr.locations l   ON l.location_id = d.location_id
          JOIN hr.countries c   ON c.country_id = l.country_id
          JOIN hr.regions r     ON r.region_id = c.region_id
         WHERE r.region_name = :REGION_NAME
      GROUP BY c.country_name
    ").await?;

    let rows = stmt.query(&region).await?;

    while let Some(row) = rows.next().await? {
        let country_name : &str = row.get(0)?;
        let median_salary : u16 = row.get(1)?;
        println!("{:25}: {:>5}", country_name, median_salary);
    }
    Ok(())
}
