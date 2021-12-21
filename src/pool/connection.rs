//! Connection Pool

use std::{ptr, sync::Arc, marker::PhantomData};

use crate::{Error, Result, oci::{self, *}, Environment, Connection};

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
    name: &'a [u8],
    phantom_env: PhantomData<&'a Environment>,
}


impl Drop for ConnectionPool<'_> {
    fn drop(&mut self) {
        oci_connection_pool_destroy(&self.pool, &self.err);
    }
}


impl<'a> ConnectionPool<'a> {
    pub(crate) fn new(env: &'a Environment, dbname: &str, username: &str, password: &str, min: usize, inc: usize, max: usize) -> Result<Self> {
        let err = Handle::<OCIError>::new(env)?;
        let pool = Handle::<OCICPool>::new(env)?;
        let mut pool_name_ptr = ptr::null::<u8>();
        let mut pool_name_len = 0u32;
        oci::connection_pool_create(
            env.as_ref(), env.as_ref(), pool.as_ref(),
            &mut pool_name_ptr, &mut pool_name_len,
            dbname.as_ptr(), dbname.len() as u32,
            min as u32, max as u32, inc as u32,
            username.as_ptr(), username.len() as u32,
            password.as_ptr(), password.len() as u32,
            OCI_DEFAULT
        )?;
        let name = unsafe {
            std::slice::from_raw_parts(pool_name_ptr, pool_name_len as usize)
        };
        Ok(Self {env: env.get_env(), err, pool, name, phantom_env: PhantomData})
    }

    pub(crate) fn get_svc_ctx(&self, username: &str, password: &str) -> Result<Ptr<OCISvcCtx>> {
        let inf = Handle::<OCIAuthInfo>::new(self.env.as_ref())?;
        inf.set_attr(OCI_ATTR_DRIVER_NAME, "sibyl", &self.err)?;
        inf.set_attr(OCI_ATTR_USERNAME, username, &self.err)?;
        inf.set_attr(OCI_ATTR_PASSWORD, password, &self.err)?;
        let mut svc = Ptr::<OCISvcCtx>::null();
        let mut found = 0u8;
        oci::session_get(
            self.env.as_ref(), &self.err, svc.as_mut_ptr(), &inf,
            self.name.as_ptr(), self.name.len() as u32, &mut found,
            OCI_SESSGET_CPOOL | OCI_SESSGET_STMTCACHE
        )?;
        Ok(svc)
    }

    pub(crate) fn get_env(&self) -> Arc<Handle<OCIEnv>> {
        self.env.clone()
    }

    /**
        Returns a new session that will be using a virtual connection from this pool.
    */
    pub fn get_session(&self, user: &str, pass: &str) -> Result<Connection> {
        Connection::from_connection_pool(self, user, pass)
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

        ```
        # use sibyl::Result;        
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 1, 1, 10)?;

        assert_eq!(pool.idle_timeout()?, 0, "idle timeout is not set");
        # Ok::<_,sibyl::Error>(())
        ```
    */
    pub fn idle_timeout(&self) -> Result<u32> {
        self.pool.get_attr(OCI_ATTR_CONN_TIMEOUT, &self.err)
    }

    /**
        Sets the maximum connection idle time (in seconds).

        # Example

        ```
        # use sibyl::Result;
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 1, 1, 10)?;
        // Note that a connection pool must have at least one connection ---^
        // to set its "idle timeout"
        pool.set_idle_timeout(600)?;
        assert_eq!(pool.idle_timeout()?, 600);
        # Ok::<_,sibyl::Error>(())
        ```
    */
    pub fn set_idle_timeout(&self, seconds: u32) -> Result<()> {
        let num_open = self.open_count()?;
        if num_open > 0 {
            self.pool.set_attr(OCI_ATTR_CONN_TIMEOUT, seconds, &self.err)
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
        # use sibyl::Result;
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 1, 1, 10)?;

        assert!(!pool.is_nowait()?);
        # Ok::<_,sibyl::Error>(())
        ```
    */
    pub fn is_nowait(&self) -> Result<bool> {
        let flag : u8 = self.pool.get_attr(OCI_ATTR_CONN_NOWAIT, &self.err)?;
        Ok(flag != 0)
    }

    /**
        Sets the "no wait" mode.

        # Example

        ```
        # use sibyl::Result;
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 1, 1, 10)?;

        pool.set_nowait()?;
        assert!(pool.is_nowait()?);
        # Ok::<_,sibyl::Error>(())
        ```
    */
    pub fn set_nowait(&self) -> Result<()> {
        oci::attr_set(self.pool.get_ptr().as_ref(), OCI_HTYPE_CPOOL, std::ptr::null(), 0, OCI_ATTR_CONN_NOWAIT, self.err.as_ref())
    }

    /**
        Returns the number of (busy) connections.

        # Example

        ```
        # use sibyl::Result;
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 1, 1, 10)?;

        assert_eq!(pool.busy_count()?, 0);
        # Ok::<_,sibyl::Error>(())
        ```
    */
    pub fn busy_count(&self) -> Result<usize> {
        let count : u32 = self.pool.get_attr(OCI_ATTR_CONN_BUSY_COUNT, &self.err)?;
        Ok(count as usize)
    }

    /**
        Returns the number of open connections.

        # Example

        ```
        # use sibyl::Result;
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 1, 1, 10)?;

        assert_eq!(pool.open_count()?, 1);
        # Ok::<_,sibyl::Error>(())
       ```
    */
    pub fn open_count(&self) -> Result<usize> {
        let count : u32 = self.pool.get_attr(OCI_ATTR_CONN_OPEN_COUNT, &self.err)?;
        Ok(count as usize)
    }
}