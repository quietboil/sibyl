use sibyl::*;

/**
    This example is a variant of `readme` that executes its work in multiple
    threads (or async tasks). It creates a single connection to the database
    and then threads (or tasks) share it to execute queries.

    Note that most of the time this is **not** how you want to do it :-) as
    this shared connection might become a bottleneck. However, this - sharing
    a single connection - is possble. Hence, this example.
*/
#[cfg(feature="blocking")]
fn main() -> Result<()> {
    use std::{env, thread, sync::Arc};
    use once_cell::sync::OnceCell;

    static ORACLE : OnceCell<Environment> = OnceCell::new();
    let oracle = ORACLE.get_or_try_init(|| {
        env()
    })?;

    let dbname = env::var("DBNAME").expect("database name");
    let dbuser = env::var("DBUSER").expect("user name");
    let dbpass = env::var("DBPASS").expect("password");

    let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    let conn = Arc::new(conn);

    let mut workers = Vec::with_capacity(100);
    for _i in 0..workers.capacity() {
        let conn = conn.clone();
        let handle = thread::spawn(move || -> Result<Option<(String,String)>> {
            let stmt = conn.prepare("
                SELECT first_name, last_name, hire_date
                  FROM (
                        SELECT first_name, last_name, hire_date
                             , Row_Number() OVER (ORDER BY hire_date DESC, last_name) AS hire_date_rank
                          FROM hr.employees
                       )
                 WHERE hire_date_rank = 1
            ")?;
            let rows = stmt.query(())?;
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

/**
    Sharing single connection between tasks is not suported.

    It is possible to implement in principle, but it would require an internal "fences" for futures.
    Considering that the approch as a whole makes little sense, adding overhead of a fence to
    support it makes even less sense.
*/
#[cfg(feature="nonblocking")]
fn main() -> Result<()> {
    eprintln!("Sharing single connection (between tasks in nonblocking mode) is not suported");
    Ok(())
}
