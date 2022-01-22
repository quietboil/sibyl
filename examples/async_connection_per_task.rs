/*!
This example is a variant of `readme` that executes its work in multiple
async tasks where each task establishes its own session and then uses it
to execute queries.

While this approch might work for some use cases, usually you are better
off with a session pool.

*Note* that `block_on` used in this example abstracts `block_on` for
various async executors and is only intended to execute Sibyl's async
tests and examples. While you can certainly use it, most likely you'd
want to create your own.
*/
#[cfg(feature="nonblocking")]
fn main() -> sibyl::Result<()> {
    sibyl::block_on(async {
        use std::{env, sync::Arc};
        use sibyl::*;

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
            let worker_result = handle.await;
            #[cfg(any(feature="tokio", feature="actix"))]
            let worker_result = worker_result.expect("completed task result");

            if let Some((name,hire_date)) = worker_result? {
                println!("{:?}: {} was hired on {}", n, name, hire_date);
            } else {
                println!("{:?}: did not find the latest hire", n);
            }
            n += 1;
        }
        Ok(())
    })
}

#[cfg(feature="blocking")]
fn main() {}
