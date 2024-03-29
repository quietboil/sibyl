/*!
This example is a variant of `readme` that executes its work in multiple
threads. It creates a session pool which threads then use to "borrow"
stateless sessions to execute queries.
*/
#[cfg(feature="blocking")]
fn main() -> sibyl::Result<()> {
    use std::{env, thread, sync::Arc};
    use once_cell::sync::OnceCell;
    use sibyl::*;

    static ORACLE : OnceCell<Environment> = OnceCell::new();
    let oracle = ORACLE.get_or_try_init(|| {
        Environment::new()
    })?;

    let dbname = env::var("DBNAME").expect("database name");
    let dbuser = env::var("DBUSER").expect("user name");
    let dbpass = env::var("DBPASS").expect("password");

    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10)?;
    let pool = Arc::new(pool);

    let mut workers = Vec::with_capacity(98);
    for _i in 0..workers.capacity() {
        let pool = pool.clone();
        let handle = thread::spawn(move || -> Result<Option<(String,String)>> {
            let session = pool.get_session()?;
            let stmt = session.prepare("
                SELECT first_name, last_name, hire_date
                  FROM (
                        SELECT first_name, last_name, hire_date
                             , Row_Number() OVER (ORDER BY hire_date DESC, last_name) AS hire_date_rank
                          FROM hr.employees
                       )
                 WHERE hire_date_rank = 1
            ")?;
            if let Some( row ) = stmt.query_single(())? {
                let first_name : Option<&str> = row.get(0)?;
                let last_name : &str = row.get(1)?;
                let name = first_name.map_or(last_name.to_string(), |first_name| format!("{} {}", first_name, last_name));
                let hire_date : Date = row.get(2)?;
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
        match handle.join() {
            Err(err) => {
                println!("cannot join {:?} - {:?}", worker_id, err);
            },
            Ok(Err(err)) => {
                println!("{:?} failed - {:?}", worker_id, err);
            },
            Ok(Ok(None)) => {
                println!("{:?}: did not find the latest hire", worker_id);
            }
            Ok(Ok(Some((name,hire_date)))) => {
                println!("{:?}: {} was hired on {}", worker_id, name, hire_date);
            },
        }
    }
    println!("There are {} open sessions in the pool.", pool.open_count()?);
    Ok(())
}

#[cfg(feature="nonblocking")]
fn main() {}
