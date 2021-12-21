//! Blocking mode User session (a.k.a. database connection) methods.

use super::{SvcCtx, Connection};
use crate::{Result, Statement, oci::{self, *, attr}, Environment, SessionPool, ConnectionPool};
use std::{marker::PhantomData, sync::Arc};

impl Drop for SvcCtx {
    fn drop(&mut self) {
        oci_session_release(&self.svc, &self.err);
    }
}

impl SvcCtx {
    pub(crate) fn new(env: &Environment, dblink: &str, user: &str, pass: &str) -> Result<Self> {
        let err = Handle::<OCIError>::new(env)?;
        let inf = Handle::<OCIAuthInfo>::new(env)?;
        inf.set_attr(OCI_ATTR_DRIVER_NAME, "sibyl", &err)?;
        inf.set_attr(OCI_ATTR_USERNAME, user, &err)?;
        inf.set_attr(OCI_ATTR_PASSWORD, pass, &err)?;
        let mut svc = Ptr::<OCISvcCtx>::null();
        let mut found = 0u8;
        oci::session_get(
            env.as_ref(), &err, svc.as_mut_ptr(), &inf, dblink.as_ptr(), dblink.len() as u32,
            &mut found, OCI_SESSGET_STMTCACHE
        )?;
        Ok(SvcCtx { env: env.get_env(), err, svc })
    }

    pub(crate) fn from_session_pool(pool: &SessionPool) -> Result<Self> {
        let env = pool.get_env();        
        let err = Handle::<OCIError>::new(env.as_ref())?;
        let svc = pool.get_svc_ctx()?;
        Ok(SvcCtx { env, err, svc })
    }

    pub(crate) fn from_connection_pool(pool: &ConnectionPool, user: &str, pass: &str) -> Result<Self> {
        let env = pool.get_env();
        let err = Handle::<OCIError>::new(env.as_ref())?;
        let svc = pool.get_svc_ctx(user, pass)?;
        Ok(SvcCtx { env, err, svc })
    }
}

impl<'a> Connection<'a> {
    pub(crate) fn new(env: &'a Environment, dblink: &str, user: &str, pass: &str) -> Result<Self> {
        let ctx = SvcCtx::new(env, dblink, user, pass)?;
        let usr : Ptr<OCISession> = attr::get(OCI_ATTR_SESSION, OCI_HTYPE_SVCCTX, ctx.svc.as_ref(), ctx.as_ref())?;
        let ctx = Arc::new(ctx);
        Ok(Self { ctx, usr, phantom_env: PhantomData })
    }

    pub(crate) fn from_session_pool(pool: &'a SessionPool) -> Result<Self> {
        let ctx = SvcCtx::from_session_pool(pool)?;
        let usr: Ptr<OCISession> = attr::get(OCI_ATTR_SESSION, OCI_HTYPE_SVCCTX, ctx.svc.as_ref(), ctx.as_ref())?;
        let ctx = Arc::new(ctx);
        Ok(Self { ctx, usr, phantom_env: PhantomData })
    }

    pub(crate) fn from_connection_pool(pool: &'a ConnectionPool, user: &str, pass: &str) -> Result<Self> {
        let ctx = SvcCtx::from_connection_pool(pool, user, pass)?;
        let usr: Ptr<OCISession> = attr::get(OCI_ATTR_SESSION, OCI_HTYPE_SVCCTX, ctx.svc.as_ref(), ctx.as_ref())?;
        let ctx = Arc::new(ctx);
        Ok(Self { ctx, usr, phantom_env: PhantomData })
    }

    /// Confirms that the connection and the server are active.
    pub fn ping(&self) -> Result<()> {
        oci::ping(self.as_ref(), self.as_ref())
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
        oci::trans_commit(self.as_ref(), self.as_ref())
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
        oci::trans_rollback(self.as_ref(), self.as_ref())
    }
}
