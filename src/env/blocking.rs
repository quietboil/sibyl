//! Blocking mode OCI environment methods.

use super::Environment;
use crate::{Connection, ConnectionPool, Result, SessionPool};

impl Environment {
    /**
        Creates and begins a user session for a given server.

        # Parameters

        * `dbname` - The TNS alias of the database to connect to.
        * `username` - The userid with which to start the sessions.
        * `password` - The password for the corresponding `username`.

        # Example
        ```
        let oracle = sibyl::env()?;
        let dbname = std::env::var("DBNAME")?;
        let dbuser = std::env::var("DBUSER")?;
        let dbpass = std::env::var("DBPASS")?;
        let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;

        assert!(!conn.is_async()?);
        assert!(conn.is_connected()?);
        assert!(conn.ping().is_ok());

        let stmt = conn.prepare("
            SELECT DISTINCT client_driver
              FROM v$session_connect_info
             WHERE sid = SYS_CONTEXT('USERENV', 'SID')
        ")?;
        let rows = stmt.query(&[])?;
        let row = rows.next()?.unwrap();
        let client_driver : &str = row.get(0)?.expect("non-NULL client_driver");
        assert_eq!(client_driver, "sibyl");
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn connect(&self, dbname: &str, username: &str, password: &str) -> Result<Connection> {
        Connection::new(self, dbname, username, password)
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
        let dbname = std::env::var("DBNAME")?;
        let dbuser = std::env::var("DBUSER")?;
        let dbpass = std::env::var("DBPASS")?;

        let oracle = sibyl::env()?;
        // Create a session pool where each session will connect to the database
        // `dbname` and authenticate itself as `dbuser` with password `dbpass`.
        // Pool will have no open sessions initially. It will create 2new sessions
        // at at time, up to the maximum of 10 sessions, when they are requested
        // and there are no idle sessions in the pool.
        let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10)?;

        let conn = pool.get_session()?;
        let stmt = conn.prepare("
            SELECT DISTINCT client_driver
              FROM v$session_connect_info
             WHERE sid = SYS_CONTEXT('USERENV', 'SID')
        ")?;
        let rows = stmt.query(&[])?;
        let row = rows.next()?.unwrap();
        let client_driver : &str = row.get(0)?.expect("non-NULL client_driver");
        assert_eq!(client_driver, "sibyl");
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn create_session_pool(&self, dbname: &str, username: &str, password: &str, min: usize, inc: usize, max: usize) -> Result<SessionPool> {
        SessionPool::new(self, dbname, username, password, min, inc, max)
    }

    /**
        Creates new connection pool.

        # Parameters

        * `dbname` - The TNS alias of the database to connect to.
        * `username` - The username with which to start the sessions.
        * `password` - The password for the corresponding `username`.
        * `min` - The minimum number of connections to be opened when the pool is created. After the connection pool is created,
            connections are opened only when necessary. Generally, this parameter should be set to the number of concurrent statements
            that the application is planning or expecting to run.
        * `inc` - incremental number of connections to be opened when all the connections are busy and a call needs a connection.
            This increment is used only when the total number of open connections is less than the maximum number of connections
            that can be opened in that pool.
        * `max` - The maximum number of connections that can be opened to the database. When the maximum number of connections
            are open and all the connections are busy, if a call needs a connection, it waits until it gets one.

        # Example

        ```
        use std::env;

        let dbname = env::var("DBNAME")?;
        let dbuser = env::var("DBUSER")?;
        let dbpass = env::var("DBPASS")?;

        let oracle = sibyl::env()?;        
        let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 1, 1, 10)?;

        let conn = pool.get_session(&dbuser, &dbpass)?;
        let stmt = conn.prepare("
            SELECT DISTINCT client_driver
              FROM v$session_connect_info
             WHERE sid = SYS_CONTEXT('USERENV', 'SID')
        ")?;
        let rows = stmt.query(&[])?;
        let row = rows.next()?.unwrap();
        let client_driver : &str = row.get(0)?.unwrap();
        assert_eq!(client_driver, "sibyl");
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn create_connection_pool(&self, dbname: &str, username: &str, password: &str, min: usize, inc: usize, max: usize) -> Result<ConnectionPool> {
        ConnectionPool::new(self, dbname, username, password, min, inc, max)
    }
}
