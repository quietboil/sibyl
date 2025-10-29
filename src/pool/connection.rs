//! Connection Pool

use std::{ptr, sync::Arc, marker::PhantomData};

use crate::{Error, Result, oci::{self, *}, Environment, Session};

/**
A shared pool of physical connections.

Connection pooling is beneficial only if the application is multithreaded.
Each thread can maintain a stateful session to the database. The actual
connections to the database are maintained by the connection pool, and
these connections are shared among all the appication threads.

With connection pooling the number of physical connections is less than
the number of database sessions in use by the application.
*/
#[cfg_attr(docsrs, doc(cfg(feature="blocking")))]
pub struct ConnectionPool<'a> {
    pool: Handle<OCICPool>,
    err:  Handle<OCIError>,
    env:  Arc<Handle<OCIEnv>>,
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

    pub(crate) fn get_svc_ctx(&self, auth_info: &OCIAuthInfo) -> Result<Ptr<OCISvcCtx>> {
        let mut svc = Ptr::<OCISvcCtx>::null();
        oci::session_get(
            self.env.as_ref(), &self.err, svc.as_mut_ptr(), &auth_info,
            self.name.as_ptr(), self.name.len() as u32,
            OCI_SESSGET_CPOOL | OCI_SESSGET_STMTCACHE
        )?;
        Ok(svc)
    }

    pub(crate) fn get_env(&self) -> Arc<Handle<OCIEnv>> {
        self.env.clone()
    }

    /**
        Returns a new session that will be using a virtual connection from this pool.

        # Parameters

        * `user` - The username with which to start the session.
        * `pass` - The password for the corresponding `user`.

        # Example

        ```
        use sibyl::{Environment, Session, Date, Result};

        fn main() -> Result<()> {
            use std::{env, thread, sync::Arc};
            use once_cell::sync::OnceCell;

            static ORACLE : OnceCell<Environment> = OnceCell::new();
            let oracle = ORACLE.get_or_try_init(|| {
                Environment::new()
            })?;

            let dbname = env::var("DBNAME").expect("database address");
            let dbuser = env::var("DBUSER").expect("username");
            let dbpass = env::var("DBPASS").expect("password");

            let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 0, 1, 3)?;
            let pool = Arc::new(pool);

            let mut workers = Vec::with_capacity(10);
            while workers.len() < 10 {
                let pool = pool.clone();
                let user = env::var("DBUSER").expect("user name");
                let pass = env::var("DBPASS").expect("password");
                let handle = thread::spawn(move || -> String {

                    let session = pool.get_session(&user, &pass).expect("database session");

                    select_latest_hire(&session).expect("selected employee name")
                });
                workers.push(handle);
            }
            for handle in workers {
                let name = handle.join().expect("select result");
                assert_eq!(name, "Amit Banda was hired on April 21, 2008");
            }
            Ok(())
        }
        # fn select_latest_hire(session: &Session) -> Result<String> {
        #     let stmt = session.prepare("
        #         SELECT first_name, last_name, hire_date
        #           FROM (
        #                 SELECT first_name, last_name, hire_date
        #                      , Row_Number() OVER (ORDER BY hire_date DESC, last_name) AS hire_date_rank
        #                   FROM hr.employees
        #                )
        #          WHERE hire_date_rank = 1
        #     ")?;
        #     if let Some( row ) = stmt.query_single(())? {
        #         let first_name : Option<&str> = row.get(0)?;
        #         let last_name : &str = row.get(1)?;
        #         let name = first_name.map_or(last_name.to_string(), |first_name| format!("{} {}", first_name, last_name));
        #         let hire_date : Date = row.get(2)?;
        #         let hire_date = hire_date.to_string("FMMonth DD, YYYY")?;
        #         Ok(format!("{} was hired on {}", name, hire_date))
        #     } else {
        #         Ok("Not found".to_string())
        #     }
        # }
        ```
    */
    pub fn get_session(&self, user: &str, pass: &str) -> Result<Session<'_>> {
        Session::from_connection_pool(self, user, pass)
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
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 1, 1, 10)?;

    let idle_timeout = pool.idle_timeout()?;

    assert_eq!(idle_timeout, 0, "idle timeout is not set");
    # Ok::<_,sibyl::Error>(())
    ```
    */
    pub fn idle_timeout(&self) -> Result<u32> {
        self.pool.get_attr(OCI_ATTR_CONN_TIMEOUT, &self.err)
    }

    /**
    Sets the maximum connection idle time (in seconds).

    # Parameters

    * `idle_time` - The maximum connection idle time (in seconds)

    # Example

    ```
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 1, 1, 10)?;
    // Note that a connection pool must have at least one connection ---^
    // to set the pool's "idle timeout"

    pool.set_idle_timeout(600)?;

    let idle_timeout = pool.idle_timeout()?;
    assert_eq!(idle_timeout, 600);
    # Ok::<_,sibyl::Error>(())
    ```
    */
    pub fn set_idle_timeout(&self, idle_time: u32) -> Result<()> {
        let num_open = self.open_count()?;
        if num_open > 0 {
            self.pool.set_attr(OCI_ATTR_CONN_TIMEOUT, idle_time, &self.err)
        } else {
            Err(Error::new("pool is empty"))
        }
    }

    /**
    Reports whether retrial for a connection must be performed when all connections
    in the pool are found to be busy and the number of connections has reached the maximum.

    If the pool operates in "no wait" mode, an error is thrown when all the connections
    are busy and no more connections can be opened. Otherwise, the [`ConnectionPool::get_session()`] call
    waits until it gets a connection.

    # Example

    ```
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 1, 1, 10)?;

    let is_nowait_mode = pool.is_nowait()?;

    assert!(!is_nowait_mode);
    # Ok::<_,sibyl::Error>(())
    ```
    */
    pub fn is_nowait(&self) -> Result<bool> {
        let flag : u8 = self.pool.get_attr(OCI_ATTR_CONN_NOWAIT, &self.err)?;
        Ok(flag != 0)
    }

    /**
    Sets the "no wait" mode.

    **Note** that once set "no wait" mode cannot be reset.

    # Example

    ```
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 1, 1, 10)?;

    pool.set_nowait()?;

    let is_nowait_mode = pool.is_nowait()?;
    assert!(is_nowait_mode);
    # Ok::<_,sibyl::Error>(())
    ```
    */
    pub fn set_nowait(&self) -> Result<()> {
        oci::attr_set(self.pool.get_ptr().as_ref(), OCI_HTYPE_CPOOL, std::ptr::null(), 0, OCI_ATTR_CONN_NOWAIT, self.err.as_ref())
    }

    /**
    Returns the number of busy connections.

    # Example

    ```
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 1, 1, 10)?;

    let num_busy = pool.busy_count()?;

    assert_eq!(num_busy, 0);
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
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 1, 1, 10)?;

    let num_open = pool.open_count()?;

    assert_eq!(num_open, 1);
    # Ok::<_,sibyl::Error>(())
    ```
    */
    pub fn open_count(&self) -> Result<usize> {
        let count : u32 = self.pool.get_attr(OCI_ATTR_CONN_OPEN_COUNT, &self.err)?;
        Ok(count as usize)
    }
}