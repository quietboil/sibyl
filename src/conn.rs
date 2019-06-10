//! User Session

use crate::*;
use crate::env::Env;

// Server Handle Attribute Values
// const OCI_SERVER_NOT_CONNECTED  : u32 = 0;
const OCI_SERVER_NORMAL : u32 = 1;

// Credential Types
const OCI_CRED_RDBMS    : u32 = 1;
const OCI_CRED_EXT      : u32 = 2;

// Attributes
const OCI_ATTR_CURRENT_SCHEMA           : u32 = 224;
const OCI_ATTR_CLIENT_IDENTIFIER        : u32 = 278;
const OCI_ATTR_MODULE                   : u32 = 366;
const OCI_ATTR_ACTION                   : u32 = 367;
const OCI_ATTR_CLIENT_INFO              : u32 = 368;
const OCI_ATTR_COLLECT_CALL_TIME        : u32 = 369;
const OCI_ATTR_CALL_TIME                : u32 = 370;
const OCI_ATTR_DRIVER_NAME              : u32 = 424;
const OCI_ATTR_DEFAULT_LOBPREFETCH_SIZE : u32 = 438;

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/connect-authorize-and-initialize-functions.html#GUID-B6291228-DA2F-4CE9-870A-F94243141757
    fn OCIServerAttach(
        srvhp:      *mut OCIServer,
        errhp:      *mut OCIError,
        dblink:     *const u8,
        dblink_len: u32,
        mode:       u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/connect-authorize-and-initialize-functions.html#GUID-402B540A-05FF-464B-B9C8-B2E7B4ABD564
    fn OCIServerDetach(
        srvhp:      *mut OCIServer,
        errhp:      *mut OCIError,
        mode:       u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/connect-authorize-and-initialize-functions.html#GUID-31B1FDB3-056E-4AF9-9B89-8DA6AA156947
    fn OCISessionBegin(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        userhp:     *mut OCISession,
        credt:      u32,
        mode:       u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/connect-authorize-and-initialize-functions.html#GUID-2AE88BDC-2C44-4958-B26A-434B0407F06F
    fn OCISessionEnd(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        userhp:     *mut OCISession,
        mode:       u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/transaction-functions.html#GUID-DDAE3122-8769-4A30-8D78-EB2A3CCF77D4
    fn OCITransCommit(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        flags:      u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/transaction-functions.html#GUID-06EF9A0A-01A3-40CE-A0B7-DF0504A93366
    fn OCITransRollback(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        flags:      u32
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/miscellaneous-functions.html#GUID-033BF96D-D88D-4F18-909A-3AB7C2F6C70F
    fn OCIPing(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        mode:       u32
    ) -> i32;
}

/// Represents a user session
pub struct Connection<'e> {
    state: ConnState,
    usr: Handle<OCISession>,
    svc: Handle<OCISvcCtx>,
    srv: Handle<OCIServer>,
    env: &'e dyn Env,
}

impl Env for Connection<'_> {
    fn env_ptr(&self) -> *mut OCIEnv      { self.env.env_ptr() }
    fn err_ptr(&self) -> *mut OCIError    { self.env.err_ptr() }
}

/// A trait for types that can provides access to `Connection` handles
pub trait Conn : Env {
    fn srv_ptr(&self) -> *mut OCIServer;
    fn svc_ptr(&self) -> *mut OCISvcCtx;
    fn usr_ptr(&self) -> *mut OCISession;
}

impl Conn for Connection<'_> {
    fn srv_ptr(&self) -> *mut OCIServer   { self.srv.get() }
    fn svc_ptr(&self) -> *mut OCISvcCtx   { self.svc.get() }
    fn usr_ptr(&self) -> *mut OCISession  { self.usr.get() }
}

enum ConnState {
    Detached,
    Attached,
    Session
}

impl Drop for Connection<'_> {
    fn drop(&mut self) {
        if let ConnState::Session = self.state {
            unsafe {
                OCISessionEnd(self.svc_ptr(), self.err_ptr(), self.usr_ptr(), OCI_DEFAULT);
            }
            self.state = ConnState::Attached;
        }
        if let ConnState::Attached = self.state {
            unsafe {
                OCIServerDetach(self.srv.get(), self.err_ptr(), OCI_DEFAULT);
            }
        }
    }
}

impl<'e> Connection<'e> {
    pub(crate) fn new(env: &'e dyn Env) -> Result<Self> {
        let srv: Handle<OCIServer>  = Handle::new(env.env_ptr())?;
        let svc: Handle<OCISvcCtx>  = Handle::new(env.env_ptr())?;
        let usr: Handle<OCISession> = Handle::new(env.env_ptr())?;
        Ok( Self { env, srv, svc, usr, state: ConnState::Detached } )
    }

    pub(crate) fn attach(&mut self, addr: &str) -> Result<()> {
        if let ConnState::Detached = self.state {} else {
            return Err( Error::new("already attached") );
        }
        catch!{self.err_ptr() =>
            OCIServerAttach(self.srv_ptr(), self.err_ptr(), addr.as_ptr(), addr.len() as u32, OCI_DEFAULT)
        }
        self.state = ConnState::Attached;
        self.svc.set_attr(OCI_ATTR_SERVER, self.srv.get(), self.err_ptr())?;
        Ok(())
    }

    pub(crate) fn login(&mut self, user: &str, pass: &str) -> Result<()> {
        match self.state {
            ConnState::Detached => return Err( Error::new("not attached to the server") ),
            ConnState::Attached => { /* the expected state */ },
            ConnState::Session  => return Err( Error::new("user session is active") ),
        }
        self.usr.set_attr(OCI_ATTR_DRIVER_NAME, "sibyl", self.err_ptr())?;
        self.usr.set_attr(OCI_ATTR_USERNAME, user, self.err_ptr())?;
        self.usr.set_attr(OCI_ATTR_PASSWORD, pass, self.err_ptr())?;
        let cred = if user.len() == 0 && pass.len() == 0 { OCI_CRED_EXT } else { OCI_CRED_RDBMS };
        catch!{self.err_ptr() =>
            OCISessionBegin(self.svc_ptr(), self.err_ptr(), self.usr_ptr(), cred, OCI_DEFAULT)
        }
        self.state = ConnState::Session;
        self.svc.set_attr(OCI_ATTR_SESSION, self.usr_ptr(), self.err_ptr())?;
        Ok(())
    }

    /// Reports whether self is connected to the server
    pub fn is_connected(&self) -> Result<bool> {
        let status : u32 = self.srv.get_attr(OCI_ATTR_SERVER_STATUS, self.err_ptr())?;
        Ok(status == OCI_SERVER_NORMAL)
    }

    /// Confirms that the connection and the server are active.
    pub fn ping(&self) -> Result<()> {
        catch!{self.err_ptr() =>
            OCIPing(self.svc_ptr(), self.err_ptr(), OCI_DEFAULT)
        }
        Ok(())
    }

    /// Reports whether connection is established in non-blocking mode.
    pub fn is_async(&self) -> Result<bool> {
        let mode : u8 = self.srv.get_attr(OCI_ATTR_NONBLOCKING_MODE, self.err_ptr())?;
        Ok(mode != 0)
    }

    /// Prepares SQL or PL/SQL statement for execution
    /// ## Example
    /// ```
    /// # let dbname = std::env::var("DBNAME")?;
    /// # let dbuser = std::env::var("DBUSER")?;
    /// # let dbpass = std::env::var("DBPASS")?;
    /// # let oracle = sibyl::env()?;
    /// # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    /// let stmt = conn.prepare("
    ///     SELECT employee_id
    ///       FROM (
    ///             SELECT employee_id, row_number() OVER (ORDER BY hire_date) ord
    ///               FROM hr.employees
    ///            )
    ///      WHERE ord = 1
    /// ")?;
    /// let rows = stmt.query(&[])?;
    /// let optrow = rows.next()?;
    /// assert!(optrow.is_some());
    /// if let Some( row ) = optrow {
    ///     // EMPLOYEE_ID is NOT NULL, so it can be unwrapped safely
    ///     let id : usize = row.get(0)?.unwrap();
    ///
    ///     assert_eq!(id, 102);
    /// }
    /// let optrow = rows.next()?;
    /// assert!(optrow.is_none());
    /// # Ok::<(),Box<dyn std::error::Error>>(())
    /// ```
    pub fn prepare(&self, sql: &str) -> Result<Statement> {
        Statement::new(sql, self as &dyn Conn)
    }

    /// Commits the current transaction.
    ///
    /// Current transaction is defined as the set of statements executed since
    /// the last commit or since beginning of the user session.
    ///
    /// ## Example
    /// ```
    /// # let dbname = std::env::var("DBNAME")?;
    /// # let dbuser = std::env::var("DBUSER")?;
    /// # let dbpass = std::env::var("DBPASS")?;
    /// # let oracle = sibyl::env()?;
    /// # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    /// let stmt = conn.prepare("
    ///     UPDATE hr.employees
    ///        SET salary = :new_salary
    ///      WHERE employee_id = :id
    /// ")?;
    /// let num_updated_rows = stmt.execute(&[
    ///     &( ":id",         107  ),
    ///     &( ":new_salary", 4200 ),
    /// ])?;
    /// assert_eq!(1, num_updated_rows);
    ///
    /// conn.commit()?;
    /// # Ok::<(),Box<dyn std::error::Error>>(())
    /// ```
    pub fn commit(&self) -> Result<()> {
        catch!{self.err_ptr() =>
            OCITransCommit(self.svc_ptr(), self.err_ptr(), OCI_DEFAULT)
        }
        Ok(())
    }

    /// Rolls back the current transaction. The modified or updated objects in
    /// the object cache for this transaction are also rolled back.
    ///
    /// Current transaction is defined as the set of statements executed since
    /// the last commit or since beginning of the user session.
    ///
    /// ## Example
    /// ```
    /// # let dbname = std::env::var("DBNAME")?;
    /// # let dbuser = std::env::var("DBUSER")?;
    /// # let dbpass = std::env::var("DBPASS")?;
    /// # let oracle = sibyl::env()?;
    /// # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    /// let stmt = conn.prepare("
    ///     UPDATE hr.employees
    ///        SET salary = ROUND(salary * 1.1)
    ///      WHERE employee_id = :id
    /// ")?;
    /// let num_updated_rows = stmt.execute(&[ &107 ])?;
    /// assert_eq!(1, num_updated_rows);
    ///
    /// conn.rollback()?;
    /// # Ok::<(),Box<dyn std::error::Error>>(())
    /// ```
    pub fn rollback(&self) -> Result<()> {
        catch!{self.err_ptr() =>
            OCITransRollback(self.svc_ptr(), self.err_ptr(), OCI_DEFAULT)
        }
        Ok(())
    }

    /// Causes the server to measure call time, in milliseconds, for each subsequent OCI call.
    pub fn start_call_time_measurements(&self) -> Result<()> {
        self.usr.set_attr(OCI_ATTR_COLLECT_CALL_TIME, 1u8, self.err_ptr())
    }

    /// Returns the server-side time for the preceding call in microseconds.
    pub fn get_call_time(&self) -> Result<u64> {
        self.usr.get_attr::<u64>(OCI_ATTR_CALL_TIME, self.err_ptr())
    }

    /// Terminates call time measurements.
    pub fn stop_call_time_measurements(&self) -> Result<()> {
        self.usr.set_attr(OCI_ATTR_COLLECT_CALL_TIME, 0u8, self.err_ptr())
    }

    /// Sets the name of the current module (`V$SESSION.MODULE`) running in the client application.
    /// When the current module terminates, call with the name of the new module, or use empty
    /// string if there is no new module. Can be up to 48 bytes long.
    ///
    /// ## Example
    /// ```
    /// # let dbname = std::env::var("DBNAME")?;
    /// # let dbuser = std::env::var("DBUSER")?;
    /// # let dbpass = std::env::var("DBPASS")?;
    /// # let oracle = sibyl::env()?;
    /// # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    /// conn.set_module("sibyl");
    ///
    /// let stmt = conn.prepare("
    ///     SELECT module
    ///       FROM v$session
    ///      WHERE sid = SYS_CONTEXT('USERENV', 'SID')
    /// ")?;
    /// let rows = stmt.query(&[])?;
    /// let row = rows.next()?;
    /// assert!(row.is_some());
    /// let row = row.unwrap();
    /// let module = row.get::<&str>(0)?;
    /// assert!(module.is_some());
    /// let module = module.unwrap();
    /// assert_eq!(module, "sibyl");
    /// # Ok::<(),Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_module(&self, name: &str) -> Result<()> {
        self.usr.set_attr(OCI_ATTR_MODULE, name, self.err_ptr())
    }

    /// Sets the name of the current action (`V$SESSION.ACTION`) within the current module.
    /// When the current action terminates, set this attribute again with the name of the
    /// next action, or empty string if there is no next action. Can be up to 32 bytes long.
    ///
    /// ## Example
    /// ```
    /// # let dbname = std::env::var("DBNAME")?;
    /// # let dbuser = std::env::var("DBUSER")?;
    /// # let dbpass = std::env::var("DBPASS")?;
    /// # let oracle = sibyl::env()?;
    /// # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    /// conn.set_action("Session Test");
    ///
    /// let stmt = conn.prepare("
    ///     SELECT action
    ///       FROM v$session
    ///      WHERE sid = SYS_CONTEXT('USERENV', 'SID')
    /// ")?;
    /// let rows = stmt.query(&[])?;
    /// let row = rows.next()?;
    /// assert!(row.is_some());
    /// let row = row.unwrap();
    /// let action = row.get::<&str>(0)?;
    /// assert!(action.is_some());
    /// let action = action.unwrap();
    /// assert_eq!(action, "Session Test");
    /// # Ok::<(),Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_action(&self, action: &str) -> Result<()> {
        self.usr.set_attr(OCI_ATTR_ACTION, action, self.err_ptr())
    }

    /// Sets the user identifier (`V$SESSION.CLIENT_IDENTIFIER`) in the session handle.
    /// Can be up to 64 bytes long.
    ///
    /// ## Example
    /// ```
    /// # let dbname = std::env::var("DBNAME")?;
    /// # let dbuser = std::env::var("DBUSER")?;
    /// # let dbpass = std::env::var("DBPASS")?;
    /// # let oracle = sibyl::env()?;
    /// # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    /// conn.set_client_identifier("Test Weilder");
    ///
    /// let stmt = conn.prepare("
    ///     SELECT client_identifier
    ///       FROM v$session
    ///      WHERE sid = SYS_CONTEXT('USERENV', 'SID')
    /// ")?;
    /// let rows = stmt.query(&[])?;
    /// let row = rows.next()?;
    /// assert!(row.is_some());
    /// let row = row.unwrap();
    /// let client_identifier = row.get::<&str>(0)?;
    /// assert!(client_identifier.is_some());
    /// let client_identifier = client_identifier.unwrap();
    /// assert_eq!(client_identifier, "Test Weilder");
    /// # Ok::<(),Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_client_identifier(&self, id: &str) -> Result<()> {
        self.usr.set_attr(OCI_ATTR_CLIENT_IDENTIFIER, id, self.err_ptr())
    }

    /// Sets additional client application information (`V$SESSION.CLIENT_INFO`).
    /// Can be up to 64 bytes long.
    ///
    /// ## Example
    /// ```
    /// # let dbname = std::env::var("DBNAME")?;
    /// # let dbuser = std::env::var("DBUSER")?;
    /// # let dbpass = std::env::var("DBPASS")?;
    /// # let oracle = sibyl::env()?;
    /// # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    /// conn.set_client_info("Nothing to see here, move along folks");
    ///
    /// let stmt = conn.prepare("
    ///     SELECT client_info
    ///       FROM v$session
    ///      WHERE sid = SYS_CONTEXT('USERENV', 'SID')
    /// ")?;
    /// let rows = stmt.query(&[])?;
    /// let row = rows.next()?;
    /// assert!(row.is_some());
    /// let row = row.unwrap();
    /// let client_info = row.get::<&str>(0)?;
    /// assert!(client_info.is_some());
    /// let client_info = client_info.unwrap();
    /// assert_eq!(client_info, "Nothing to see here, move along folks");
    /// # Ok::<(),Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_client_info(&self, info: &str) -> Result<()> {
        self.usr.set_attr(OCI_ATTR_CLIENT_INFO, info, self.err_ptr())
    }

    /// Returns the current schema.
    pub fn get_current_schema(&self) -> Result<&str> {
        self.usr.get_attr::<&str>(OCI_ATTR_CURRENT_SCHEMA, self.err_ptr())
    }

    /// Sets the current schema. It has the same effect as the SQL command ALTER SESSION SET CURRENT_SCHEMA
    /// if the schema name and the session exist. The schema is altered on the next OCI call that does a
    /// round-trip to the server, avoiding an extra round-trip. If the new schema name does not exist, the
    /// same error is returned as the error returned from ALTER SESSION SET CURRENT_SCHEMA. The new schema
    /// name is placed before database objects in DML or DDL commands that you then enter.
    ///
    /// ## Example
    /// ```
    /// # let dbname = std::env::var("DBNAME")?;
    /// # let dbuser = std::env::var("DBUSER")?;
    /// # let dbpass = std::env::var("DBPASS")?;
    /// # let oracle = sibyl::env()?;
    /// # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
    /// let orig_name = conn.get_current_schema()?;
    /// conn.set_current_schema("HR")?;
    /// assert_eq!(conn.get_current_schema()?, "HR");
    ///
    /// let stmt = conn.prepare("
    ///     SELECT schemaname
    ///       FROM v$session
    ///      WHERE sid = SYS_CONTEXT('USERENV', 'SID')
    /// ")?;
    /// let rows = stmt.query(&[])?;
    /// let row = rows.next()?;
    /// assert!(row.is_some());
    /// let row = row.unwrap();
    /// let schema_name = row.get::<&str>(0)?;
    /// assert!(schema_name.is_some());
    /// let schema_name = schema_name.unwrap();
    /// assert_eq!(schema_name, "HR");
    ///
    /// conn.set_current_schema(orig_name)?;
    /// assert_eq!(conn.get_current_schema()?, orig_name);
    /// # Ok::<(),Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_current_schema(&self, schema_name: &str) -> Result<()> {
        self.usr.set_attr(OCI_ATTR_CURRENT_SCHEMA, schema_name, self.err_ptr())
    }

    /// Sets the default prefetch buffer size for each LOB locator.
    ///
    /// This attribute value enables prefetching for all the LOB locators fetched in the session.
    /// The default value for this attribute is zero (no prefetch of LOB data). This option
    /// relieves the application developer from setting the prefetch LOB size for each define handle.
    ///
    ///
    pub fn set_lob_prefetch_size(&self, size: u32) -> Result<()> {
        self.usr.set_attr(OCI_ATTR_DEFAULT_LOBPREFETCH_SIZE, size, self.err_ptr())
    }
}
