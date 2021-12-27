use sibyl::*;

/**
    This example is a variant of `readme` that executes its work in multiple
    threads (or async tasks). It creates a connection pool which threads (or
    tasks) then use to establish their own private sessions with the database,
    which share a small number of physical connections.

    Note that `multi_thread_block_on` used in nonblocking version of this example
    abstracts `block_on` for various executors and is intended to execute async tests
    and examples.
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

    let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 1, 1, 10)?;
    let pool = Arc::new(pool);

    let mut workers = Vec::with_capacity(98);
    for _i in 0..workers.capacity() {
        let pool = pool.clone();
        let handle = thread::spawn(move || -> Result<Option<(String,String)>> {
            let dbuser = env::var("DBUSER").expect("user name");
            let dbpass = env::var("DBPASS").expect("password");

            let conn = pool.get_session(&dbuser, &dbpass)?;
            let stmt = conn.prepare("
                SELECT first_name, last_name, hire_date
                  FROM (
                        SELECT first_name, last_name, hire_date
                             , Row_Number() OVER (ORDER BY hire_date DESC, last_name) AS hire_date_rank
                          FROM hr.employees
                       )
                 WHERE hire_date_rank = 1
            ")?;
            let rows = stmt.query(&[])?;
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
    println!("There are {} open connections in the pool.", pool.open_count()?);
    Ok(())
}

/**
    Connection pools (presently?) are not supported in nonblocking mode.
    OCI returns "ORA-03126 network driver does not support non-blocking operations"
    when one tries to set OCI_ATTR_NONBLOCKING_MODE on a pooled connection.
*/
#[cfg(feature="nonblocking")]
fn main() -> Result<()> {
    Ok(())
}
