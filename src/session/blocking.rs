//! Blocking mode database session methods.

use super::{SvcCtx, Session};
use crate::{Result, Statement, oci::{self, *, attr}, Environment, SessionPool, ConnectionPool};
use std::{marker::PhantomData, sync::Arc};

impl SvcCtx {
    pub(crate) fn new(env: &Environment, dblink: &str, user: &str, pass: &str) -> Result<Self> {
        let err = Handle::<OCIError>::new(env)?;
        let inf = Handle::<OCIAuthInfo>::new(env)?;
        inf.set_attr(OCI_ATTR_DRIVER_NAME, "sibyl", &err)?;
        inf.set_attr(OCI_ATTR_USERNAME, user, &err)?;
        inf.set_attr(OCI_ATTR_PASSWORD, pass, &err)?;
        let mut svc = Ptr::<OCISvcCtx>::null();
        let mut found = oci::Aligned::new(0u8);
        oci::session_get(
            env.as_ref(), &err, svc.as_mut_ptr(), &inf, dblink.as_ptr(), dblink.len() as u32,
            found.as_mut_ptr(), OCI_SESSGET_STMTCACHE
        )?;
        Ok(SvcCtx { env: env.get_env(), err, inf, svc, spool: None })
    }

    pub(crate) fn from_session_pool(pool: &SessionPool) -> Result<Self> {
        let env = pool.get_env();
        let err = Handle::<OCIError>::new(env.as_ref())?;
        let inf = Handle::<OCIAuthInfo>::new(env.as_ref())?;
        let svc = pool.get_svc_ctx(&inf)?;
        Ok(Self { svc, inf, err, env, spool: Some(pool.get_spool()) })
    }

    pub(crate) fn from_connection_pool(pool: &ConnectionPool, username: &str, password: &str) -> Result<Self> {
        let env = pool.get_env();
        let err = Handle::<OCIError>::new(env.as_ref())?;
        let inf = Handle::<OCIAuthInfo>::new(env.as_ref())?;
        inf.set_attr(OCI_ATTR_DRIVER_NAME, "sibyl", &err)?;
        inf.set_attr(OCI_ATTR_USERNAME, username, &err)?;
        inf.set_attr(OCI_ATTR_PASSWORD, password, &err)?;

        let svc = pool.get_svc_ctx(&inf)?;
        Ok(SvcCtx { env, err, inf, svc, spool: None })
    }
}

impl<'a> Session<'a> {
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

    /**
    Confirms that the connection and the server are active.

    # Example

    ```
    # let session = sibyl::test_env::get_session()?;
    # session.start_call_time_measurements()?;
    session.ping()?;
    # let dt = session.call_time()?;
    # session.stop_call_time_measurements()?;
    # assert!(dt > 0);
    # Ok::<(),sibyl::Error>(())
    ```
    */
    pub fn ping(&self) -> Result<()> {
        oci::ping(self.as_ref(), self.as_ref())
    }

    /**
    Prepares SQL or PL/SQL statement for execution.

    # Parameters

    * `sql` - SQL or PL/SQL statement

    # Example

    ```
    # let session = sibyl::test_env::get_session()?;
    let stmt = session.prepare("
        SELECT employee_id
          FROM (
                SELECT employee_id
                     , row_number() OVER (ORDER BY hire_date) AS hire_date_rank
                  FROM hr.employees
               )
         WHERE hire_date_rank = 1
    ")?;
    let row = stmt.query_single(())?.unwrap();
    let id : u32 = row.get(0)?;
    assert_eq!(id, 102);
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

    # Example

    ```
    # let session = sibyl::test_env::get_session()?;
    let stmt = session.prepare("
        UPDATE hr.employees
           SET salary = :new_salary
         WHERE employee_id = :emp_id
    ")?;
    let num_updated_rows = stmt.execute((
        (":EMP_ID",     107 ),
        (":NEW_SALARY", 4200),
    ))?;
    assert_eq!(num_updated_rows, 1);

    session.commit()?;
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn commit(&self) -> Result<()> {
        oci::trans_commit(self.as_ref(), self.as_ref())
    }

    /**
    Rolls back the current transaction. The modified or updated objects in
    the object cache for this transaction are also rolled back.

    # Example

    ```
    # let session = sibyl::test_env::get_session()?;
    let stmt = session.prepare("
        UPDATE hr.employees
           SET salary = ROUND(salary * 1.1)
         WHERE employee_id = :emp_id
    ")?;
    let num_updated_rows = stmt.execute(107)?;
    assert_eq!(num_updated_rows, 1);

    session.rollback()?;
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn rollback(&self) -> Result<()> {
        oci::trans_rollback(self.as_ref(), self.as_ref())
    }
}
