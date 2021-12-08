//! Nonblocking mode Connection methods.

use super::{Session, Connection};
use crate::{Result, env::Env, oci::{self, *}, task, Environment, Statement, ptr::ScopedPtr, SessionPool, ConnectionPool};
use std::{ptr, marker::PhantomData, sync::Arc};
use libc::c_void;

impl Drop for Session {
    fn drop(&mut self) {
        let mut svc = Ptr::null();
        svc.swap(&mut self.svc);
        let err = Handle::take_over(&mut self.err);
        let env = self.env.clone();
        task::spawn(oci::futures::SessionRelease::new(svc, err, env));
    }
}

impl Session {
    pub(crate) async fn new(env: &Environment, dblink: &str, user: &str, pass: &str) -> Result<Self> {
        let err = Handle::<OCIError>::new(env.env_ptr())?;
        let inf = Handle::<OCIAuthInfo>::new(env.env_ptr())?;
        inf.set_attr(OCI_ATTR_DRIVER_NAME, "sibyl", err.get())?;
        inf.set_attr(OCI_ATTR_USERNAME, user, err.get())?;
        inf.set_attr(OCI_ATTR_PASSWORD, pass, err.get())?;

        let env_ptr = env.get_env_ptr();
        let err_ptr = err.get_ptr();
        let dblink_ptr = ScopedPtr::new(dblink.as_ptr());
        let dblink_len = dblink.len() as u32;
        let svc = task::spawn_blocking(move || -> Result<Ptr<OCISvcCtx>> {
            let mut svc = Ptr::null();
            let mut found = 0u8;
            oci::session_get(
                env_ptr.get(), err_ptr.get(), svc.as_mut_ptr(), inf.get(), dblink_ptr.get(), dblink_len,
                ptr::null(), 0, ptr::null_mut(), ptr::null_mut(), &mut found,
                OCI_SESSGET_STMTCACHE
            )?;
            Ok(svc)
        }).await??;
        Ok(Self { env: env.clone_env(), err, svc })
    }

    pub(crate) async fn from_session_pool(pool: &SessionPool<'_>) -> Result<Self> {
        let env = pool.clone_env();        
        let err = Handle::<OCIError>::new(env.get())?;
        let svc = pool.get_svc_ctx().await?;
        Ok(Session { env, err, svc })
    }

    pub(crate) async fn from_connection_pool(pool: &ConnectionPool<'_>, user: &str, pass: &str) -> Result<Self> {
        let env = pool.clone_env();
        let err = Handle::<OCIError>::new(env.get())?;
        let svc = pool.get_svc_ctx(user, pass).await?;
        Ok(Session { env, err, svc })
    }
}

impl<'a> Connection<'a> {
    pub(crate) async fn new(env: &'a Environment, dblink: &str, user: &str, pass: &str) -> Result<Connection<'a>> {
        let session = Session::new(env, dblink, user, pass).await?;
        let session = Arc::new(session);
        let usr = attr::get::<Ptr<OCISession>>(OCI_ATTR_SESSION, OCI_HTYPE_SVCCTX, session.svc_ptr() as *const c_void, session.err_ptr())?;
        Ok(Self { session, usr, phantom_env: PhantomData })
    }

    pub(crate) async fn from_session_pool(pool: &'a SessionPool<'_>) -> Result<Connection<'a>> {
        let session = Session::from_session_pool(pool).await?;
        let session = Arc::new(session);
        let usr = attr::get::<Ptr<OCISession>>(OCI_ATTR_SESSION, OCI_HTYPE_SVCCTX, session.svc_ptr() as *const c_void, session.err_ptr())?;
        Ok(Self { session, usr, phantom_env: PhantomData })
    }

    pub(crate) async fn from_connection_pool(pool: &'a ConnectionPool<'_>, user: &str, pass: &str) -> Result<Connection<'a>> {
        let session = Session::from_connection_pool(pool, user, pass).await?;
        let session = Arc::new(session);
        let usr = attr::get::<Ptr<OCISession>>(OCI_ATTR_SESSION, OCI_HTYPE_SVCCTX, session.svc_ptr() as *const c_void, session.err_ptr())?;
        Ok(Self { session, usr, phantom_env: PhantomData })
    }

    /// Confirms that the connection and the server are active.
    pub async fn ping(&self) -> Result<()> {
        oci::futures::Ping::new(Ptr::new(self.svc_ptr()), Ptr::new(self.err_ptr())).await
    }

    /**
        Prepares SQL or PL/SQL statement for execution.

        # Example

        ```
        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
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

    /**
        Commits the current transaction.

        Current transaction is defined as the set of statements executed since
        the last commit or since the beginning of the user session.

        # Example

        ```
        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
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
        oci::futures::TransCommit::new(Ptr::new(self.svc_ptr()), Ptr::new(self.err_ptr())).await
    }

    /**
        Rolls back the current transaction. The modified or updated objects in
        the object cache for this transaction are also rolled back.

        # Example

        ```
        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
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
        oci::futures::TransRollback::new(Ptr::new(self.svc_ptr()), Ptr::new(self.err_ptr())).await
    }
}
