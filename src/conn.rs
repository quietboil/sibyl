//! User Session

#[cfg(feature="blocking")]
#[cfg_attr(docsrs, doc(cfg(feature="blocking")))]
pub mod blocking;

#[cfg(feature="nonblocking")]
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
pub mod nonblocking;

use crate::{Environment, Result, catch, env::Env, oci::*, types::Ctx};
use libc::c_void;

/**
    Connects to the speficied server and starts new user session.

    As nonblocking mode can only be set after the `OCISessionBegin()`,
    this function is always blocking.
*/ 
fn connect<'a>(env: &'a Environment, addr: &str, user: &str, pass: &str) -> Result<Connection<'a>> {
    let err = Handle::<OCIError>::new(env.env_ptr())?;
    let srv = Handle::<OCIServer>::new(env.env_ptr())?;
    let svc = Handle::<OCISvcCtx>::new(env.env_ptr())?;
    let usr = Handle::<OCISession>::new(env.env_ptr())?;
    usr.set_attr(OCI_ATTR_DRIVER_NAME, "sibyl", err.get())?;
    catch!{err.get() =>
        OCIServerAttach(srv.get(), err.get(), addr.as_ptr(), addr.len() as u32, OCI_DEFAULT)
    }
    if let Err(set_attr_err) = svc.set_attr(OCI_ATTR_SERVER, srv.get(), err.get()) {
        unsafe {
            OCIServerDetach(srv.get(), err.get(), OCI_DEFAULT);
        }
        return Err(set_attr_err);
    }

    let conn = Connection { env, usr, svc, srv, err };
    conn.usr.set_attr(OCI_ATTR_USERNAME, user, conn.err_ptr())?;
    conn.usr.set_attr(OCI_ATTR_PASSWORD, pass, conn.err_ptr())?;
    catch!{conn.err_ptr() =>
        OCISessionBegin(
            conn.svc_ptr(), conn.err_ptr(), conn.usr_ptr(),
            if user.len() == 0 && pass.len() == 0 { OCI_CRED_EXT } else { OCI_CRED_RDBMS },
            OCI_DEFAULT
        )
    }
    if let Err(set_attr_err) = conn.svc.set_attr(OCI_ATTR_SESSION, conn.usr_ptr(), conn.err_ptr()) {
        unsafe {
            OCISessionEnd(conn.svc_ptr(), conn.err_ptr(), conn.usr_ptr(), OCI_DEFAULT);
        }
        return Err(set_attr_err);
    }
    Ok(conn)
}

/// Represents a user session
pub struct Connection<'a> {
    env: &'a Environment,
    usr: Handle<OCISession>,
    svc: Handle<OCISvcCtx>,
    srv: Handle<OCIServer>,
    err: Handle<OCIError>,
}

impl Env for Connection<'_> {
    fn env_ptr(&self) -> *mut OCIEnv {
        self.env.env_ptr()
    }

    fn err_ptr(&self) -> *mut OCIError {
        self.err.get()
    }
}

impl Ctx for Connection<'_> {
    fn as_ptr(&self) -> *mut c_void {
        self.usr.get() as *mut c_void
    }
}

impl<'a> Connection<'a> {
    pub(crate) fn srv_ptr(&self) -> *mut OCIServer {
        self.srv.get()
    }

    pub(crate) fn svc_ptr(&self) -> *mut OCISvcCtx {
        self.svc.get()
    }

    pub(crate) fn usr_ptr(&self) -> *mut OCISession {
        self.usr.get()
    }

    /// Reports whether self is connected to the server
    pub fn is_connected(&self) -> Result<bool> {
        let status : u32 = self.srv.get_attr(OCI_ATTR_SERVER_STATUS, self.err_ptr())?;
        Ok(status == OCI_SERVER_NORMAL)
    }

    /// Reports whether connection is established in non-blocking mode.
    pub fn is_async(&self) -> Result<bool> {
        let mode : u8 = self.srv.get_attr(OCI_ATTR_NONBLOCKING_MODE, self.err_ptr())?;
        Ok(mode != 0)
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

    /**
        Sets the name of the current module (`V$SESSION.MODULE`) running in the client application.
        When the current module terminates, call with the name of the new module, or use empty
        string if there is no new module. Can be up to 48 bytes long.

        ## Example
        ```
        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        conn.set_module("sibyl");

        let stmt = conn.prepare("
            SELECT module
              FROM v$session
             WHERE sid = SYS_CONTEXT('USERENV', 'SID')
        ")?;
        let mut rows = stmt.query(&[])?;
        let row = rows.next()?.unwrap();
        let module : &str = row.get(0)?.unwrap();
        assert_eq!(module, "sibyl");
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn set_module(&self, name: &str) -> Result<()> {
        self.usr.set_attr(OCI_ATTR_MODULE, name, self.err_ptr())
    }

    /**
        Sets the name of the current action (`V$SESSION.ACTION`) within the current module.
        When the current action terminates, set this attribute again with the name of the
        next action, or empty string if there is no next action. Can be up to 32 bytes long.

        ## Example
        ```
        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        conn.set_action("Session Test");

        let stmt = conn.prepare("
            SELECT action
              FROM v$session
             WHERE sid = SYS_CONTEXT('USERENV', 'SID')
        ")?;
        let mut rows = stmt.query(&[])?;
        let row = rows.next()?.unwrap();
        let action : &str = row.get(0)?.unwrap();
        assert_eq!(action, "Session Test");
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn set_action(&self, action: &str) -> Result<()> {
        self.usr.set_attr(OCI_ATTR_ACTION, action, self.err_ptr())
    }

    /**
        Sets the user identifier (`V$SESSION.CLIENT_IDENTIFIER`) in the session handle.
        Can be up to 64 bytes long.

        ## Example
        ```
        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        conn.set_client_identifier("Test Wielder");

        let stmt = conn.prepare("
            SELECT client_identifier
              FROM v$session
             WHERE sid = SYS_CONTEXT('USERENV', 'SID')
        ")?;
        let mut rows = stmt.query(&[])?;
        let row = rows.next()?.unwrap();
        let client_identifier : &str = row.get(0)?.unwrap();
        assert_eq!(client_identifier, "Test Wielder");
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn set_client_identifier(&self, id: &str) -> Result<()> {
        self.usr.set_attr(OCI_ATTR_CLIENT_IDENTIFIER, id, self.err_ptr())
    }

    /**
        Sets additional client application information (`V$SESSION.CLIENT_INFO`).
        Can be up to 64 bytes long.

        ## Example
        ```
        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        conn.set_client_info("Nothing to see here, move along folks");

        let stmt = conn.prepare("
            SELECT client_info
              FROM v$session
             WHERE sid = SYS_CONTEXT('USERENV', 'SID')
        ")?;
        let mut rows = stmt.query(&[])?;
        let row = rows.next()?.unwrap();
        let client_info : &str = row.get(0)?.unwrap();
        assert_eq!(client_info, "Nothing to see here, move along folks");
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn set_client_info(&self, info: &str) -> Result<()> {
        self.usr.set_attr(OCI_ATTR_CLIENT_INFO, info, self.err_ptr())
    }

    /**
        Returns the current schema.

        # Example
        ```
        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let orig_name = conn.get_current_schema()?;

        conn.set_current_schema("HR")?;
        assert_eq!(conn.get_current_schema()?, "HR");

        conn.set_current_schema(orig_name)?;
        assert_eq!(conn.get_current_schema()?, orig_name);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn get_current_schema(&self) -> Result<&str> {
        self.usr.get_attr::<&str>(OCI_ATTR_CURRENT_SCHEMA, self.err_ptr())
    }

    /**
        Sets the current schema. It has the same effect as the SQL command ALTER SESSION SET CURRENT_SCHEMA
        if the schema name and the session exist. The schema is altered on the next OCI call that does a
        round-trip to the server, avoiding an extra round-trip. If the new schema name does not exist, the
        same error is returned as the error returned from ALTER SESSION SET CURRENT_SCHEMA. The new schema
        name is placed before database objects in DML or DDL commands that you then enter.

        ## Example
        ```
        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let orig_name = conn.get_current_schema()?;
        conn.set_current_schema("HR")?;
        assert_eq!(conn.get_current_schema()?, "HR");

        let stmt = conn.prepare("
            SELECT schemaname
              FROM v$session
             WHERE sid = SYS_CONTEXT('USERENV', 'SID')
        ")?;
        let mut rows = stmt.query(&[])?;
        let row = rows.next()?.unwrap();
        let schema_name : &str = row.get(0)?.unwrap();
        assert_eq!(schema_name, "HR");

        conn.set_current_schema(orig_name)?;
        assert_eq!(conn.get_current_schema()?, orig_name);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn set_current_schema(&self, schema_name: &str) -> Result<()> {
        self.usr.set_attr(OCI_ATTR_CURRENT_SCHEMA, schema_name, self.err_ptr())
    }

    /**
        Sets the default prefetch buffer size for each LOB locator.

        This attribute value enables prefetching for all the LOB locators fetched in the session.
        The default value for this attribute is zero (no prefetch of LOB data). This option
        relieves the application developer from setting the prefetch LOB size for each LOB column
        in each prepared statement.
    */
    pub fn set_lob_prefetch_size(&self, size: u32) -> Result<()> {
        self.usr.set_attr(OCI_ATTR_DEFAULT_LOBPREFETCH_SIZE, size, self.err_ptr())
    }
}
