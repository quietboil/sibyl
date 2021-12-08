//! User Session

#[cfg(feature="blocking")]
#[cfg_attr(docsrs, doc(cfg(feature="blocking")))]
mod blocking;

#[cfg(feature="nonblocking")]
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
mod nonblocking;

use std::{sync::Arc, marker::PhantomData};

use libc::c_void;

use crate::{Result, env::Env, oci::{*, attr::{AttrGet, AttrSet}}, types::Ctx, Environment};

pub(crate) struct Session {
    env: Arc<Handle<OCIEnv>>,
    err: Handle<OCIError>,
    svc: Ptr<OCISvcCtx>,
}

impl Session {
    pub(crate) fn svc_ptr(&self) -> *mut OCISvcCtx {
        self.svc.get()
    }

    // fn usr_ptr(&self) -> *mut OCISession {
    //     self.usr.get()
    // }

    pub(crate) fn get_svc_ptr(&self) -> Ptr<OCISvcCtx> {
        Ptr::new(self.svc.get())
    }
}

impl Env for Session {
    fn env_ptr(&self) -> *mut OCIEnv {
        self.env.get()
    }

    fn err_ptr(&self) -> *mut OCIError {
        self.err.get()
    }

    fn get_env_ptr(&self) -> Ptr<OCIEnv> {
        Ptr::new(self.env_ptr())
    }

    fn get_err_ptr(&self) -> Ptr<OCIError> {
        Ptr::new(self.err_ptr())
    }
}

// impl Ctx for Session {
//     fn ctx_ptr(&self) -> *mut c_void {
//         self.usr.get() as *mut c_void
//     }
// }

/// Represents a user session
pub struct Connection<'a> {
    session:      Arc<Session>,
    usr:          Ptr<OCISession>,
    phantom_env:  PhantomData<&'a Environment>
}

impl Env for Connection<'_> {
    fn env_ptr(&self) -> *mut OCIEnv {
        self.session.env_ptr()
    }

    fn err_ptr(&self) -> *mut OCIError {
        self.session.err_ptr()
    }

    fn get_env_ptr(&self) -> Ptr<OCIEnv> {
        self.session.get_env_ptr()
    }

    fn get_err_ptr(&self) -> Ptr<OCIError> {
        self.session.get_err_ptr()
    }
}

impl Ctx for Connection<'_> {
    fn ctx_ptr(&self) -> *mut c_void {
        self.usr_ptr() as *mut c_void
    }
}

impl Connection<'_> {
    pub(crate) fn svc_ptr(&self) -> *mut OCISvcCtx {
        self.session.svc_ptr()
    }   

    fn usr_ptr(&self) -> *mut OCISession {
        self.usr.get()
    }

    pub(crate) fn get_svc_ptr(&self) -> Ptr<OCISvcCtx> {
        self.session.get_svc_ptr()
    }   

    pub(crate) fn clone_session(&self) -> Arc<Session> {
        self.session.clone()
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
        string if there is no new module. The name can be up to 48 bytes long.

        # Example

        ```
        # use sibyl::Result;
        // === Blocking mode variant ===
        # #[cfg(feature="blocking")]
        # fn test() -> Result<()> {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;

        conn.set_module("sibyl");

        let stmt = conn.prepare("
            SELECT module
              FROM v$session
             WHERE sid = SYS_CONTEXT('USERENV', 'SID')
        ")?;
        let rows = stmt.query(&[])?;
        let row = rows.next()?.unwrap();
        let module : &str = row.get(0)?.unwrap();
        assert_eq!(module, "sibyl");
        # Ok(())
        # }

        // === Nonblocking mode variant ===
        # #[cfg(feature="nonblocking")]
        # fn test() -> Result<()> {
        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;

        conn.set_module("sibyl");

        let stmt = conn.prepare("
            SELECT module
              FROM v$session
             WHERE sid = SYS_CONTEXT('USERENV', 'SID')
        ").await?;
        let rows = stmt.query(&[]).await?;
        let row = rows.next().await?.unwrap();
        let module : &str = row.get(0)?.unwrap();
        assert_eq!(module, "sibyl");
        # Ok(()) })
        # }
        # fn main() -> Result<()> { test() }
        ```
    */
    pub fn set_module(&self, name: &str) -> Result<()> {
        self.set_session_attr(OCI_ATTR_MODULE, name)
    }

    /**
        Sets the name of the current action (`V$SESSION.ACTION`) within the current module.
        When the current action terminates, set this attribute again with the name of the
        next action, or empty string if there is no next action. Can be up to 32 bytes long.

        # Example

        ```
        # use sibyl::Result;
        // === Blocking mode variant ===
        # #[cfg(feature="blocking")]
        # fn test() -> Result<()> {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;

        conn.set_action("Session Test");

        let stmt = conn.prepare("
            SELECT action
              FROM v$session
             WHERE sid = SYS_CONTEXT('USERENV', 'SID')
        ")?;
        let rows = stmt.query(&[])?;
        let row = rows.next()?.unwrap();
        let action : &str = row.get(0)?.unwrap();
        assert_eq!(action, "Session Test");
        # Ok(())
        # }

        // === Nonblocking mode variant ===
        # #[cfg(feature="nonblocking")]
        # fn test() -> Result<()> {
        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;

        conn.set_action("Session Test");

        let stmt = conn.prepare("
            SELECT action
              FROM v$session
             WHERE sid = SYS_CONTEXT('USERENV', 'SID')
        ").await?;
        let rows = stmt.query(&[]).await?;
        let row = rows.next().await?.unwrap();
        let action : &str = row.get(0)?.unwrap();
        assert_eq!(action, "Session Test");
        # Ok(()) })
        # }
        # fn main() -> Result<()> { test() }
        ```
    */
    pub fn set_action(&self, action: &str) -> Result<()> {
        self.set_session_attr(OCI_ATTR_ACTION, action)
    }

    /**
        Sets the user identifier (`V$SESSION.CLIENT_IDENTIFIER`) in the session handle.
        Can be up to 64 bytes long.

        # Example

        ```
        # use sibyl::Result;
        // === Blocking mode variant ===
        # #[cfg(feature="blocking")]
        # fn test() -> Result<()> {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;

        conn.set_client_identifier("Test Wielder");

        let stmt = conn.prepare("
            SELECT client_identifier
              FROM v$session
             WHERE sid = SYS_CONTEXT('USERENV', 'SID')
        ")?;
        let rows = stmt.query(&[])?;
        let row = rows.next()?.unwrap();
        let client_identifier : &str = row.get(0)?.unwrap();
        assert_eq!(client_identifier, "Test Wielder");
        # Ok(())
        # }

        // === Nonblocking mode variant ===
        # #[cfg(feature="nonblocking")]
        # fn test() -> Result<()> {
        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;

        conn.set_client_identifier("Test Wielder");

        let stmt = conn.prepare("
            SELECT client_identifier
              FROM v$session
             WHERE sid = SYS_CONTEXT('USERENV', 'SID')
        ").await?;
        let rows = stmt.query(&[]).await?;
        let row = rows.next().await?.unwrap();
        let client_identifier : &str = row.get(0)?.unwrap();
        assert_eq!(client_identifier, "Test Wielder");
        # Ok(()) })
        # }
        # fn main() -> Result<()> { test() }
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
        # use sibyl::Result;
        // === Blocking mode variant ===
        # #[cfg(feature="blocking")]
        # fn test() -> Result<()> {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;

        conn.set_client_info("Nothing to see here, move along folks");

        let stmt = conn.prepare("
            SELECT client_info
              FROM v$session
             WHERE sid = SYS_CONTEXT('USERENV', 'SID')
        ")?;
        let rows = stmt.query(&[])?;
        let row = rows.next()?.unwrap();
        let client_info : &str = row.get(0)?.unwrap();
        assert_eq!(client_info, "Nothing to see here, move along folks");
        # Ok(())
        # }

        // === Nonblocking mode variant ===
        # #[cfg(feature="nonblocking")]
        # fn test() -> Result<()> {
        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;

        conn.set_client_info("Nothing to see here, move along folks");

        let stmt = conn.prepare("
            SELECT client_info
              FROM v$session
             WHERE sid = SYS_CONTEXT('USERENV', 'SID')
        ").await?;
        let rows = stmt.query(&[]).await?;
        let row = rows.next().await?.unwrap();
        let client_info : &str = row.get(0)?.unwrap();
        assert_eq!(client_info, "Nothing to see here, move along folks");
        # Ok(()) })
        # }
        # fn main() -> Result<()> { test() }
        ```
    */
    pub fn set_client_info(&self, info: &str) -> Result<()> {
        self.set_session_attr(OCI_ATTR_CLIENT_INFO, info)
    }

    /**
        Returns the current schema.

        # Example

        ```
        # use sibyl::Result;
        # #[cfg(feature="blocking")]
        # fn test() -> Result<()> {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        #
        let orig_name = conn.current_schema()?;

        conn.set_current_schema("HR")?;
        assert_eq!(conn.current_schema()?, "HR");

        conn.set_current_schema(orig_name)?;
        assert_eq!(conn.current_schema()?, orig_name);
        #
        # Ok(())
        # }
        # #[cfg(feature="nonblocking")]
        # fn test() -> Result<()> { Ok(()) }
        # fn main() -> Result<()> { test() }
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

        # Example

        ```
        # use sibyl::Result;
        // === Blocking mode variant ===
        # #[cfg(feature="blocking")]
        # fn test() -> Result<()> {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;

        let orig_name = conn.current_schema()?;
        conn.set_current_schema("HR")?;
        assert_eq!(conn.current_schema()?, "HR");

        let stmt = conn.prepare("
            SELECT schemaname
              FROM v$session
             WHERE sid = SYS_CONTEXT('USERENV', 'SID')
        ")?;
        let rows = stmt.query(&[])?;
        let row = rows.next()?.unwrap();
        let schema_name : &str = row.get(0)?.unwrap();
        assert_eq!(schema_name, "HR");

        conn.set_current_schema(orig_name)?;
        assert_eq!(conn.current_schema()?, orig_name);
        # Ok(())
        # }

        // === Nonblocking mode variant ===
        # #[cfg(feature="nonblocking")]
        # fn test() -> Result<()> {
        # sibyl::test::on_single_thread(async {
        # let oracle = sibyl::env()?;
        # let dbname = std::env::var("DBNAME").expect("database name");
        # let dbuser = std::env::var("DBUSER").expect("schema name");
        # let dbpass = std::env::var("DBPASS").expect("password");
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass).await?;

        let orig_name = conn.current_schema()?;
        conn.set_current_schema("HR")?;
        assert_eq!(conn.current_schema()?, "HR");

        let stmt = conn.prepare("
            SELECT schemaname
              FROM v$session
             WHERE sid = SYS_CONTEXT('USERENV', 'SID')
        ").await?;
        let rows = stmt.query(&[]).await?;
        let row = rows.next().await?.unwrap();
        let schema_name : &str = row.get(0)?.unwrap();
        assert_eq!(schema_name, "HR");

        conn.set_current_schema(orig_name)?;
        assert_eq!(conn.current_schema()?, orig_name);
        # Ok(()) })
        # }
        # fn main() -> Result<()> { test() }
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
