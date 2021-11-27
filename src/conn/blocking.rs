//! Blocking mode User session (a.k.a. database connection) methods.

use super::{connect, get_from_session_pool, get_from_connection_pool, Connection};
use crate::{Environment, Result, Statement, oci::{self, *}, env::Env};
// use libc::c_void;

impl Drop for Connection<'_> {
    fn drop(&mut self) {
        unsafe {
            OCISessionRelease(self.svc.get(), self.err.get(), std::ptr::null(), 0, OCI_DEFAULT);
        }
    }
}

impl<'a> Connection<'a> {
    pub(crate) fn new(env: &'a Environment, addr: &str, user: &str, pass: &str) -> Result<Self> {
        connect(env, addr, user, pass)
    }

    pub(crate) fn from_session_pool(env: &'a Environment, pool_name: &str) -> Result<Self> {
        get_from_session_pool(env, pool_name)
    }

    pub(crate) fn from_connection_pool(env: &'a Environment, pool_name: &str, user: &str, pass: &str) -> Result<Self> {
        get_from_connection_pool(env, pool_name, user, pass)
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
        let mut rows = stmt.query(&[])?;
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
