//! Nonblocking mode Connection methods.

use std::{sync::Arc, marker::PhantomData};

use crate::{oci::{self, *}, task, Environment, Result, pool::SessionPool, Statement};

use super::{SvcCtx, Connection};

impl Drop for SvcCtx {
    fn drop(&mut self) {
        let mut svc = Ptr::<OCISvcCtx>::null();
        svc.swap(&mut self.svc);
        let err = Handle::take_over(&mut self.err);
        let env = self.env.clone();
        task::spawn(oci::futures::SessionRelease::new(svc, err, env));
    }
}

impl SvcCtx {
    async fn new(env: &Environment, dblink: &str, user: &str, pass: &str) -> Result<Self> {
        let err = Handle::<OCIError>::new(&env)?;
        let inf = Handle::<OCIAuthInfo>::new(&env)?;
        inf.set_attr(OCI_ATTR_DRIVER_NAME, "sibyl", &err)?;
        inf.set_attr(OCI_ATTR_USERNAME, user, &err)?;
        inf.set_attr(OCI_ATTR_PASSWORD, pass, &err)?;

        let env_ptr = Ptr::<OCIEnv>::from(env.as_ref());
        let err_ptr = Ptr::<OCIError>::from(err.as_ref());
        let dblink_ptr = Ptr::new(dblink.as_ptr() as *mut u8);
        let dblink_len = dblink.len() as u32;
        let svc = task::spawn_blocking(move || -> Result<Ptr<OCISvcCtx>> {
            let mut svc = Ptr::<OCISvcCtx>::null();
            let mut found = 0u8;
            oci::session_get(
                &env_ptr, &err_ptr, svc.as_mut_ptr(), inf.as_ref(), dblink_ptr.as_ref() as _, dblink_len,
                &mut found, OCI_SESSGET_STMTCACHE
            )?;
            Ok(svc)
        }).await??;
        Ok(Self { env: env.get_env(), err, svc })
    }

    fn set_nonblocking_mode(&self) -> Result<()> {
        let srv: Ptr<OCIServer> = attr::get(OCI_ATTR_SERVER, OCI_HTYPE_SVCCTX, self.svc.as_ref(), self.err.as_ref())?;
        oci::attr_set(srv.as_ref(), OCI_HTYPE_SERVER, std::ptr::null(), 0, OCI_ATTR_NONBLOCKING_MODE, self.err.as_ref())
    }

    async fn from_session_pool(pool: &SessionPool<'_>) -> Result<Self> {
        let env = pool.get_env();
        let err = Handle::<OCIError>::new(env.as_ref())?;
        let svc = pool.get_svc_ctx().await?;
        Ok(Self { env, err, svc })
    }
}

impl<'a> Connection<'a> {
    pub(crate) async fn new(env: &'a Environment, dblink: &str, user: &str, pass: &str) -> Result<Connection<'a>> {
        let ctx = SvcCtx::new(env, dblink, user, pass).await?;
        ctx.set_nonblocking_mode()?;
        let usr: Ptr<OCISession> = attr::get(OCI_ATTR_SESSION, OCI_HTYPE_SVCCTX, ctx.svc.as_ref(), ctx.as_ref())?;
        let ctx = Arc::new(ctx);
        Ok(Self { ctx, usr, phantom_env: PhantomData })
    }

    pub(crate) async fn from_session_pool(pool: &'a SessionPool<'_>) -> Result<Connection<'a>> {
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
        # sibyl::current_thread_block_on(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("user name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        # conn.start_call_time_measurements()?;
        conn.ping().await?;
        # let dt = conn.call_time()?;
        # conn.stop_call_time_measurements()?;
        # assert!(dt > 0);
        # println!("dt={}", dt);
        # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
        ```
    */
    pub async fn ping(&self) -> Result<()> {
        oci::futures::Ping::new(self.as_ref(), self.as_ref()).await
    }

    /**
        Commits the current transaction.

        Current transaction is defined as the set of statements executed since
        the last commit or since the beginning of the user session.

        # Example

        ```
        # sibyl::current_thread_block_on(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("user name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        let stmt = conn.prepare("
            UPDATE hr.employees
               SET salary = :new_salary
             WHERE employee_id = :emp_id
        ").await?;
        let num_updated_rows = stmt.execute(&[
            &( ":EMP_ID",     107  ),
            &( ":NEW_SALARY", 4200 ),
        ]).await?;
        assert_eq!(num_updated_rows, 1);

        conn.commit().await?;
        # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
        ```
    */
    pub async fn commit(&self) -> Result<()> {
        oci::futures::TransCommit::new(self.as_ref(), self.as_ref()).await
    }

    /**
        Rolls back the current transaction. The modified or updated objects in
        the object cache for this transaction are also rolled back.

        # Example

        ```
        # sibyl::current_thread_block_on(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("user name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        let stmt = conn.prepare("
            UPDATE hr.employees
               SET salary = ROUND(salary * 1.1)
             WHERE employee_id = :emp_id
        ").await?;
        let num_updated_rows = stmt.execute(&[ &107 ]).await?;
        assert_eq!(num_updated_rows, 1);

        conn.rollback().await?;
        # Ok::<(),sibyl::Error>(()) }).expect("Ok from async");
        ```
    */
    pub async fn rollback(&self) -> Result<()> {
        oci::futures::TransRollback::new(self.as_ref(), self.as_ref()).await
    }

    /**
        Prepares SQL or PL/SQL statement for execution.

        # Example

        ```
        # sibyl::current_thread_block_on(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("user name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
        let stmt = conn.prepare("
            SELECT employee_id
              FROM (
                    SELECT employee_id
                         , row_number() OVER (ORDER BY hire_date) AS hire_date_rank
                      FROM hr.employees
                   )
             WHERE hire_date_rank = 1
        ").await?;
        let rows = stmt.query(&[]).await?;
        let row = rows.next().await?.expect("first (and only) row");
        // EMPLOYEE_ID is NOT NULL, so it always can be unwrapped safely
        let id : u32 = row.get(0)?.unwrap();
        assert_eq!(id, 102);
        // Only one row is expected
        assert!(rows.next().await?.is_none());
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
        crate::multi_thread_block_on(async {
            use std::env;
            use once_cell::sync::OnceCell;

            static ORACLE : OnceCell<Environment> = OnceCell::new();
            let oracle = ORACLE.get_or_try_init(|| {
                Environment::new()
            })?;

            let dbname = env::var("DBNAME").expect("database name");
            let dbuser = env::var("DBUSER").expect("user name");
            let dbpass = env::var("DBPASS").expect("password");

            let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
            conn.ping().await?;

            Ok(())
        })
    }

    /// Tests that `OCIEnv` is kept beyond `Environment` drop to have it
    /// available for `Connection`'s async drop
    #[test]
    fn async_connect_single_thread() -> Result<()> {
        crate::current_thread_block_on(async {
            use std::env;

            let oracle = Environment::new()?;

            let dbname = env::var("DBNAME").expect("database name");
            let dbuser = env::var("DBUSER").expect("user name");
            let dbpass = env::var("DBPASS").expect("password");

            let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
            conn.start_call_time_measurements()?;
            conn.ping().await?;
            let dt = conn.call_time()?;
            conn.stop_call_time_measurements()?;

            assert!(dt > 0);
            println!("dt={}", dt);
            Ok(())
        })
    }

    /// Tests that `OCIEnv` is kept beyond `Environment` drop to have it
    /// available for `Connection`'s async drop
    #[test]
    fn async_connect_multi_thread_stack_env() -> Result<()> {
        crate::multi_thread_block_on(async {
            use std::env;

            let oracle = Environment::new()?;

            let dbname = env::var("DBNAME").expect("database name");
            let dbuser = env::var("DBUSER").expect("user name");
            let dbpass = env::var("DBPASS").expect("password");

            let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;
            conn.ping().await?;

            Ok(())
        })
    }

}
