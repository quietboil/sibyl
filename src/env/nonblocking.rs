//! Nonblocking mode OCI environment methods.

use super::Environment;
use crate::{Result, Connection, SessionPool};

impl Environment {
    /**
        Creates and begins a user session for a given server.

        # Parameters

        * `dbname` - The TNS alias of the database to connect to.
        * `username` - The userid with which to start the sessions.
        * `password` - The password for the corresponding `username`.

        # Example

        ```
        # sibyl::current_thread_block_on(async {
        let oracle = sibyl::env()?;

        let dbname = std::env::var("DBNAME").expect("database name");
        let dbuser = std::env::var("DBUSER").expect("schema name");
        let dbpass = std::env::var("DBPASS").expect("password");

        let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        conn.ping().await?;
        # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
        ```
    */
    pub async fn connect(&self, dbname: &str, username: &str, password: &str) -> Result<Connection<'_>> {
        Connection::new(self, dbname, username, password).await
    }

    /**
        Creates new session pool.

        # Parameters

        * `dbname` - The TNS alias of the database to connect to.
        * `username` - The username with which to start the sessions.
        * `password` - The password for the corresponding `username`.
        * `min` - The minimum number of sessions in the session pool. This number of sessions will be started
            during pool creation. After `min` sessions are started, sessions are opened only when necessary.
        * `inc` - The next increment for sessions to be started if the current number of sessions is less
            than `max`. The valid values are 0 and higher.
        * `max` - The maximum number of sessions that can be opened in the session pool. After this value is
            reached, no more sessions are opened. The valid values are 1 and higher.

        # Example

        ```
        # sibyl::current_thread_block_on(async {
        let oracle = sibyl::env()?;

        let dbname = std::env::var("DBNAME").expect("database name");
        let dbuser = std::env::var("DBUSER").expect("schema name");
        let dbpass = std::env::var("DBPASS").expect("password");

        // Create a session pool where each session will connect to the database
        // `dbname` and authenticate itself as `dbuser` with password `dbpass`.
        // Pool will have no open sessions initially. It will create 1 new sessios
        // at at time, up to the maximum of 10 sessions, when they are requested
        // and there are no idle sessions in the pool.
        let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10).await?;

        let conn = pool.get_session().await?;
        let stmt = conn.prepare("
            SELECT DISTINCT client_driver
                FROM v$session_connect_info
                WHERE sid = SYS_CONTEXT('USERENV', 'SID')
        ").await?;
        let rows = stmt.query(&[]).await?;
        let row = rows.next().await?.unwrap();
        let client_driver : &str = row.get(0)?.expect("non-NULL client_driver");
        assert_eq!(client_driver, "sibyl");
        # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
        ```
    */
    pub async fn create_session_pool(&self, dbname: &str, username: &str, password: &str, min: usize, inc: usize, max: usize) -> Result<SessionPool<'_>> {
        SessionPool::new(self, dbname, username, password, min, inc, max).await
    }
}

#[cfg(test)]
mod tests {
    use crate::Result;

    #[test]
    fn async_session_pool() -> Result<()> {
        crate::current_thread_block_on(async {
            let oracle = crate::env()?;

            let dbname = std::env::var("DBNAME").expect("database name");
            let dbuser = std::env::var("DBUSER").expect("schema name");
            let dbpass = std::env::var("DBPASS").expect("password");

            // Create a session pool where each session will connect to the database
            // `dbname` and authenticate itself as `dbuser` with password `dbpass`.
            // Pool will have no open sessions initially. It will create 1 new sessios
            // at at time, up to the maximum of 10 sessions, when they are requested
            // and there are no idle sessions in the pool.
            let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10).await?;

            let conn = pool.get_session().await?;
            let stmt = conn.prepare("
                SELECT DISTINCT client_driver
                    FROM v$session_connect_info
                    WHERE sid = SYS_CONTEXT('USERENV', 'SID')
            ").await?;
            let rows = stmt.query(&[]).await?;
            let row = rows.next().await?.unwrap();
            let client_driver : &str = row.get(0)?.expect("non-NULL client_driver");
            assert_eq!(client_driver, "sibyl");

            Ok(())
        })
    }
}
