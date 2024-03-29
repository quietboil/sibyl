/*!
This is not really an example :-) This is an async variant of the `pooled_sessions` example
that, unlike its progenitor, runs forever. This is used as a regression test for
[issue #3](https://github.com/quietboil/sibyl/issues/3).
*/
#[cfg(feature="nonblocking")]
fn main() -> sibyl::Result<()> {
    sibyl::block_on(async {
        use std::env;
        use std::sync::Arc;
        use once_cell::sync::OnceCell;
        use sibyl::spawn;

        static ORACLE : OnceCell<sibyl::Environment> = OnceCell::new();
        let oracle = ORACLE.get_or_try_init(|| {
            sibyl::Environment::new()
        })?;

        let dbname = env::var("DBNAME").expect("database name");
        let dbuser = env::var("DBUSER").expect("user name");
        let dbpass = env::var("DBPASS").expect("password");

        let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10).await?;
        let pool = Arc::new(pool);

        let mut workers = Vec::with_capacity(10);
        for i in 0..workers.capacity() {
            let pool = pool.clone();
            let handle = spawn(async move {
                for n in 1..10_000 {
                    for _ in 0..1000 {
                        let report = select_latest_hire(&pool).await.expect("selected data");
                        assert_eq!(report, "Amit Banda was hired on April 21, 2008");
                    }
                    println!("{}:{}", i, n);
                }
            });
            workers.push(handle);
        }
        for handle in workers {
            let _ = handle.await;
        }
        Ok(())
    })
}

#[cfg(feature="nonblocking")]
async fn select_latest_hire(pool: &sibyl::SessionPool<'_>) -> sibyl::Result<String> {
    let session = pool.get_session().await?;
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
        let last_name : &str = row.get(1)?;
        let name = first_name.map_or(last_name.to_string(), |first_name| format!("{} {}", first_name, last_name));
        let hire_date : sibyl::Date = row.get(2)?;
        let hire_date = hire_date.to_string("FMMonth DD, YYYY")?;

        Ok(format!("{} was hired on {}", name, hire_date))
    } else {
        Ok("Not found".to_string())
    }
}

#[cfg(feature="blocking")]
fn main() {}