/*!
    This example is a variant of `readme` that executes its work in multiple
    threads where each thread establishes its own connection and then uses it
    to execute queries. The connection gets dropped once the work is done
    (i.e. when the closure exits)
*/
use sibyl::*;
use std::{env, thread, sync::Arc};

#[cfg(feature="blocking")]
fn main() -> Result<()> {
    let oracle = sibyl::env()?;
    let oracle = Arc::new(oracle);

    // Start 100 "worker" threads
    let mut workers = Vec::with_capacity(100);
    for _i in 0..workers.capacity() {
        let oracle = oracle.clone();
        let handle = thread::spawn(move || -> Result<Option<(String,String)>> {
            let dbname = env::var("DBNAME").expect("database name");
            let dbuser = env::var("DBUSER").expect("schema name");
            let dbpass = env::var("DBPASS").expect("password");

            let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
            let stmt = conn.prepare("
                SELECT first_name, last_name, hire_date
                  FROM (
                        SELECT first_name, last_name, hire_date
                             , Row_Number() OVER (ORDER BY hire_date DESC, last_name) AS hire_date_rank
                          FROM hr.employees
                       )
                 WHERE hire_date_rank = 1
            ")?;
            let mut rows = stmt.query(&[])?;
            if let Some( row ) = rows.next()? {
                let first_name : Option<&str> = row.get(0)?;
                let last_name : &str = row.get(1)?.unwrap();
                let name = first_name.map_or(last_name.to_string(), |first_name| format!("{} {}", first_name, last_name));
                let hire_date : Date = row.get(2)?.unwrap();
                let hire_date = hire_date.to_string("FMMonth DD, YYYY")?;

                Ok(Some((name, hire_date)))
            } else {
                Ok(None)
            }
        });
        workers.push(handle);
    }
    for handle in workers {
        let worker_id = handle.thread().id();
        if let Some((name,hire_date)) = handle.join().expect("result from worker thread")? {
            println!("{:?}: {} was hired on {}", worker_id, name, hire_date);
        } else {
            println!("{:?}: did not find the latest hire", worker_id);
        }
    }
    Ok(())
}