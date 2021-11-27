//! User Session

#[cfg(feature="blocking")]
#[cfg_attr(docsrs, doc(cfg(feature="blocking")))]
pub mod blocking;

#[cfg(feature="nonblocking")]
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
pub mod nonblocking;

use crate::{Environment, Result, env::Env, oci::{*, self, attr::{AttrGet, AttrSet}}, types::Ctx};
use std::ptr;
use libc::c_void;

/**
    Connects to the speficied server and starts new user session.

    As nonblocking mode can only be set after the `OCISessionBegin()`,
    this function is always blocking.
*/
fn connect<'a>(env: &'a Environment, addr: &str, user: &str, pass: &str) -> Result<Connection<'a>> {
    let err = Handle::<OCIError>::new(env.env_ptr())?;
    let inf = Handle::<OCIAuthInfo>::new(env.env_ptr())?;
    inf.set_attr(OCI_ATTR_DRIVER_NAME, "sibyl", err.get())?;
    inf.set_attr(OCI_ATTR_USERNAME, user, err.get())?;
    inf.set_attr(OCI_ATTR_PASSWORD, pass, err.get())?;
    let mut svc = Ptr::null();
    let mut found = 0u8;
    oci::session_get(
        env.env_ptr(), err.get(), svc.as_mut_ptr(), inf.get(), addr.as_ptr(), addr.len() as u32,
        ptr::null(), 0, ptr::null_mut(), ptr::null_mut(), &mut found, 
        OCI_SESSGET_STMTCACHE
    )?;    
    attr::get::<Ptr<OCISession>>(OCI_ATTR_SESSION, OCI_HTYPE_SVCCTX, svc.get() as *const c_void, err.get())
    .or_else(|attr_err| {
        unsafe {
            OCISessionRelease(svc.get(), err.get(), ptr::null(), 0, OCI_DEFAULT);
        }
        Err(attr_err)
    })
    .map(|usr| 
        Connection { env, err, svc, usr }
    )
}

fn get_from_session_pool<'a>(env: &'a Environment, pool_name: &str) -> Result<Connection<'a>> {
    let err = Handle::<OCIError>::new(env.env_ptr())?;
    let inf = Handle::<OCIAuthInfo>::new(env.env_ptr())?;
    let mut svc = Ptr::null();
    let mut found = 0u8;
    oci::session_get(
        env.env_ptr(), err.get(), svc.as_mut_ptr(), inf.get(), pool_name.as_ptr(), pool_name.len() as u32,
        ptr::null(), 0, ptr::null_mut(), ptr::null_mut(), &mut found, 
        OCI_SESSGET_SPOOL | OCI_SESSGET_SPOOL_MATCHANY | OCI_SESSGET_PURITY_SELF
    )?;    
    attr::get::<Ptr<OCISession>>(OCI_ATTR_SESSION, OCI_HTYPE_SVCCTX, svc.get() as *const c_void, err.get())
    .or_else(|attr_err| {
        unsafe {
            OCISessionRelease(svc.get(), err.get(), ptr::null(), 0, OCI_DEFAULT);
        }
        Err(attr_err)
    })
    .map(|usr| 
        Connection { env, err, svc, usr }
    )
}

fn get_from_connection_pool<'a>(env: &'a Environment, pool_name: &str, username: &str, password: &str) -> Result<Connection<'a>> {
    let err = Handle::<OCIError>::new(env.env_ptr())?;
    let inf = Handle::<OCIAuthInfo>::new(env.env_ptr())?;
    inf.set_attr(OCI_ATTR_DRIVER_NAME, "sibyl", err.get())?;
    inf.set_attr(OCI_ATTR_USERNAME, username, err.get())?;
    inf.set_attr(OCI_ATTR_PASSWORD, password, err.get())?;
    let mut svc = Ptr::null();
    let mut found = 0u8;
    oci::session_get(
        env.env_ptr(), err.get(), svc.as_mut_ptr(), inf.get(), pool_name.as_ptr(), pool_name.len() as u32,
        ptr::null(), 0, ptr::null_mut(), ptr::null_mut(), &mut found, 
        OCI_SESSGET_CPOOL | OCI_SESSGET_STMTCACHE
    )?;    
    attr::get::<Ptr<OCISession>>(OCI_ATTR_SESSION, OCI_HTYPE_SVCCTX, svc.get() as *const c_void, err.get())
    .or_else(|attr_err| {
        unsafe {
            OCISessionRelease(svc.get(), err.get(), ptr::null(), 0, OCI_DEFAULT);
        }
        Err(attr_err)
    })
    .map(|usr| 
        Connection { env, err, svc, usr }
    )
}


/// Represents a user session
pub struct Connection<'a> {
    env: &'a Environment,
    err: Handle<OCIError>,
    usr: Ptr<OCISession>,
    svc: Ptr<OCISvcCtx>,
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
    fn ctx_ptr(&self) -> *mut c_void {
        self.usr_ptr() as *mut c_void
    }
}

impl Connection<'_> {
    pub(crate) fn svc_ptr(&self) -> *mut OCISvcCtx {
        self.svc.get()
    }

    pub(crate) fn usr_ptr(&self) -> *mut OCISession {
        self.usr.get()
    }

    fn set_session_attr<T: AttrSet>(&self, attr_type: u32, attr_val: T) -> Result<()> {
        attr::set(attr_type, attr_val, OCI_HTYPE_SESSION, self.usr_ptr() as *mut c_void, self.err_ptr())        
    }

    fn get_session_attr<T: AttrGet>(&self, attr_type: u32) -> Result<T> {
        attr::get(attr_type, OCI_HTYPE_SESSION, self.usr_ptr() as *const c_void, self.err_ptr())
    }

    /// Reports whether self is connected to the server
    pub fn is_connected(&self) -> Result<bool> {
        attr::get::<Ptr<OCIServer>>(OCI_ATTR_SERVER, OCI_HTYPE_SVCCTX, self.svc_ptr() as *const c_void, self.err_ptr())
        .and_then(|srv| 
            attr::get::<u32>(OCI_ATTR_SERVER_STATUS, OCI_HTYPE_SERVER, srv.get() as *const c_void, self.err_ptr())
        )
        .map(|status| 
            status == OCI_SERVER_NORMAL
        )
    }

    /// Reports whether connection is established in non-blocking mode.
    pub fn is_async(&self) -> Result<bool> {
        attr::get::<Ptr<OCIServer>>(OCI_ATTR_SERVER, OCI_HTYPE_SVCCTX, self.svc_ptr() as *const c_void, self.err_ptr())
        .and_then(|srv| 
            attr::get::<u8>(OCI_ATTR_NONBLOCKING_MODE, OCI_HTYPE_SERVER, srv.get() as *const c_void, self.err_ptr())
        )
        .map(|mode| 
            mode != 0
        )
    }

    /// Causes the server to measure call time, in milliseconds, for each subsequent OCI call.
    pub fn start_call_time_measurements(&self) -> Result<()> {
        self.set_session_attr(OCI_ATTR_COLLECT_CALL_TIME, 1u8)
    }

    /// Returns the server-side time for the preceding call in microseconds.
    pub fn call_time(&self) -> Result<u64> {
        self.get_session_attr(OCI_ATTR_CALL_TIME)
    }

    /// Terminates call time measurements.
    pub fn stop_call_time_measurements(&self) -> Result<()> {
        self.set_session_attr(OCI_ATTR_COLLECT_CALL_TIME, 0u8)
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
        self.set_session_attr(OCI_ATTR_MODULE, name)
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
        self.set_session_attr(OCI_ATTR_ACTION, action)
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
        self.set_session_attr(OCI_ATTR_CLIENT_IDENTIFIER, id)
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
        self.set_session_attr(OCI_ATTR_CLIENT_INFO, info)
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
        let orig_name = conn.current_schema()?;

        conn.set_current_schema("HR")?;
        assert_eq!(conn.current_schema()?, "HR");

        conn.set_current_schema(orig_name)?;
        assert_eq!(conn.current_schema()?, orig_name);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn current_schema(&self) -> Result<&str> {
        self.get_session_attr(OCI_ATTR_CURRENT_SCHEMA)
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
        let orig_name = conn.current_schema()?;
        conn.set_current_schema("HR")?;
        assert_eq!(conn.current_schema()?, "HR");

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
        assert_eq!(conn.current_schema()?, orig_name);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn set_current_schema(&self, schema_name: &str) -> Result<()> {
        self.set_session_attr(OCI_ATTR_CURRENT_SCHEMA, schema_name)
    }

    /**
        Sets the default prefetch buffer size for each LOB locator.

        This attribute value enables prefetching for all the LOB locators fetched in the session.
        The default value for this attribute is zero (no prefetch of LOB data). This option
        relieves the application developer from setting the prefetch LOB size for each LOB column
        in each prepared statement.
    */
    pub fn set_lob_prefetch_size(&self, size: u32) -> Result<()> {
        self.set_session_attr(OCI_ATTR_DEFAULT_LOBPREFETCH_SIZE, size)
    }
}
