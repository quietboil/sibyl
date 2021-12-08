//! Blocking mode User session (a.k.a. database connection) methods.

use super::{Session, Connection};
use crate::{Result, Statement, env::Env, oci::{self, *, attr}, Environment, SessionPool, ConnectionPool};
use std::{ptr, marker::PhantomData, sync::Arc};
use libc::c_void;


impl Drop for Session {
    fn drop(&mut self) {
        unsafe {
            OCISessionRelease(self.svc.get(), self.err.get(), std::ptr::null(), 0, OCI_DEFAULT);
        }
    }
}

impl Session {
    pub(crate) fn new(env: &Environment, dblink: &str, user: &str, pass: &str) -> Result<Self> {
        let err = Handle::<OCIError>::new(env.env_ptr())?;
        let inf = Handle::<OCIAuthInfo>::new(env.env_ptr())?;
        inf.set_attr(OCI_ATTR_DRIVER_NAME, "sibyl", err.get())?;
        inf.set_attr(OCI_ATTR_USERNAME, user, err.get())?;
        inf.set_attr(OCI_ATTR_PASSWORD, pass, err.get())?;
        let mut svc = Ptr::null();
        let mut found = 0u8;
        oci::session_get(
            env.env_ptr(), err.get(), svc.as_mut_ptr(), inf.get(), dblink.as_ptr(), dblink.len() as u32,
            ptr::null(), 0, ptr::null_mut(), ptr::null_mut(), &mut found,
            OCI_SESSGET_STMTCACHE
        )?;
        Ok(Session { env: env.clone_env(), err, svc })
    }

    pub(crate) fn from_session_pool(pool: &SessionPool) -> Result<Self> {
        let env = pool.clone_env();        
        let err = Handle::<OCIError>::new(env.get())?;
        let svc = pool.get_svc_ctx()?;
        Ok(Session { env, err, svc })
    }

    pub(crate) fn from_connection_pool(pool: &ConnectionPool, user: &str, pass: &str) -> Result<Self> {
        let env = pool.clone_env();
        let err = Handle::<OCIError>::new(env.get())?;
        let svc = pool.get_svc_ctx(user, pass)?;
        Ok(Session { env, err, svc })
    }
}

impl<'a> Connection<'a> {
    pub(crate) fn new(env: &'a Environment, dblink: &str, user: &str, pass: &str) -> Result<Self> {
        let session = Session::new(env, dblink, user, pass)?;
        let session = Arc::new(session);
        let usr = attr::get::<Ptr<OCISession>>(OCI_ATTR_SESSION, OCI_HTYPE_SVCCTX, session.svc_ptr() as *const c_void, session.err_ptr())?;
        Ok(Self { session, usr, phantom_env: PhantomData })
    }

    pub(crate) fn from_session_pool(pool: &'a SessionPool) -> Result<Self> {
        let session = Session::from_session_pool(pool)?;
        let session = Arc::new(session);
        let usr = attr::get::<Ptr<OCISession>>(OCI_ATTR_SESSION, OCI_HTYPE_SVCCTX, session.svc_ptr() as *const c_void, session.err_ptr())?;
        Ok(Self { session, usr, phantom_env: PhantomData })
    }

    pub(crate) fn from_connection_pool(pool: &'a ConnectionPool, user: &str, pass: &str) -> Result<Self> {
        let session = Session::from_connection_pool(pool, user, pass)?;
        let session = Arc::new(session);
        let usr = attr::get::<Ptr<OCISession>>(OCI_ATTR_SESSION, OCI_HTYPE_SVCCTX, session.svc_ptr() as *const c_void, session.err_ptr())?;
        Ok(Self { session, usr, phantom_env: PhantomData })
    }

    /// Confirms that the connection and the server are active.
    pub fn ping(&self) -> Result<()> {
        oci::ping(self.svc_ptr(), self.err_ptr(), OCI_DEFAULT)
    }

    /**
        Prepares SQL or PL/SQL statement for execution.

        ## Example
        ```
        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            SELECT employee_id
              FROM (
                    SELECT employee_id
                         , row_number() OVER (ORDER BY hire_date) AS hire_date_rank
                      FROM hr.employees
                   )
             WHERE hire_date_rank = 1
        ")?;
        let rows = stmt.query(&[])?;
        let row = rows.next()?.expect("first (and only) row");
        // EMPLOYEE_ID is NOT NULL, so it can be unwrapped safely
        let id : u32 = row.get(0)?.unwrap();
        assert_eq!(id, 102);
        assert!(rows.next()?.is_none());
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn prepare(&self, sql: &str) -> Result<Statement> {
        Statement::new(sql, self)
    }

    /**
        Commits the current transaction.

        Current transaction is defined as the set of statements executed since
        the last commit or since the beginning of the user session.

        ## Example
        ```
        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            UPDATE hr.employees
               SET salary = :new_salary
             WHERE employee_id = :emp_id
        ")?;
        let num_updated_rows = stmt.execute(&[
            &( ":EMP_ID",     107  ),
            &( ":NEW_SALARY", 4200 ),
        ])?;
        assert_eq!(num_updated_rows, 1);

        conn.commit()?;
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn commit(&self) -> Result<()> {
        oci::trans_commit(self.svc_ptr(), self.err_ptr(), OCI_DEFAULT)
    }

    /**
        Rolls back the current transaction. The modified or updated objects in
        the object cache for this transaction are also rolled back.

        ## Example
        ```
        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let stmt = conn.prepare("
            UPDATE hr.employees
               SET salary = ROUND(salary * 1.1)
             WHERE employee_id = :emp_id
        ")?;
        let num_updated_rows = stmt.execute(&[ &107 ])?;
        assert_eq!(num_updated_rows, 1);

        conn.rollback()?;
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn rollback(&self) -> Result<()> {
        oci::trans_rollback(self.svc_ptr(), self.err_ptr(), OCI_DEFAULT)
    }
}
