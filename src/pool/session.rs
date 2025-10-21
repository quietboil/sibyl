//! Session Pool

#[cfg(feature="blocking")]
#[cfg_attr(docsrs, doc(cfg(feature="blocking")))]
mod blocking;

#[cfg(feature="nonblocking")]
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
mod nonblocking;

use std::{sync::Arc, marker::PhantomData};

use crate::{Error, Result, oci::*, Environment};

/**
Internal (Arc protected) details of a session pool.

This ensures that the OCI pool is still accessible when nonblocking [`SessionPool::get_svc_ctx()`]
is still running while the task that called [`SessionPool::get_session()`] has been already destroyed.
*/
pub(crate) struct SPool {
    name: Vec<u8>,
    pool: Handle<OCISPool>,
    info: Handle<OCIAuthInfo>,
    err:  Handle<OCIError>,
    env:  Arc<Handle<OCIEnv>>,
}

impl Drop for SPool {
    fn drop(&mut self) {
        let _ = &self.info;
        oci_session_pool_destroy(&self.pool, &self.err);
    }
}

#[cfg(feature="nonblocking")]
impl SPool {
    pub(crate) fn get_env(&self) -> Arc<Handle<OCIEnv>> {
        self.env.clone()
    }

    pub(crate) fn get_name(&self) -> &[u8] {
        &self.name
    }
}

/**
Session pool creates and maintains a group of stateless sessions to the database.

These sessions are provided to the application as requested. If no sessions are
available, a new one may be created. Thus, the number of sessions in the pool
can increase dynamically. When the application is done (DROPS) with the session,
it is returned to the pool.
*/
pub struct SessionPool<'a> {
    inner: Arc<SPool>,
    phantom_env: PhantomData<&'a Environment>
}

/**
Represents the behavior of the session pool when all sessions in the pool
are found to be busy and the number of sessions has reached the maximum or
the pool must create new connections.
*/
#[derive(Debug, PartialEq, Eq)]
pub enum SessionPoolGetMode {
    /// The thread waits and blocks until a session is freed or a new one is created. This is the default value.
    Wait = 0,
    /// An error is returned if there are no free connections or if the pool must create a new connection.
    NoWait,
    /**
    A new session is created even though all the sessions are busy and the maximum number of sessions has been reached.

    **Note** that if this value is set, it is possible that there can be an attempt to create more sessions than can be
    supported by the instance of the Oracle database.
    */
    ForcedGet,
    /// Keep trying internally for a free session until the time out expires.
    TimedWait,
}

impl SessionPool<'_> {
    pub(crate) fn get_spool(&self) -> Arc<SPool> {
        self.inner.clone()
    }

    #[cfg(feature="blocking")]
    pub(crate) fn get_env(&self) -> Arc<Handle<OCIEnv>> {
        self.inner.env.clone()
    }

    /**
    Returns the number of sessions checked out from the pool.

    # Example

    ## Blocking

    ```
    # #[cfg(feature="blocking")]
    # fn main() -> sibyl::Result<()> {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 2, 2, 10)?;

    let num_busy = pool.busy_count()?;

    assert_eq!(num_busy, 0);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() {}
    ```

    ## Nonblocking

    ```
    # #[cfg(feature="nonblocking")]
    # fn main() -> sibyl::Result<()> {
    # sibyl::block_on(async {
    # use once_cell::sync::OnceCell;
    # static ORACLE: OnceCell<sibyl::Environment> = OnceCell::new();
    # let oracle = ORACLE.get_or_try_init(|| sibyl::Environment::new())?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 2, 2, 10).await?;

    let num_busy = pool.busy_count()?;

    assert_eq!(num_busy, 0);
    # Ok(()) })
    # }
    # #[cfg(feature="blocking")]
    # fn main() {}
    ```
    */
    pub fn busy_count(&self) -> Result<usize> {
        let count : u32 = self.inner.pool.get_attr(OCI_ATTR_SPOOL_BUSY_COUNT, &self.inner.err)?;
        Ok(count as usize)
    }

    /**
    Returns the number of open sessions.

    # Example

    ## Blocking

    ```
    # #[cfg(feature="blocking")]
    # fn main() -> sibyl::Result<()> {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 2, 2, 10)?;

    let num_sessions = pool.open_count()?;

    assert_eq!(num_sessions, 2);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() {}
    ```

    ## Nonblocking

    ```
    # #[cfg(feature="nonblocking")]
    # fn main() -> sibyl::Result<()> {
    # sibyl::block_on(async {
    # use once_cell::sync::OnceCell;
    # static ORACLE: OnceCell<sibyl::Environment> = OnceCell::new();
    # let oracle = ORACLE.get_or_try_init(|| sibyl::Environment::new())?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 2, 2, 10).await?;

    let num_sessions = pool.open_count()?;

    assert_eq!(num_sessions, 2);
    # Ok(()) })
    # }
    # #[cfg(feature="blocking")]
    # fn main() {}
    ```
    */
    pub fn open_count(&self) -> Result<usize> {
        let count : u32 = self.inner.pool.get_attr(OCI_ATTR_SPOOL_OPEN_COUNT, &self.inner.err)?;
        Ok(count as usize)
    }

    /**
    Returns the "get mode" or the behavior of the session pool when all sessions in the pool
    are found to be busy and the number of sessions has reached the maximum.

    # Example

    ## Blocking

    ```
    use sibyl::SessionPoolGetMode;

    # #[cfg(feature="blocking")]
    # fn main() -> sibyl::Result<()> {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10)?;

    let get_mode = pool.get_mode()?;

    assert_eq!(get_mode, SessionPoolGetMode::Wait);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() {}
    ```

    ## Nonblocking

    ```
    use sibyl::SessionPoolGetMode;

    # #[cfg(feature="nonblocking")]
    # fn main() -> sibyl::Result<()> {
    # sibyl::block_on(async {
    # use once_cell::sync::OnceCell;
    # static ORACLE: OnceCell<sibyl::Environment> = OnceCell::new();
    # let oracle = ORACLE.get_or_try_init(|| sibyl::Environment::new())?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10).await?;

    let get_mode = pool.get_mode()?;

    assert_eq!(get_mode, SessionPoolGetMode::Wait);
    # Ok(()) })
    # }
    # #[cfg(feature="blocking")]
    # fn main() {}
    ```
    */
    pub fn get_mode(&self) -> Result<SessionPoolGetMode> {
        let mode : u8 = self.inner.pool.get_attr(OCI_ATTR_SPOOL_GETMODE, &self.inner.err)?;
        match mode {
            OCI_SPOOL_ATTRVAL_WAIT      => Ok(SessionPoolGetMode::Wait),
            OCI_SPOOL_ATTRVAL_NOWAIT    => Ok(SessionPoolGetMode::NoWait),
            OCI_SPOOL_ATTRVAL_FORCEGET  => Ok(SessionPoolGetMode::ForcedGet),
            OCI_SPOOL_ATTRVAL_TIMEDWAIT => Ok(SessionPoolGetMode::TimedWait),
            _ => Err(Error::new("unknown get mmode returned"))
        }
    }

    /**
    Sets "get mode" or the behavior of the session pool when all sessions in the pool
    are found to be busy and the number of sessions has reached the maximum.

    # Parameters

    * `mode` - new pool "get mode"

    # Example

    ## Blocking

    ```
    use sibyl::SessionPoolGetMode;

    # #[cfg(feature="blocking")]
    # fn main() -> sibyl::Result<()> {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10)?;

    pool.set_get_mode(SessionPoolGetMode::ForcedGet)?;
    # let get_mode = pool.get_mode()?;
    # assert_eq!(get_mode, SessionPoolGetMode::ForcedGet);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() {}
    ```

    ## Nonblocking

    ```
    use sibyl::SessionPoolGetMode;

    # #[cfg(feature="nonblocking")]
    # fn main() -> sibyl::Result<()> {
    # sibyl::block_on(async {
    # use once_cell::sync::OnceCell;
    # static ORACLE: OnceCell<sibyl::Environment> = OnceCell::new();
    # let oracle = ORACLE.get_or_try_init(|| sibyl::Environment::new())?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10).await?;

    pool.set_get_mode(SessionPoolGetMode::ForcedGet)?;
    # let get_mode = pool.get_mode()?;
    # assert_eq!(get_mode, SessionPoolGetMode::ForcedGet);
    # Ok(()) })
    # }
    # #[cfg(feature="blocking")]
    # fn main() {}
    ```
    */
    pub fn set_get_mode(&self, mode: SessionPoolGetMode) -> Result<()> {
        self.inner.pool.set_attr(OCI_ATTR_SPOOL_GETMODE, mode as u8, &self.inner.err)
    }

    /**
    Returns the maximum time (in milliseconds) [`SessionPool::get_session()`] would wait
    for a free session when "get mode" is set to [`SessionPoolGetMode::TimedWait`].

    # Example

    ## Blocking

    ```
    # #[cfg(feature="blocking")]
    # fn main() -> sibyl::Result<()> {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10)?;

    let get_session_max_wait_time = pool.wait_timeout()?;

    // The out-of-the-box "wait timeout" (on 64-bit Linux, Instant Client 19.13)
    // is 5000 ms. This, however, is not documented anywhere. So, there might be
    // a chance that other OCI implementations would set it to a different value.
    assert_eq!(get_session_max_wait_time, 5000);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() {}
    ```

    ## Nonblocking

    ```
    # #[cfg(feature="nonblocking")]
    # fn main() -> sibyl::Result<()> {
    # sibyl::block_on(async {
    # use once_cell::sync::OnceCell;
    # static ORACLE: OnceCell<sibyl::Environment> = OnceCell::new();
    # let oracle = ORACLE.get_or_try_init(|| sibyl::Environment::new())?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10).await?;
    
    let get_session_max_wait_time = pool.wait_timeout()?;
    
    assert_eq!(get_session_max_wait_time, 5000);
    # Ok(()) })
    # }
    # #[cfg(feature="blocking")]
    # fn main() {}
    ```
    */
    pub fn wait_timeout(&self) -> Result<u32> {
        self.inner.pool.get_attr(OCI_ATTR_SPOOL_WAIT_TIMEOUT, &self.inner.err)
    }

    /**
    Sets the maximum time (in milliseconds) [`SessionPool::get_session()`] would wait
    for a free session when "get mode" is set to [`SessionPoolGetMode::TimedWait`].

    # Parameters

    * `milliseconds` - "get session" wait timeout.

    # Example

    ## Blocking

    ```
    # #[cfg(feature="blocking")]
    # fn main() -> sibyl::Result<()> {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10)?;

    pool.set_wait_timeout(1000)?;
    # let wait_timeout = pool.wait_timeout()?;
    # assert_eq!(wait_timeout, 1000);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() {}
    ```

    ## Nonblocking

    ```
    # #[cfg(feature="nonblocking")]
    # fn main() -> sibyl::Result<()> {
    # sibyl::block_on(async {
    # use once_cell::sync::OnceCell;
    # static ORACLE: OnceCell<sibyl::Environment> = OnceCell::new();
    # let oracle = ORACLE.get_or_try_init(|| sibyl::Environment::new())?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10).await?;

    pool.set_wait_timeout(1000)?;
    # let wait_timeout = pool.wait_timeout()?;
    # assert_eq!(wait_timeout, 1000);
    # Ok(()) })
    # }
    # #[cfg(feature="blocking")]
    # fn main() {}
    ```
    */
    pub fn set_wait_timeout(&self, milliseconds: u32) -> Result<()> {
        self.inner.pool.set_attr(OCI_ATTR_SPOOL_WAIT_TIMEOUT, milliseconds, &self.inner.err)
    }

    /**
    Returns maximum idle time for sessions (in seconds).

    # Example

    ## Blocking

    ```
    # #[cfg(feature="blocking")]
    # fn main() -> sibyl::Result<()> {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10)?;

    let session_max_idle_time = pool.idle_timeout()?;

    assert_eq!(session_max_idle_time, 0);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() {}
    ```

    ## Nonblocking

    ```
    # #[cfg(feature="nonblocking")]
    # fn main() -> sibyl::Result<()> {
    # sibyl::block_on(async {
    # use once_cell::sync::OnceCell;
    # static ORACLE: OnceCell<sibyl::Environment> = OnceCell::new();
    # let oracle = ORACLE.get_or_try_init(|| sibyl::Environment::new())?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10).await?;

    let session_max_idle_time = pool.idle_timeout()?;

    assert_eq!(session_max_idle_time, 0);
    # Ok(()) })
    # }
    # #[cfg(feature="blocking")]
    # fn main() {}
    ```
    */
    pub fn idle_timeout(&self) -> Result<u32> {
        self.inner.pool.get_attr(OCI_ATTR_SPOOL_TIMEOUT, &self.inner.err)
    }

    /**
    Sets maximum idle time for sessions (in seconds).

    Sessions that are idle for more than this time are terminated periodically to maintain
    an optimum number of open sessions. If this attribute is not set, the least recently
    used sessions may be timed out if and when space in the pool is required.
    The idle sessions are checked when a busy one is released back to the pool.

    # Parameters

    * `seconds` - maximum session idle time

    # Example

    ## Blocking

    ```
    # #[cfg(feature="blocking")]
    # fn main() -> sibyl::Result<()> {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 1, 1, 10)?;

    pool.set_idle_timeout(600);
    # let timeout = pool.idle_timeout()?;
    # assert_eq!(timeout, 600, "pool idle timeout");
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() {}
    ```

    ## Nonblocking

    ```
    # #[cfg(feature="nonblocking")]
    # fn main() -> sibyl::Result<()> {
    # sibyl::block_on(async {
    # use once_cell::sync::OnceCell;
    # static ORACLE: OnceCell<sibyl::Environment> = OnceCell::new();
    # let oracle = ORACLE.get_or_try_init(|| sibyl::Environment::new())?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 1, 1, 10).await?;
    
    pool.set_idle_timeout(600)?;
    # let timeout = pool.idle_timeout()?;
    # assert_eq!(timeout, 600);
    # Ok(()) })
    # }
    # #[cfg(feature="blocking")]
    # fn main() {}
    ```
    */
    pub fn set_idle_timeout(&self, seconds: u32) -> Result<()> {
        self.inner.pool.set_attr(OCI_ATTR_SPOOL_TIMEOUT, seconds, &self.inner.err)
    }

    /**
    Returns the lifetime (in seconds) for all the sessions in the pool.

    Sessions in the pool are terminated when they have reached or exceeded their lifetime.

    # Example

    ## Blocking

    ```
    # #[cfg(feature="blocking")]
    # fn main() -> sibyl::Result<()> {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10)?;

    let max_lifetime = pool.session_max_lifetime()?;

    assert_eq!(max_lifetime, 0);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() {}
    ```

    ## Nonblocking

    ```
    # #[cfg(feature="nonblocking")]
    # fn main() -> sibyl::Result<()> {
    # sibyl::block_on(async {
    # use once_cell::sync::OnceCell;
    # static ORACLE: OnceCell<sibyl::Environment> = OnceCell::new();
    # let oracle = ORACLE.get_or_try_init(|| sibyl::Environment::new())?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10).await?;

    let max_lifetime = pool.session_max_lifetime()?;

    assert_eq!(max_lifetime, 0);
    # Ok(()) })
    # }
    # #[cfg(feature="blocking")]
    # fn main() {}
    ```
    */
    pub fn session_max_lifetime(&self) -> Result<u32> {
        self.inner.pool.get_attr(OCI_ATTR_SPOOL_MAX_LIFETIME_SESSION, &self.inner.err)
    }

    /**
    Sets the lifetime (in seconds) for all the sessions in the pool.

    Sessions in the pool are terminated when they have reached or exceeded their lifetime.

    # Parameters

    * `seconds` - duration of the session lifetime

    # Example

    ## Blocking

    ```
    # #[cfg(feature="blocking")]
    # fn main() -> sibyl::Result<()> {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10)?;

    pool.set_session_max_lifetime(10 * 3600)?;
    # let max_lifetime = pool.session_max_lifetime()?;
    # assert_eq!(max_lifetime, 10 * 3600);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() {}
    ```

    ## Nonblocking

    ```
    # #[cfg(feature="nonblocking")]
    # fn main() -> sibyl::Result<()> {
    # sibyl::block_on(async {
    # use once_cell::sync::OnceCell;
    # static ORACLE: OnceCell<sibyl::Environment> = OnceCell::new();
    # let oracle = ORACLE.get_or_try_init(|| sibyl::Environment::new())?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10).await?;

    pool.set_session_max_lifetime(10 * 3600)?;
    # let max_lifetime = pool.session_max_lifetime()?;
    # assert_eq!(max_lifetime, 10 * 3600);
    # Ok(()) })
    # }
    # #[cfg(feature="blocking")]
    # fn main() {}
    ```
    */
    pub fn set_session_max_lifetime(&self, seconds: u32) -> Result<()> {
        self.inner.pool.set_attr(OCI_ATTR_SPOOL_MAX_LIFETIME_SESSION, seconds, &self.inner.err)
    }

    /**
    Returns the maximum number of times one session can be checked out of the session pool.
    After that the session is automatically destroyed. The default value is 0, which means
    there is no limit.

    # Example

    ## Blocking

    ```
    # #[cfg(feature="blocking")]
    # fn main() -> sibyl::Result<()> {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10)?;

    let max_use_count = pool.session_max_use_count()?;

    assert_eq!(max_use_count, 0);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() {}
    ```

    ## Nonblocking

    ```
    # #[cfg(feature="nonblocking")]
    # fn main() -> sibyl::Result<()> {
    # sibyl::block_on(async {
    # use once_cell::sync::OnceCell;
    # static ORACLE: OnceCell<sibyl::Environment> = OnceCell::new();
    # let oracle = ORACLE.get_or_try_init(|| sibyl::Environment::new())?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10).await?;
    
    let max_use_count = pool.session_max_use_count()?;
    
    assert_eq!(max_use_count, 0);
    # Ok(()) })
    # }
    # #[cfg(feature="blocking")]
    # fn main() {}
    ```
    */
    pub fn session_max_use_count(&self) -> Result<u32> {
        self.inner.pool.get_attr(OCI_ATTR_SPOOL_MAX_USE_SESSION, &self.inner.err)
    }

    /**
    Sets the maximum number of times one session can be checked out of the session pool.

    # Parameters

    * `count` - The maximum number of times one session can be checked out of the session pool.

    # Example

    ## Blocking

    ```
    # #[cfg(feature="blocking")]
    # fn main() -> sibyl::Result<()> {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10)?;

    pool.set_session_max_use_count(10_000)?;
    # let max_use_count = pool.session_max_use_count()?;
    # assert_eq!(max_use_count, 10_000);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() {}
    ```

    ## Nonblocking
    ```
    # #[cfg(feature="nonblocking")]
    # fn main() -> sibyl::Result<()> {
    # sibyl::block_on(async {
    # use once_cell::sync::OnceCell;
    # static ORACLE: OnceCell<sibyl::Environment> = OnceCell::new();
    # let oracle = ORACLE.get_or_try_init(|| sibyl::Environment::new())?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10).await?;

    pool.set_session_max_use_count(10_000)?;
    # let max_use_count = pool.session_max_use_count()?;
    # assert_eq!(max_use_count, 10_000);
    # Ok(()) })
    # }
    # #[cfg(feature="blocking")]
    # fn main() {}
    ```
    */
    pub fn set_session_max_use_count(&self, count: u32) -> Result<()> {
        self.inner.pool.set_attr(OCI_ATTR_SPOOL_MAX_USE_SESSION, count, &self.inner.err)
    }

    /**
    Returns the default statement cache size (in number of statements). The default value is 20.
    When an application asks for a session from a session pool, the statement cache size
    for that session defaults to that of the pool.

    # Example

    ## Blocking

    ```
    # #[cfg(feature="blocking")]
    # fn main() -> sibyl::Result<()> {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 1, 1, 10)?;

    let cache_size = pool.statement_cache_size()?;

    assert_eq!(cache_size, 20);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() {}
    ```

    ## Nonblocking

    ```
    # #[cfg(feature="nonblocking")]
    # fn main() -> sibyl::Result<()> {
    # sibyl::block_on(async {
    # use once_cell::sync::OnceCell;
    # static ORACLE: OnceCell<sibyl::Environment> = OnceCell::new();
    # let oracle = ORACLE.get_or_try_init(|| sibyl::Environment::new())?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 1, 1, 10).await?;

    let cache_size = pool.statement_cache_size()?;

    assert_eq!(cache_size, 20);
    # Ok(()) })
    # }
    # #[cfg(feature="blocking")]
    # fn main() {}
    ```
    */
    pub fn statement_cache_size(&self) -> Result<u32> {
        self.inner.pool.get_attr(OCI_ATTR_SPOOL_STMTCACHESIZE, &self.inner.err)
    }

    /**
    Sets the default statement cache size (in number of statements) .

    The change is reflected on individual sessions in the pool, when they are provided to a user.

    # Parameters

    * `size` - cache size in number of statements

    # Example

    ## Blocking

    ```
    # #[cfg(feature="blocking")]
    # fn main() -> sibyl::Result<()> {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10)?;

    pool.set_statement_cache_size(100)?;
    # let cache_size = pool.statement_cache_size()?;
    # assert_eq!(cache_size, 100);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() {}
    ```

    ## Nonblocking

    ```
    # #[cfg(feature="nonblocking")]
    # fn main() -> sibyl::Result<()> {
    # sibyl::block_on(async {
    # use once_cell::sync::OnceCell;
    # static ORACLE: OnceCell<sibyl::Environment> = OnceCell::new();
    # let oracle = ORACLE.get_or_try_init(|| sibyl::Environment::new())?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10).await?;

    pool.set_statement_cache_size(100)?;
    # let cache_size = pool.statement_cache_size()?;
    # assert_eq!(cache_size, 100);
    # Ok(()) })
    # }
    # #[cfg(feature="blocking")]
    # fn main() {}
    ```
    */
    pub fn set_statement_cache_size(&self, size: u32) -> Result<()> {
        self.inner.pool.set_attr(OCI_ATTR_SPOOL_STMTCACHESIZE, size, &self.inner.err)
    }
}
