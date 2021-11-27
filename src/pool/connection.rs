//! Connection Pool

#[cfg(feature="blocking")]
#[cfg_attr(docsrs, doc(cfg(feature="blocking")))]
mod blocking;

use crate::{Environment, Error, Result, env::Env, oci::*};

/**
    Connection pool - a shared pool of physical connections.

    Connection pooling is beneficial only if the application is multithreaded.
    Each thread can maintain a stateful session to the database. The actual
    connections to the database are maintained by the connection pool, and
    these connections are shared among all the appication threads.

    With connection pooling the number of physical connections is less than
    the number of database sessions in use by the application.
*/
pub struct ConnectionPool<'a> {
    env: &'a Environment,
    pool: Handle<OCICPool>,
    name: &'a str,
    user: String,
    pass: String,
}

impl ConnectionPool<'_> {
    /**
        Returns the maximum connection idle time. Connections idle for more
        than this time value (in seconds) are terminated to maintain an
        optimum number of open connections.

        If "idle timeout" is not set, the connections are never timed out.

        **Note:** Shrinkage of the pool only occurs when there is a network
        round-trip. If there are no operations, then the connections remain
        active.

        # Example

        ```
        let dbname = std::env::var("DBNAME")?;
        let dbuser = std::env::var("DBUSER")?;
        let dbpass = std::env::var("DBPASS")?;

        let oracle = sibyl::env()?;
        let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 0, 1, 10)?;

        assert_eq!(pool.idle_timeout()?, 0, "idle timeout is not set");
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn idle_timeout(&self) -> Result<u32> {
        self.pool.get_attr(OCI_ATTR_CONN_TIMEOUT, self.env.err_ptr())
    }

    /**
        Sets the maximum connection idle time (in seconds).

        # Example

        ```
        let dbname = std::env::var("DBNAME")?;
        let dbuser = std::env::var("DBUSER")?;
        let dbpass = std::env::var("DBPASS")?;

        let oracle = sibyl::env()?;
        let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 1, 1, 10)?;
        // Note that the pool needs at least one connection to set "idle timeout"
        pool.set_idle_timeout(600)?;
        assert_eq!(pool.idle_timeout()?, 600);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn set_idle_timeout(&self, seconds: u32) -> Result<()> {
        let num_open = self.open_count()?;
        if num_open > 0 {
            self.pool.set_attr(OCI_ATTR_CONN_TIMEOUT, seconds, self.env.err_ptr())
        } else {
            Err(Error::new("pool is empty"))
        }
    }

    /**
        Reports whether retrial for a connection must be performed when all connections
        in the pool are found to be busy and the number of connections has reached the maximum.

        If the pool operates in "no wait" mode, an error is thrown when all the connections
        are busy and no more connections can be opened. Otherwise, the [`get_session()`] call
        waits until it gets a connection.

        # Example

        ```
        let dbname = std::env::var("DBNAME")?;
        let dbuser = std::env::var("DBUSER")?;
        let dbpass = std::env::var("DBPASS")?;

        let oracle = sibyl::env()?;
        let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 0, 1, 10)?;

        assert!(!pool.is_nowait()?);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn is_nowait(&self) -> Result<bool> {
        let flag : u8 = self.pool.get_attr(OCI_ATTR_CONN_NOWAIT, self.env.err_ptr())?;
        println!("nowait={}", flag);
        Ok(flag != 0)
    }

    /**
        Sets the "no wait" mode.

        # Example

        ```
        let dbname = std::env::var("DBNAME")?;
        let dbuser = std::env::var("DBUSER")?;
        let dbpass = std::env::var("DBPASS")?;

        let oracle = sibyl::env()?;
        let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 0, 1, 10)?;

        pool.set_nowait()?;
        assert!(pool.is_nowait()?);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn set_nowait(&self) -> Result<()> {
        self.pool.set_attr(OCI_ATTR_CONN_NOWAIT, 0u8, self.env.err_ptr())
    }

    /**
        Returns the number of (busy) connections.

        # Example

        ```
        let dbname = std::env::var("DBNAME")?;
        let dbuser = std::env::var("DBUSER")?;
        let dbpass = std::env::var("DBPASS")?;

        let oracle = sibyl::env()?;
        let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 2, 2, 10)?;

        let num_busy = pool.busy_count()?;
        assert_eq!(num_busy, 0);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn busy_count(&self) -> Result<usize> {
        let count : u32 = self.pool.get_attr(OCI_ATTR_CONN_BUSY_COUNT, self.env.err_ptr())?;
        Ok(count as usize)
    }

    /**
        Returns the number of open connections.

        # Example

        ```
        let dbname = std::env::var("DBNAME")?;
        let dbuser = std::env::var("DBUSER")?;
        let dbpass = std::env::var("DBPASS")?;

        let oracle = sibyl::env()?;
        let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 2, 2, 10)?;

        let num_conn = pool.open_count()?;
        assert_eq!(num_conn, 2);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn open_count(&self) -> Result<usize> {
        let count : u32 = self.pool.get_attr(OCI_ATTR_CONN_OPEN_COUNT, self.env.err_ptr())?;
        Ok(count as usize)
    }
}