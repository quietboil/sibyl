use sibyl::*;
/**
    This example is a variant of `readme` that executes its work in multiple
    threads (or async tasks) where each thread (or task) establishes its own
    connection and then uses it to execute queries.

    While this approch might work for some use cases, usually you are better
    off with either a session pool or a connection pool. You would use the
    latter if your work need stateful sessions, but you can allow only so many
    actual database connections.

    *Note* that connection pooling is only available in `blocking` mode.

    *Note* also that `block_on` used in nonblocking version of this example
    abstracts `block_on` for various async executors and is only intended to
    execute Sibyl's async tests and examples. While you can certainly use it,
    most likely you'd want to create your own version of it.
*/
fn main() -> Result<()> {
    example()
}

#[cfg(feature="blocking")]
fn example() -> Result<()> {
    use std::{env, thread, sync::Arc};

    let oracle = sibyl::env()?;
    let oracle = Arc::new(oracle);

    // Start 100 "worker" threads
    let mut workers = Vec::with_capacity(100);
    for _i in 0..workers.capacity() {
        let oracle = oracle.clone();
        let handle = thread::spawn(move || -> Result<Option<(String,String)>> {
            let dbname = env::var("DBNAME").expect("database name");
            let dbuser = env::var("DBUSER").expect("user name");
            let dbpass = env::var("DBPASS").expect("password");

            let session = oracle.connect(&dbname, &dbuser, &dbpass)?;
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
                let last_name : &str = row.get_not_null(1)?;
                let name = first_name.map_or(last_name.to_string(), |first_name| format!("{} {}", first_name, last_name));
                let hire_date : Date = row.get_not_null(2)?;
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

#[cfg(feature="nonblocking")]
fn example() -> Result<()> {
    use std::{env, sync::Arc};

    block_on(async {
        let oracle = sibyl::env()?;
        let oracle = Arc::new(oracle);

        // Start 100 "worker" tasks
        let mut workers = Vec::with_capacity(100);
        for _i in 0..workers.capacity() {
            let oracle = oracle.clone();
            let handle = spawn(async move {
                let dbname = env::var("DBNAME").expect("database name");
                let dbuser = env::var("DBUSER").expect("user name");
                let dbpass = env::var("DBPASS").expect("password");

                let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
                let stmt = session.prepare("
                    SELECT first_name, last_name, hire_date
                      FROM (
                            SELECT first_name, last_name, hire_date
                                 , Row_Number() OVER (ORDER BY hire_date DESC, last_name) AS hire_date_rank
                              FROM hr.employees
                           )
                     WHERE hire_date_rank = 1
                ").await?;
                if let Some( row ) = stmt.query_single(()).await? {
                    let first_name : Option<&str> = row.get(0)?;
                    let last_name : &str = row.get_not_null(1)?;
                    let name = first_name.map_or(last_name.to_string(), |first_name| format!("{} {}", first_name, last_name));
                    let hire_date : Date = row.get_not_null(2)?;
                    let hire_date = hire_date.to_string("FMMonth DD, YYYY")?;

                    Ok::<_,Error>(Some((name, hire_date)))
                } else {
                    Ok(None)
                }
            });
            workers.push(handle);
        }
        let mut n = 1;
        for handle in workers {
            if let Some((name,hire_date)) = handle.await.expect("task's result")? {
                println!("{:?}: {} was hired on {}", n, name, hire_date);
            } else {
                println!("{:?}: did not find the latest hire", n);
            }
            n += 1;
        }
        Ok(())
    })
}
