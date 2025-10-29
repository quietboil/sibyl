//! Session pool nonblocking mode implementation

use super::{SessionPool, SPool};
use crate::{Session, Result, oci::{self, *}, Environment, task};
use std::{ptr, slice, str, marker::PhantomData, sync::Arc};

impl SPool {
    pub(crate) async fn new(env: &Environment, dblink: &str, username: &str, password: &str, min: usize, inc: usize, max: usize, session_state_fixup_callback: &str) -> Result<Self> {
        let err  = Handle::<OCIError>::new(&env)?;
        let pool = Handle::<OCISPool>::new(&env)?;
        let info = Handle::<OCIAuthInfo>::new(&env)?;
        info.set_attr(OCI_ATTR_DRIVER_NAME, "sibyl", &err)?;
        if session_state_fixup_callback.len() > 0 {
            info.set_attr(OCI_ATTR_FIXUP_CALLBACK, session_state_fixup_callback, &err)?;
        }
        pool.set_attr(OCI_ATTR_SPOOL_AUTH, info.get_ptr(), &err)?;

        let mut spool = Self { pool, info, err, env: env.get_env(), name: Vec::new() };
        let dblink = String::from(dblink);
        let username = String::from(username);
        let password = String::from(password);

        task::execute_blocking(move || -> Result<Self> {
            let mut pool_name_ptr = ptr::null::<u8>();
            let mut pool_name_len = 0u32;
            oci::session_pool_create(
                spool.env.as_ref(), spool.err.as_ref(), spool.pool.as_ref(),
                &mut pool_name_ptr, &mut pool_name_len,
                dblink.as_ptr(), dblink.len() as _,
                min as _, max as _, inc as _,
                username.as_ptr(), username.len() as _,
                password.as_ptr(), password.len() as _,
                OCI_SPC_HOMOGENEOUS | OCI_SPC_STMTCACHE
            )?;
            let name = unsafe {
                slice::from_raw_parts(pool_name_ptr, pool_name_len as usize)
            };
            spool.name.extend_from_slice(name);
            Ok(spool)
        }).await?
    }
}

impl<'a> SessionPool<'a> {
    pub(crate) async fn new(env: &'a Environment, dblink: &str, username: &str, password: &str, min: usize, inc: usize, max: usize, session_state_fixup_callback: &str) -> Result<SessionPool<'a>> {
        let inner = SPool::new(env, dblink, username, password, min, inc, max, session_state_fixup_callback).await?;
        let inner = Arc::new(inner);
        Ok(Self { inner, phantom_env: PhantomData })
    }

    /**
        Returns a default (untagged) session from this session pool.

        # Example

        ```
        use sibyl::{Environment, Session, Date, Result};
        use once_cell::sync::OnceCell;
        use std::{env, sync::Arc};

        fn main() -> Result<()> {
            // `sibyl::block_on` is used to run Sibyl's async doc tests
            // and would not be used when `main` can be declared as async.
            sibyl::block_on(async {
                static ORACLE : OnceCell<Environment> = OnceCell::new();
                let oracle = ORACLE.get_or_try_init(|| {
                    Environment::new()
                })?;

                let dbname = env::var("DBNAME").expect("database name");
                let dbuser = env::var("DBUSER").expect("user name");
                let dbpass = env::var("DBPASS").expect("password");

                let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 4).await?;
                let pool = Arc::new(pool);

                let mut workers = Vec::with_capacity(10);
                for _i in 0..workers.capacity() {
                    let pool = pool.clone();
                    // `sibyl::spawn` abstracts `spawn` for Sibyl's use with different async
                    // runtimes. One does not need it when a specific runtime is selected.
                    let handle = sibyl::spawn(async move {
                        let session = pool.get_session().await?;

                        select_latest_hire(&session).await
                    });
                    workers.push(handle);
                }
                for handle in workers {
                    let worker_result = handle.await;
                    #[cfg(any(feature="tokio", feature="actix"))]
                    let worker_result = worker_result.expect("completed task result");

                    let name = worker_result?;
                    assert_eq!(name, "Amit Banda was hired on April 21, 2008");
                }
                Ok(())
            })
        }

        async fn select_latest_hire(session: &Session<'_>) -> Result<String> {
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
                let hire_date : Date = row.get(2)?;
                let hire_date = hire_date.to_string("FMMonth DD, YYYY")?;
                Ok(format!("{} was hired on {}", name, hire_date))
            } else {
                Ok("Not found".to_string())
            }
        }
        ```
    */
    pub async fn get_session(&self) -> Result<Session<'_>> {
        let (session, _found) = Session::from_session_pool(self, "").await?;
        Ok(session)
    }

    /**
    Returns a tagged session, i.e. a session with the specified type/tag.

    The tags provide a way to customize sessions in the pool. A client can get a default or untagged
    session from a pool, set certain attributes on the session (such as globalization settings) and
    label it with an appropriate tag.

    This session or a session with the same attributes can then be requested by providing the same
    tag that was used during session customization.

    If a user asks for a session with tag 'A', and a matching session is not available, an appropriately
    authenticated untagged session is returned, if such a session is free.

    # Parameters

    - `taginfo` - Either a single or a [Multi-Property Tag](https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/session-and-connection-pooling.html#GUID-DFA21225-E83C-4177-A79A-B8BA29DC662C).
    
    # Returns

    A tuple with the returned session and the boolean flag. The flag indicates whether the type of the returned
    session is the same as requested.

    # Example

    ```
    # fn main() -> sibyl::Result<()> {
    # sibyl::block_on(async {
    # use once_cell::sync::OnceCell;
    # static ORACLE: OnceCell<sibyl::Environment> = OnceCell::new();
    # let oracle = ORACLE.get_or_try_init(|| sibyl::Environment::new())?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 5).await?;

    {
        let (session,found) = pool.get_tagged_session("CUSTOM").await?;
        assert!(!found, "a default (not yet tagged) session was returned");
        if !found {
    #        let stmt = session.prepare("SELECT value FROM nls_session_parameters WHERE parameter=:PARAM_NAME").await?;
    #        let row = stmt.query_single("NLS_DATE_FORMAT").await?.expect("one row");
    #        let fmt: &str = row.get(0)?;
    #        assert_ne!(fmt, "YYYY-MM-DD", "Default NLS_DATE_FORMAT differs from what we want to set");
            let stmt = session.prepare("ALTER SESSION SET NLS_DATE_FORMAT='YYYY-MM-DD'").await?;
            stmt.execute(()).await?;
            session.set_tag("CUSTOM")?;
        }
    } // session is released back to the pool here

    {
        let (session,found) = pool.get_tagged_session("CUSTOM").await?;
        assert!(found, "the customized session was returned");

        let stmt = session.prepare("SELECT value FROM nls_session_parameters WHERE parameter=:PARAM_NAME").await?;
        let row = stmt.query_single("NLS_DATE_FORMAT").await?.expect("one row");
        let fmt: &str = row.get(0)?;
        assert_eq!(fmt, "YYYY-MM-DD", "Session uses custom NLS_DATE_FORMAT");
    }
    # Ok(()) })
    # }
    ```
    */
    pub async fn get_tagged_session(&self, taginfo: &str) -> Result<(Session<'_>,bool)> {
        Session::from_session_pool(self, taginfo).await
    }
}
