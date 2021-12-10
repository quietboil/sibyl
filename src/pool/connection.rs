//! Connection Pool

#[cfg(feature="blocking")]
#[cfg_attr(docsrs, doc(cfg(feature="blocking")))]
mod blocking;

#[cfg(feature="nonblocking")]
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
mod nonblocking;

use std::{sync::Arc, marker::PhantomData};

use crate::{Error, Result, oci::*, Environment};

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
    env:  Arc<Handle<OCIEnv>>,
    err:  Handle<OCIError>,
    pool: Handle<OCICPool>,
    name: &'a str,
    phantom_env: PhantomData<&'a Environment>,
}

impl ConnectionPool<'_> {
    pub(crate) fn clone_env(&self) -> Arc<Handle<OCIEnv>> {
        self.env.clone()
    }

    /**
        Returns the maximum connection idle time. Connections idle for more
        than this time value (in seconds) are terminated to maintain an
        optimum number of open connections.

        If "idle timeout" is not set, the connections are never timed out.

        **Note:** Shrinkage of the pool only occurs when there is a network
        round-trip. If there are no operations, then the connections remain
        active.

        # Example

        🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
        to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest).

        ```
        # use sibyl::Result;
        # #[cfg(feature="blocking")]
        # fn main() -> Result<()> {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 0, 1, 10)?;

        assert_eq!(pool.idle_timeout()?, 0, "idle timeout is not set");
        # Ok(())
        # }
        # #[cfg(feature="nonblocking")]
        # fn main() -> Result<()> {
        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 0, 1, 10).await?;
        # assert_eq!(pool.idle_timeout()?, 0, "idle timeout is not set");
        # Ok(()) })
        # }
        ```
    */
    pub fn idle_timeout(&self) -> Result<u32> {
        self.pool.get_attr(OCI_ATTR_CONN_TIMEOUT, self.err.get())
    }

    /**
        Sets the maximum connection idle time (in seconds).

        # Example

        🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
        to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest).

        ```
        # use sibyl::Result;
        # #[cfg(feature="blocking")]
        # fn main() -> Result<()> {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 1, 1, 10)?;
        // Note that a connection pool must have at least one connection
        // to set its "idle timeout"
        pool.set_idle_timeout(600)?;
        assert_eq!(pool.idle_timeout()?, 600);
        # Ok(())
        # }
        # #[cfg(feature="nonblocking")]
        # fn main() -> Result<()> {
        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 1, 1, 10).await?;
        # pool.set_idle_timeout(600)?;
        # assert_eq!(pool.idle_timeout()?, 600);
        # Ok(()) })
        # }
        ```
    */
    pub fn set_idle_timeout(&self, seconds: u32) -> Result<()> {
        let num_open = self.open_count()?;
        if num_open > 0 {
            self.pool.set_attr(OCI_ATTR_CONN_TIMEOUT, seconds, self.err.get())
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

        🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
        to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest).

        ```
        # use sibyl::Result;
        # #[cfg(feature="blocking")]
        # fn main() -> Result<()> {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 0, 1, 10)?;

        assert!(!pool.is_nowait()?);
        # Ok(())
        # }
        # #[cfg(feature="nonblocking")]
        # fn main() -> Result<()> {
        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 0, 1, 10).await?;
        # assert!(!pool.is_nowait()?);
        # Ok(()) })
        # }
        ```
    */
    pub fn is_nowait(&self) -> Result<bool> {
        let flag : u8 = self.pool.get_attr(OCI_ATTR_CONN_NOWAIT, self.err.get())?;
        println!("nowait={}", flag);
        Ok(flag != 0)
    }

    /**
        Sets the "no wait" mode.

        # Example

        🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
        to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest).

        ```
        # use sibyl::Result;
        # #[cfg(feature="blocking")]
        # fn main() -> Result<()> {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 0, 1, 10)?;

        pool.set_nowait()?;
        assert!(pool.is_nowait()?);
        # Ok(())
        # }
        # #[cfg(feature="nonblocking")]
        # fn main() -> Result<()> {
        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 0, 1, 10).await?;
        # pool.set_nowait()?;
        # assert!(pool.is_nowait()?);
        # Ok(()) })
        # }
        ```
    */
    pub fn set_nowait(&self) -> Result<()> {
        self.pool.set_attr(OCI_ATTR_CONN_NOWAIT, 0u8, self.err.get())
    }

    /**
        Returns the number of (busy) connections.

        # Example

        🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
        to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest).

        ```
        # use sibyl::Result;
        # #[cfg(feature="blocking")]
        # fn main() -> Result<()> {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 2, 2, 10)?;

        assert_eq!(pool.busy_count()?, 0);
        # Ok(())
        # }
        # #[cfg(feature="nonblocking")]
        # fn main() -> Result<()> {
        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 2, 2, 10).await?;
        # assert_eq!(pool.busy_count()?, 0);
        # Ok(()) })
        # }
        ```
    */
    pub fn busy_count(&self) -> Result<usize> {
        let count : u32 = self.pool.get_attr(OCI_ATTR_CONN_BUSY_COUNT, self.err.get())?;
        Ok(count as usize)
    }

    /**
        Returns the number of open connections.

        # Example

        🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
        to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest).

        ```
        # use sibyl::Result;
        # #[cfg(feature="blocking")]
        # fn main() -> Result<()> {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 2, 2, 10)?;

        let num_conn = pool.open_count()?;
        assert_eq!(num_conn, 2);
        # Ok(())
        # }
        # #[cfg(feature="nonblocking")]
        # fn main() -> Result<()> {
        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 2, 2, 10).await?;
        # assert_eq!(pool.open_count()?, 2);
        # Ok(()) })
        # }
       ```
    */
    pub fn open_count(&self) -> Result<usize> {
        let count : u32 = self.pool.get_attr(OCI_ATTR_CONN_OPEN_COUNT, self.err.get())?;
        Ok(count as usize)
    }
}