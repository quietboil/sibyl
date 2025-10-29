//! Session pool blocking mode implementation

use super::{SessionPool, SPool};
use crate::{Result, oci::{self, *}, Environment, Session};
use std::{ptr, marker::PhantomData, sync::Arc};

impl SPool {
    pub(crate) fn new(env: &Environment, dbname: &str, username: &str, password: &str, min: usize, inc: usize, max: usize, session_state_fixup_callback: &str) -> Result<Self> {
        let err = Handle::<OCIError>::new(env)?;
        let pool = Handle::<OCISPool>::new(env)?;
        let info = Handle::<OCIAuthInfo>::new(env)?;
        info.set_attr(OCI_ATTR_DRIVER_NAME, "sibyl", &err)?;
        if session_state_fixup_callback.len() > 0 {
            info.set_attr(OCI_ATTR_FIXUP_CALLBACK, session_state_fixup_callback, &err)?;
        }
        pool.set_attr(OCI_ATTR_SPOOL_AUTH, info.get_ptr(), &err)?;

        let mut pool_name_ptr = ptr::null::<u8>();
        let mut pool_name_len = 0u32;
        oci::session_pool_create(
            env.as_ref(), &err, &pool,
            &mut pool_name_ptr, &mut pool_name_len,
            dbname.as_ptr(), dbname.len() as u32,
            min as u32, max as u32, inc as u32,
            username.as_ptr(), username.len() as u32,
            password.as_ptr(), password.len() as u32,
            OCI_SPC_HOMOGENEOUS | OCI_SPC_STMTCACHE
        )?;
        let name = unsafe { std::slice::from_raw_parts(pool_name_ptr, pool_name_len as usize) };
        let name = name.to_vec();
        Ok(Self {env: env.get_env(), err, info, pool, name})
    }
}

impl<'a> SessionPool<'a> {
    pub(crate) fn new(env: &'a Environment, dbname: &str, username: &str, password: &str, min: usize, inc: usize, max: usize, session_state_fixup_callback: &str) -> Result<Self> {
        let inner = SPool::new(env, dbname, username, password, min, inc, max, session_state_fixup_callback)?;
        let inner = Arc::new(inner);
        Ok(Self { inner, phantom_env: PhantomData })
    }

    /**
        Returns a default (untagged) session from this session pool.

        # Example

        ```
        use sibyl::{Environment, Session, Date, Result};
        use once_cell::sync::OnceCell;
        use std::{env, thread, sync::Arc};

        fn main() -> Result<()> {
            static ORACLE : OnceCell<Environment> = OnceCell::new();
            let oracle = ORACLE.get_or_try_init(|| {
                Environment::new()
            })?;

            let dbname = env::var("DBNAME").expect("database name");
            let dbuser = env::var("DBUSER").expect("user name");
            let dbpass = env::var("DBPASS").expect("password");

            let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 3)?;
            let pool = Arc::new(pool);

            let mut workers = Vec::with_capacity(10);
            while workers.len() < 10 {
                let pool = pool.clone();
                let handle = thread::spawn(move || -> String {

                    let session = pool.get_session().expect("database session");

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

        fn select_latest_hire(session: &Session) -> Result<String> {
            let stmt = session.prepare("
                SELECT first_name, last_name, hire_date
                  FROM (
                        SELECT first_name, last_name, hire_date
                             , Row_Number() OVER (ORDER BY hire_date DESC, last_name) AS hire_date_rank
                          FROM hr.employees
                       )
                 WHERE hire_date_rank = 1
            ")?;
            if let Some( row ) = stmt.query_single(())? {
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
    pub fn get_session(&self) -> Result<Session<'_>> {
        let (session, _found) = Session::from_session_pool(self, "")?;
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
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 5)?;

    {
        let (session,found) = pool.get_tagged_session("CUSTOM")?;
        assert!(!found, "a default (not yet tagged) session was returned");
        if !found {
    #        let stmt = session.prepare("SELECT value FROM nls_session_parameters WHERE parameter=:PARAM_NAME")?;
    #        let row = stmt.query_single("NLS_DATE_FORMAT")?.expect("one row");
    #        let fmt: &str = row.get(0)?;
    #        assert_ne!(fmt, "YYYY-MM-DD", "Default NLS_DATE_FORMAT differs from what we want to set");
            let stmt = session.prepare("ALTER SESSION SET NLS_DATE_FORMAT='YYYY-MM-DD'")?;
            stmt.execute(())?;
            session.set_tag("CUSTOM")?;
        }
    } // session is released back to the pool here

    {
        let (session,found) = pool.get_tagged_session("CUSTOM")?;
        assert!(found, "the customized session was returned");

        let stmt = session.prepare("SELECT value FROM nls_session_parameters WHERE parameter=:PARAM_NAME")?;
        let row = stmt.query_single("NLS_DATE_FORMAT")?.expect("one row");
        let fmt: &str = row.get(0)?;
        assert_eq!(fmt, "YYYY-MM-DD", "Session uses custom NLS_DATE_FORMAT");
    }
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn get_tagged_session(&self, taginfo: &str) -> Result<(Session<'_>,bool)> {
        Session::from_session_pool(self, taginfo)
    }
}
