//! Nonblocking mode database session methods.

use std::{sync::{Arc, atomic::{AtomicUsize, Ordering}}, marker::PhantomData};

use crate::{oci::{self, *}, task, Environment, Result, pool::SessionPool, Statement};

use super::{SvcCtx, Session};

impl SvcCtx {
    async fn new(env: &Environment, dblink: &str, user: &str, pass: &str) -> Result<Self> {
        let err = Handle::<OCIError>::new(&env)?;
        let inf = Handle::<OCIAuthInfo>::new(&env)?;
        inf.set_attr(OCI_ATTR_DRIVER_NAME, "sibyl", &err)?;
        inf.set_attr(OCI_ATTR_USERNAME, user, &err)?;
        inf.set_attr(OCI_ATTR_PASSWORD, pass, &err)?;

        let env = env.get_env();
        let dblink = String::from(dblink);
        task::execute_blocking(move || -> Result<Self> {
            let mut svc = Ptr::<OCISvcCtx>::null();
            let mut found = oci::Aligned::new(0u8);
            oci::session_get(
                env.as_ref(), err.as_ref(), svc.as_mut_ptr(), inf.as_ref(),
                dblink.as_ptr(), dblink.len() as _,
                found.as_mut_ptr(), OCI_SESSGET_STMTCACHE
            )?;
            Ok(Self { svc, inf, err, env, spool: None, active_future: AtomicUsize::new(0) })
        }).await?
    }

    fn set_nonblocking_mode(&self) -> Result<()> {
        let srv: Ptr<OCIServer> = attr::get(OCI_ATTR_SERVER, OCI_HTYPE_SVCCTX, self.svc.as_ref(), self.err.as_ref())?;
        oci::attr_set(srv.as_ref(), OCI_HTYPE_SERVER, std::ptr::null(), 0, OCI_ATTR_NONBLOCKING_MODE, self.err.as_ref())
    }

    async fn from_session_pool(pool: &SessionPool<'_>) -> Result<Self> {
        let spool = pool.get_spool();
        let env = spool.get_env();
        let err = Handle::<OCIError>::new(env.as_ref())?;
        let inf = Handle::<OCIAuthInfo>::new(env.as_ref())?;

        task::execute_blocking(move || -> Result<Self> {
            let name = spool.get_name();
            let mut svc = Ptr::<OCISvcCtx>::null();
            let mut found = oci::Aligned::new(0u8);
            oci::session_get(
                env.as_ref(), err.as_ref(), svc.as_mut_ptr(), inf.as_ref(),
                name.as_ptr(), name.len() as _, found.as_mut_ptr(),
                OCI_SESSGET_SPOOL | OCI_SESSGET_PURITY_SELF
            )?;
            Ok(Self { svc, inf, err, env, spool: Some(spool), active_future: AtomicUsize::new(0) })
        }).await?
    }

    pub(crate) fn lock(&self, id: usize) -> bool {
        if let Err(current) = self.active_future.compare_exchange(0, id, Ordering::AcqRel, Ordering::Relaxed) {
            current == id
        } else {
            true
        }
    }

    pub(crate) fn unlock(&self) {
        self.active_future.store(0, Ordering::Release)
    }
}

impl<'a> Session<'a> {
    pub(crate) async fn new(env: &'a Environment, dblink: &str, user: &str, pass: &str) -> Result<Session<'a>> {
        let ctx = SvcCtx::new(env, dblink, user, pass).await?;
        ctx.set_nonblocking_mode()?;
        let usr: Ptr<OCISession> = attr::get(OCI_ATTR_SESSION, OCI_HTYPE_SVCCTX, ctx.svc.as_ref(), ctx.as_ref())?;
        let ctx = Arc::new(ctx);
        Ok(Self { ctx, usr, phantom_env: PhantomData })
    }

    pub(crate) async fn from_session_pool(pool: &'a SessionPool<'_>) -> Result<Session<'a>> {
        let ctx = SvcCtx::from_session_pool(pool).await?;
        ctx.set_nonblocking_mode()?;
        let usr: Ptr<OCISession> = attr::get(OCI_ATTR_SESSION, OCI_HTYPE_SVCCTX, ctx.svc.as_ref(), ctx.as_ref())?;
        let ctx = Arc::new(ctx);
        Ok(Self { ctx, usr, phantom_env: PhantomData })
    }

    /**
    Confirms that the connection and the server are active.

    # Example

    ```
    # sibyl::block_on(async {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
    # session.start_call_time_measurements()?;
    session.ping().await?;
    # let dt = session.call_time()?;
    # session.stop_call_time_measurements()?;
    # assert!(dt > 0);
    # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
    ```
    */
    pub async fn ping(&self) -> Result<()> {
        futures::Ping::new(self.get_svc()).await
    }

    /**
    Commits the current transaction.

    Current transaction is defined as the set of statements executed since
    the last commit or since the beginning of the user session.

    # Example

    ```
    # sibyl::block_on(async {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
    let stmt = session.prepare("
        UPDATE hr.employees
           SET salary = :new_salary
         WHERE employee_id = :emp_id
    ").await?;
    let num_updated_rows = stmt.execute((
        (":EMP_ID",     107 ),
        (":NEW_SALARY", 4200),
    )).await?;
    assert_eq!(num_updated_rows, 1);

    session.commit().await?;
    # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
    ```
    */
    pub async fn commit(&self) -> Result<()> {
        futures::TransCommit::new(self.get_svc()).await
    }

    /**
    Rolls back the current transaction. The modified or updated objects in
    the object cache for this transaction are also rolled back.

    # Example

    ```
    # sibyl::block_on(async {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
    let stmt = session.prepare("
        UPDATE hr.employees
           SET salary = ROUND(salary * 1.1)
         WHERE employee_id = :emp_id
    ").await?;
    let num_updated_rows = stmt.execute(107).await?;
    assert_eq!(num_updated_rows, 1);

    session.rollback().await?;
    # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
    ```
    */
    pub async fn rollback(&self) -> Result<()> {
        futures::TransRollback::new(self.get_svc()).await
    }

    /**
    Prepares SQL or PL/SQL statement for execution.

    # Parameters

    * `sql` - SQL or PL/SQL statement

    # Example

    ```
    # sibyl::block_on(async {
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    # let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
    let stmt = session.prepare("
        SELECT employee_id
          FROM (
                SELECT employee_id
                     , row_number() OVER (ORDER BY hire_date) AS hire_date_rank
                  FROM hr.employees
               )
         WHERE hire_date_rank = 1
    ").await?;
    let row = stmt.query_single(()).await?.unwrap();
    let id : u32 = row.get(0)?;
    assert_eq!(id, 102);
    # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
    ```
    */
    pub async fn prepare(&'a self, sql: &str) -> Result<Statement<'a>> {
        Statement::new(sql, self).await
    }
}

#[cfg(test)]
mod tests {
    use crate::{Environment, Result};

    #[test]
    fn async_connect_multi_thread_static_env() -> Result<()> {
        crate::block_on(async {
            use std::env;
            use once_cell::sync::OnceCell;

            static ORACLE : OnceCell<Environment> = OnceCell::new();
            let oracle = ORACLE.get_or_try_init(|| {
                Environment::new()
            })?;

            let dbname = env::var("DBNAME").expect("database name");
            let dbuser = env::var("DBUSER").expect("user name");
            let dbpass = env::var("DBPASS").expect("password");

            let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
            session.ping().await?;

            Ok(())
        })
    }

    /// Tests that `OCIEnv` is kept beyond `Environment` drop to have it
    /// available for `Session`'s async drop
    #[test]
    fn async_connect_single_thread() -> Result<()> {
        crate::block_on(async {
            use std::env;

            let oracle = Environment::new()?;

            let dbname = env::var("DBNAME").expect("database name");
            let dbuser = env::var("DBUSER").expect("user name");
            let dbpass = env::var("DBPASS").expect("password");

            let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
            session.start_call_time_measurements()?;
            session.ping().await?;
            let dt = session.call_time()?;
            session.stop_call_time_measurements()?;

            assert!(dt > 0);
            println!("dt={}", dt);
            Ok(())
        })
    }

    /// Tests that `OCIEnv` is kept beyond `Environment` drop to have it
    /// available for `Session`'s async drop
    #[test]
    fn async_connect_multi_thread_stack_env() -> Result<()> {
        crate::block_on(async {
            use std::env;

            let oracle = Environment::new()?;

            let dbname = env::var("DBNAME").expect("database name");
            let dbuser = env::var("DBUSER").expect("user name");
            let dbpass = env::var("DBPASS").expect("password");

            let session = oracle.connect(&dbname, &dbuser, &dbpass).await?;
            session.ping().await?;

            Ok(())
        })
    }
}