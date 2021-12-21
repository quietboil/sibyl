#[cfg(feature="blocking")]
mod tests {
    use sibyl::{self as oracle, *};
    use std::{env, thread, sync::Arc};

    /**
        Creates multiple connections in a multithreaded environment -
        one connection per thread - using a common shared OCI environment.
    */
    #[test]
    fn connection_per_thread() -> Result<()> {
        let oracle = env()?;
        let oracle = Arc::new(oracle);

        let mut workers = Vec::with_capacity(100);
        for _i in 0..workers.capacity() {
            let oracle = oracle.clone();
            let handle = thread::spawn(move || -> String {
                let dbname = env::var("DBNAME").expect("database name");
                let dbuser = env::var("DBUSER").expect("schema name");
                let dbpass = env::var("DBPASS").expect("password");

                let conn = oracle.connect(&dbname, &dbuser, &dbpass).expect("database connection");
                let stmt = conn.prepare("
                    SELECT first_name, last_name, hire_date
                      FROM (
                            SELECT first_name, last_name, hire_date
                                 , Row_Number() OVER (ORDER BY hire_date DESC, last_name) AS hire_date_rank
                              FROM hr.employees
                           )
                     WHERE hire_date_rank = 1
                ").expect("prepared select");
                fetch_latest_hire(stmt).expect("selected employee name")
            });
            workers.push(handle);
        }
        for handle in workers {
            let name = handle.join().expect("select result");
            assert_eq!(name, "Amit Banda was hired on April 21, 2008");
        }
        Ok(())
    }

    fn fetch_latest_hire(stmt: Statement) -> Result<String> {
        let rows = stmt.query(&[])?;
        if let Some( row ) = rows.next()? {
            let first_name : Option<&str> = row.get(0)?;
            let last_name : &str = row.get(1)?.expect("last_name");
            let name = first_name.map_or(last_name.to_string(), |first_name| format!("{} {}", first_name, last_name));
            let hire_date : oracle::Date = row.get(2)?.expect("hire_date");
            let hire_date = hire_date.to_string("FMMonth DD, YYYY")?;

            Ok(format!("{} was hired on {}", name, hire_date))
        } else {
            Ok("Not found".to_string())
        }
    }

     /**
        Creates a single connections in a multithreaded environment,
        which is then used by (shared between) all threads.
    */
    #[test]
    fn shared_connection() -> Result<()> {
        use once_cell::sync::OnceCell;

        static ORACLE : OnceCell<Environment> = OnceCell::new();
        let oracle = ORACLE.get_or_try_init(|| {
            env()
        })?;

        let dbname = env::var("DBNAME").expect("database name");
        let dbuser = env::var("DBUSER").expect("schema name");
        let dbpass = env::var("DBPASS").expect("password");

        let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let conn = Arc::new(conn);

        let mut workers = Vec::with_capacity(100);
        for _i in 0..workers.capacity() {
            let conn = conn.clone();
            let handle = thread::spawn(move || -> String {
                let stmt = conn.prepare("
                    SELECT first_name, last_name, hire_date
                      FROM (
                            SELECT first_name, last_name, hire_date
                                 , Row_Number() OVER (ORDER BY hire_date DESC, last_name) AS hire_date_rank
                              FROM hr.employees
                           )
                     WHERE hire_date_rank = 1
                ").expect("prepared select");
                fetch_latest_hire(stmt).expect("selected employee name")
            });
            workers.push(handle);
        }
        for handle in workers {
            let name = handle.join().expect("select result");
            assert_eq!(name, "Amit Banda was hired on April 21, 2008");
        }

        Ok(())
    }

    /**
        Creates a session pool in a multithreaded environment.
        Threads get sessions (`Connection`s) from this pool.
    */
    #[test]
    fn pooled_sessions() -> Result<()> {
        use once_cell::sync::OnceCell;

        static ORACLE : OnceCell<Environment> = OnceCell::new();
        let oracle = ORACLE.get_or_try_init(|| {
            env()
        })?;

        let dbname = env::var("DBNAME").expect("database name");
        let dbuser = env::var("DBUSER").expect("schema name");
        let dbpass = env::var("DBPASS").expect("password");

        let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 2, 10)?;
        let pool = Arc::new(pool);

        let mut workers = Vec::with_capacity(100);
        for _i in 0..workers.capacity() {
            let pool = pool.clone();
            let handle = thread::spawn(move || -> String {
                let conn = pool.get_session().expect("database session");
                let stmt = conn.prepare("
                    SELECT first_name, last_name, hire_date
                      FROM (
                            SELECT first_name, last_name, hire_date
                                 , Row_Number() OVER (ORDER BY hire_date DESC, last_name) AS hire_date_rank
                              FROM hr.employees
                           )
                     WHERE hire_date_rank = 1
                ").expect("prepared select");
                fetch_latest_hire(stmt).expect("selected employee name")
            });
            workers.push(handle);
        }
        for handle in workers {
            let name = handle.join().expect("select result");
            assert_eq!(name, "Amit Banda was hired on April 21, 2008");
        }

        Ok(())
    }

    /**
        Creates a connection pool in a multithreaded environment.
        Threads get their own (stateful) sessions fro this pool.
        These sessions, however, share the available connections.
    */
    #[test]
    fn pooled_connections() -> Result<()> {
        use once_cell::sync::OnceCell;

        static ORACLE : OnceCell<Environment> = OnceCell::new();
        let oracle = ORACLE.get_or_try_init(|| {
            env()
        })?;

        let dbname = env::var("DBNAME").expect("database name");
        let dbuser = env::var("DBUSER").expect("schema name");
        let dbpass = env::var("DBPASS").expect("password");

        let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 0, 2, 10)?;
        let pool = Arc::new(pool);
        let user = Arc::new(dbuser);
        let pass = Arc::new(dbpass);

        let mut workers = Vec::with_capacity(100);
        for _i in 0..workers.capacity() {
            let pool = pool.clone();
            let user = user.clone();
            let pass = pass.clone();
            let handle = thread::spawn(move || -> String {
                let conn = pool.get_session(user.as_str(), pass.as_str()).expect("database session");
                let stmt = conn.prepare("
                    SELECT first_name, last_name, hire_date
                      FROM (
                            SELECT first_name, last_name, hire_date
                                 , Row_Number() OVER (ORDER BY hire_date DESC, last_name) AS hire_date_rank
                              FROM hr.employees
                           )
                     WHERE hire_date_rank = 1
                ").expect("prepared select");
                fetch_latest_hire(stmt).expect("selected employee name")
            });
            workers.push(handle);
        }
        for handle in workers {
            let name = handle.join().expect("select result");
            assert_eq!(name, "Amit Banda was hired on April 21, 2008");
        }

        Ok(())
    }
}

#[cfg(feature="nonblocking")]
mod tests {
    use sibyl::*;
    use std::{env, sync::Arc};

    #[test]
    fn session_pool() -> Result<()> {
        sibyl::multi_thread_block_on(async {
            use once_cell::sync::OnceCell;

            static ORACLE : OnceCell<Environment> = OnceCell::new();
            let oracle = ORACLE.get_or_try_init(|| {
                sibyl::env()
            })?;
    
            let dbname = env::var("DBNAME").expect("database name");
            let dbuser = env::var("DBUSER").expect("schema name");
            let dbpass = env::var("DBPASS").expect("password");
    
            let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10).await?;
            let pool = Arc::new(pool);
                
            let mut workers = Vec::with_capacity(100);
            for _i in 0..workers.capacity() {
                let pool = pool.clone();
                let handle = sibyl::spawn(async move {
                    let conn = pool.get_session().await.expect("database session");
                    let stmt = conn.prepare("
                        SELECT first_name, last_name, hire_date
                          FROM (
                                SELECT first_name, last_name, hire_date
                                     , Row_Number() OVER (ORDER BY hire_date DESC, last_name) AS hire_date_rank
                                  FROM hr.employees
                               )
                         WHERE hire_date_rank = 1
                    ").await.expect("prepared select");
                    fetch_latest_hire(stmt).await.expect("selected employee name")
                });
                workers.push(handle);
            }
            for handle in workers {
                let name = handle.await.expect("select result");
                assert_eq!(name, "Amit Banda was hired on April 21, 2008");
            }
    
            Ok(())
        })
    }

    async fn fetch_latest_hire(stmt: Statement<'_>) -> Result<String> {
        let rows = stmt.query(&[]).await?;
        if let Some( row ) = rows.next().await? {
            let first_name : Option<&str> = row.get(0)?;
            let last_name : &str = row.get(1)?.expect("last_name");
            let name = first_name.map_or(last_name.to_string(), |first_name| format!("{} {}", first_name, last_name));
            let hire_date : Date = row.get(2)?.expect("hire_date");
            let hire_date = hire_date.to_string("FMMonth DD, YYYY")?;

            Ok(format!("{} was hired on {}", name, hire_date))
        } else {
            Ok("Not found".to_string())
        }
    }
}
