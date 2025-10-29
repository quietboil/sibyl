//! Blocking mode database session methods.

use parking_lot::RwLock;

use super::{SvcCtx, Session};
use crate::{ConnectionPool, Environment, Result, SessionPool, Statement, oci::{self, attr, *}, session::SessionTagInfo};
use std::{marker::PhantomData, sync::Arc};

impl SvcCtx {
    pub(crate) fn new(env: &Environment, dblink: &str, user: &str, pass: &str, mode: u32) -> Result<Self> {
        let err = Handle::<OCIError>::new(env)?;
        let inf = Handle::<OCIAuthInfo>::new(env)?;
        inf.set_attr(OCI_ATTR_DRIVER_NAME, "sibyl", &err)?;
        inf.set_attr(OCI_ATTR_USERNAME, user, &err)?;
        inf.set_attr(OCI_ATTR_PASSWORD, pass, &err)?;
        let mut svc = Ptr::<OCISvcCtx>::null();
        oci::session_get(
            env.as_ref(), &err, svc.as_mut_ptr(), &inf, dblink.as_ptr(), dblink.len() as u32,
            mode | OCI_SESSGET_STMTCACHE
        )?;
        Ok(SvcCtx { env: env.get_env(), err, inf, svc, spool: None, tag: None  })
    }

    pub(crate) fn from_session_pool(pool: &SessionPool, tag: &str) -> Result<(Self,bool)> {
        let env = pool.get_env();
        let err = Handle::<OCIError>::new(env.as_ref())?;
        let inf = Handle::<OCIAuthInfo>::new(env.as_ref())?;
        let spool = pool.get_spool();
        let pool_name = spool.get_name();

        let mut svc = Ptr::<OCISvcCtx>::null();
        let tag_mode = if tag.len() > 0 && tag.find('=').is_some() { OCI_SESSGET_MULTIPROPERTY_TAG } else { OCI_DEFAULT };
        let mut ret_tag: *const u8 = std::ptr::null();
        let mut ret_tag_len: u32 = 0;

        let mut found = oci::Aligned::new(0u8);
        oci::session_get_tagged(
            &env, &err, svc.as_mut_ptr(), &inf,
            pool_name.as_ptr(), pool_name.len() as _,
            tag.as_ptr(), tag.len() as _, &mut ret_tag, &mut ret_tag_len,
            found.as_mut_ptr(), tag_mode | OCI_SESSGET_SPOOL | OCI_SESSGET_PURITY_SELF
        )?;
        let found = <u8>::from(found) != 0;
        let svc_ctx = Self { svc, inf, err, env,
            spool: Some(spool),
            tag: Some(RwLock::new(SessionTagInfo {
                tag_ptr: Ptr::new(ret_tag),
                tag_len: ret_tag_len as _,
                new_tag: String::new(),
            }))
        };
        Ok((svc_ctx, found))
    }

    pub(crate) fn from_connection_pool(pool: &ConnectionPool, username: &str, password: &str) -> Result<Self> {
        let env = pool.get_env();
        let err = Handle::<OCIError>::new(env.as_ref())?;
        let inf = Handle::<OCIAuthInfo>::new(env.as_ref())?;
        inf.set_attr(OCI_ATTR_DRIVER_NAME, "sibyl", &err)?;
        inf.set_attr(OCI_ATTR_USERNAME, username, &err)?;
        inf.set_attr(OCI_ATTR_PASSWORD, password, &err)?;

        let svc = pool.get_svc_ctx(&inf)?;
        Ok(SvcCtx { env, err, inf, svc, spool: None, tag: None })
    }
}

impl<'a> Session<'a> {
    pub(crate) fn new(env: &'a Environment, dblink: &str, user: &str, pass: &str, mode: u32) -> Result<Self> {
        let ctx = SvcCtx::new(env, dblink, user, pass, mode)?;
        let usr : Ptr<OCISession> = attr::get(OCI_ATTR_SESSION, OCI_HTYPE_SVCCTX, ctx.svc.as_ref(), ctx.as_ref())?;
        let ctx = Arc::new(ctx);
        Ok(Self { ctx, usr, phantom_env: PhantomData })
    }

    pub(crate) fn from_session_pool(pool: &'a SessionPool, tag: &str) -> Result<(Self,bool)> {
        let (ctx, found) = SvcCtx::from_session_pool(pool, tag)?;
        let usr: Ptr<OCISession> = attr::get(OCI_ATTR_SESSION, OCI_HTYPE_SVCCTX, ctx.svc.as_ref(), ctx.as_ref())?;
        let ctx = Arc::new(ctx);
        Ok((Self { ctx, usr, phantom_env: PhantomData }, found))
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
    pub fn prepare(&self, sql: &str) -> Result<Statement<'_>> {
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
