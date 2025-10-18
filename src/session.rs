//! User Session

#[cfg(feature="blocking")]
#[cfg_attr(docsrs, doc(cfg(feature="blocking")))]
mod blocking;

#[cfg(feature="nonblocking")]
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
mod nonblocking;

use std::{sync::Arc, marker::PhantomData};
use crate::{Result, Environment, oci::*, types::Ctx};
use crate::pool::session::SPool;

/// Representation of the service context.
/// It will be behinfd `Arc` as it needs to survive the `Session`
/// drop to allow statements and cursors to be dropped asynchronously.
pub(crate) struct SvcCtx {
    svc: Ptr<OCISvcCtx>,
    inf: Handle<OCIAuthInfo>,
    err: Handle<OCIError>,
    spool: Option<Arc<SPool>>,
    env: Arc<Handle<OCIEnv>>,
    #[cfg(feature="nonblocking")]
    active_future: std::sync::atomic::AtomicUsize,
}

#[cfg(not(docsrs))]
impl Drop for SvcCtx {
    fn drop(&mut self) {
        let _ = &self.inf;
        let _ = &self.spool;

        #[cfg(feature="nonblocking")]
        let _ = self.set_blocking_mode(); // best effort, nothing we can do if it fails

        let svc : &OCISvcCtx = self.as_ref();
        let err : &OCIError  = self.as_ref();
        oci_trans_rollback(svc, err);
        oci_session_release(svc, err);
    }
}

impl AsRef<OCIEnv> for SvcCtx {
    fn as_ref(&self) -> &OCIEnv {
        &*self.env
    }
}

impl AsRef<OCIError> for SvcCtx {
    fn as_ref(&self) -> &OCIError {
        &*self.err
    }
}

impl AsRef<OCISvcCtx> for SvcCtx {
    fn as_ref(&self) -> &OCISvcCtx {
        &*self.svc
    }
}

/// Represents a user session
pub struct Session<'a> {
    usr: Ptr<OCISession>,
    ctx: Arc<SvcCtx>,
    phantom_env:  PhantomData<&'a Environment>
}

impl AsRef<OCIEnv> for Session<'_> {
    fn as_ref(&self) -> &OCIEnv {
        self.ctx.as_ref().as_ref()
    }
}

impl AsRef<OCIError> for Session<'_> {
    fn as_ref(&self) -> &OCIError {
        self.ctx.as_ref().as_ref()
    }
}

impl AsRef<OCISvcCtx> for Session<'_> {
    fn as_ref(&self) -> &OCISvcCtx {
        self.ctx.as_ref().as_ref()
    }
}

impl Ctx for Session<'_> {
    fn try_as_session(&self) -> Option<&OCISession> {
        Some(&self.usr)
    }
}

impl Session<'_> {
    fn set_attr<T: attr::AttrSet>(&self, attr_type: u32, attr_val: T) -> Result<()> {
        attr::set(attr_type, attr_val, OCI_HTYPE_SESSION, self.usr.as_ref(), self.as_ref())
    }

    fn get_attr<T: attr::AttrGet>(&self, attr_type: u32) -> Result<T> {
        attr::get(attr_type, OCI_HTYPE_SESSION, self.usr.as_ref(), self.as_ref())
    }

    pub(crate) fn get_svc(&self) -> Arc<SvcCtx> {
        self.ctx.clone()
    }



    /// Reports whether self is connected to the server
    pub fn is_connected(&self) -> Result<bool> {
        let srv : Ptr<OCIServer> = attr::get(OCI_ATTR_SERVER, OCI_HTYPE_SVCCTX, self.ctx.svc.as_ref(), self.as_ref())?;
        let status : u32 = attr::get(OCI_ATTR_SERVER_STATUS, OCI_HTYPE_SERVER, srv.as_ref(), self.as_ref())?;
        Ok(status == OCI_SERVER_NORMAL)
    }

    /// Reports whether connection is established in non-blocking mode.
    pub fn is_async(&self) -> Result<bool> {
        let srv : Ptr<OCIServer> = attr::get(OCI_ATTR_SERVER, OCI_HTYPE_SVCCTX, self.ctx.svc.as_ref(), self.as_ref())?;
        let mode : u8 = attr::get(OCI_ATTR_NONBLOCKING_MODE, OCI_HTYPE_SERVER, srv.as_ref(), self.as_ref())?;
        Ok(mode != 0)
    }

    /**
    Sets the statement cache size.

    The default value of the statement cache size is 20 statements, for a statement cache-enabled session.
    Statement caching can be enabled by setting the attribute to a nonzero size and disabled by setting it to zero.

    # Parameters

    * `num_stmts` - Statement cache size

    # Example

    ```
    # use sibyl::Result;
    # #[cfg(feature="blocking")]
    # fn main() -> Result<()> {
    # let session = sibyl::test_env::get_session()?;
    session.set_stmt_cache_size(100)?;
    # let size = session.stmt_cache_size()?;
    # assert_eq!(size, 100);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() -> Result<()> {
    # sibyl::block_on(async {
    # let session = sibyl::test_env::get_session().await?;
    # session.set_stmt_cache_size(100)?;
    # let size = session.stmt_cache_size()?;
    # assert_eq!(size, 100);
    # Ok(()) })
    # }
    ```
    */
    pub fn set_stmt_cache_size(&self, num_stmts: u32) -> Result<()> {
        let ctx : &OCISvcCtx = self.as_ref();
        attr::set(OCI_ATTR_STMTCACHESIZE, num_stmts, OCI_HTYPE_SVCCTX, ctx, self.as_ref())
    }

    /**
    Returns the statement cache size.

    ```
    # use sibyl::Result;
    # #[cfg(feature="blocking")]
    # fn main() -> Result<()> {
    # let session = sibyl::test_env::get_session()?;
    let size = session.stmt_cache_size()?;
    assert_eq!(size, 20);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() -> Result<()> {
    # sibyl::block_on(async {
    # let session = sibyl::test_env::get_session().await?;
    # let size = session.stmt_cache_size()?;
    # assert_eq!(size, 20);
    # Ok(()) })
    # }
    ```
    */
    pub fn stmt_cache_size(&self) -> Result<u32> {
        let ctx : &OCISvcCtx = self.as_ref();
        attr::get(OCI_ATTR_STMTCACHESIZE, OCI_HTYPE_SVCCTX, ctx, self.as_ref())
    }

    /**
    Sets the time (in milliseconds) for a database round-trip call to time out. When the call times out,
    a network timeout error is returned. Setting this value stays effective for all subsequent round-trip
    calls until a different value is set. To remove the timeout, the value must be set to 0.

    The call timeout is applied to each individual round-trip between OCI and Oracle database. Each OCI
    method or operation may require zero or more round-trips to Oracle database. The timeout value applies
    to each round-trip individually, not to the sum of all round-trips. Time spent processing in OCI before
    or after the completion of each round-trip is not counted.

    # Parameters

    * `timeout` - The time (in milliseconds) for a database round-trip call to time out.

    # Example

    ```
    # use sibyl::Result;
    # #[cfg(feature="blocking")]
    # fn main() -> Result<()> {
    # let session = sibyl::test_env::get_session()?;
    session.set_call_timeout(5000)?;
    # let time = session.call_timeout()?;
    # assert_eq!(time, 5000);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() -> Result<()> {
    # sibyl::block_on(async {
    # let session = sibyl::test_env::get_session().await?;
    # session.set_call_timeout(5000)?;
    # let time = session.call_timeout()?;
    # assert_eq!(time, 5000);
    # Ok(()) })
    # }
    */
    pub fn set_call_timeout(&self, timeout: u32) -> Result<()> {
        let ctx : &OCISvcCtx = self.as_ref();
        attr::set(OCI_ATTR_CALL_TIMEOUT, timeout, OCI_HTYPE_SVCCTX, ctx, self.as_ref())
    }

    /**
    Returns time (in milliseconds) for a database round-trip call to time out.

    # Example

    ```
    # use sibyl::Result;
    # #[cfg(feature="blocking")]
    # fn main() -> Result<()> {
    # let session = sibyl::test_env::get_session()?;
    session.set_call_timeout(1000)?;

    let time = session.call_timeout()?;

    assert_eq!(time, 1000);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() -> Result<()> {
    # sibyl::block_on(async {
    # let session = sibyl::test_env::get_session().await?;
    # session.set_call_timeout(1000)?;
    # let time = session.call_timeout()?;
    # assert_eq!(time, 1000);
    # Ok(()) })
    # }
    */
    pub fn call_timeout(&self) -> Result<u32> {
        let ctx : &OCISvcCtx = self.as_ref();
        attr::get(OCI_ATTR_CALL_TIMEOUT, OCI_HTYPE_SVCCTX, ctx, self.as_ref())
    }

    /**
    Causes the server to measure call time, in milliseconds, for each subsequent OCI call.
    */
    pub fn start_call_time_measurements(&self) -> Result<()> {
        self.set_attr(OCI_ATTR_COLLECT_CALL_TIME, 1u32)
    }

    /**
    Returns the server-side time for the preceding call in microseconds.

    # Example

    🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
    to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest).

    ```
    # use sibyl::Result;
    # #[cfg(feature="blocking")]
    # fn main() -> Result<()> {
    # let session = sibyl::test_env::get_session()?;
    session.start_call_time_measurements()?;
    session.ping()?;
    let dt = session.call_time()?;
    session.stop_call_time_measurements()?;
    assert!(dt > 0);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() -> Result<()> {
    # sibyl::block_on(async {
    # let session = sibyl::test_env::get_session().await?;
    # session.start_call_time_measurements()?;
    # session.ping().await?;
    # let dt = session.call_time()?;
    # session.stop_call_time_measurements()?;
    # assert!(dt > 0);
    # Ok(()) })
    # }
    ```
    */
    pub fn call_time(&self) -> Result<u64> {
        self.get_attr(OCI_ATTR_CALL_TIME)
    }

    /// Terminates call time measurements.
    pub fn stop_call_time_measurements(&self) -> Result<()> {
        self.set_attr(OCI_ATTR_COLLECT_CALL_TIME, 0u32)
    }

    /**
    Sets the name of the current module (`V$SESSION.MODULE`) running in the client application.
    When the current module terminates, call with the name of the new module, or use empty
    string if there is no new module. The name can be up to 48 bytes long.

    # Parameters

    * `name` - The name of the current module running in the client application.

    # Example

    🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
    to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest).

    ```
    # use sibyl::Result;
    # #[cfg(feature="blocking")]
    # fn main() -> Result<()> {
    # let session = sibyl::test_env::get_session()?;
    session.set_module("Sibyl DocTest");

    let stmt = session.prepare("
        SELECT module
          FROM v$session
         WHERE sid = SYS_CONTEXT('USERENV', 'SID')
    ")?;
    let row = stmt.query_single(())?.unwrap();
    let module : &str = row.get(0)?;
    assert_eq!(module, "Sibyl DocTest");
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() -> Result<()> {
    # sibyl::block_on(async {
    # let session = sibyl::test_env::get_session().await?;
    # session.set_module("Sibyl DocTest");
    # let stmt = session.prepare("
    #     SELECT module
    #       FROM v$session
    #      WHERE sid = SYS_CONTEXT('USERENV', 'SID')
    # ").await?;
    # let row = stmt.query_single(()).await?.unwrap();
    # let module : &str = row.get(0)?;
    # assert_eq!(module, "Sibyl DocTest");
    # Ok(()) })
    # }
    ```
    */
    pub fn set_module(&self, name: &str) -> Result<()> {
        self.set_attr(OCI_ATTR_MODULE, name)
    }

    /**
    Sets the name of the current action (`V$SESSION.ACTION`) within the current module.
    When the current action terminates, set this attribute again with the name of the
    next action, or empty string if there is no next action. Can be up to 32 bytes long.

    # Parameters

    * `action` - The name of the current action within the current module.

    # Example

    🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
    to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest).

    ```
    # use sibyl::Result;
    # #[cfg(feature="blocking")]
    # fn main() -> Result<()> {
    # let session = sibyl::test_env::get_session()?;
    session.set_action("Action Name Test");

    let stmt = session.prepare("
        SELECT action
          FROM v$session
         WHERE sid = SYS_CONTEXT('USERENV', 'SID')
    ")?;
    let row = stmt.query_single(())?.unwrap();
    let action : &str = row.get(0)?;
    assert_eq!(action, "Action Name Test");
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() -> Result<()> {
    # sibyl::block_on(async {
    # let session = sibyl::test_env::get_session().await?;
    # session.set_action("Action Name Test");
    # let stmt = session.prepare("
    #     SELECT action
    #       FROM v$session
    #      WHERE sid = SYS_CONTEXT('USERENV', 'SID')
    # ").await?;
    # let row = stmt.query_single(()).await?.unwrap();
    # let action : &str = row.get(0)?;
    # assert_eq!(action, "Action Name Test");
    # Ok(()) })
    # }
    ```
    */
    pub fn set_action(&self, action: &str) -> Result<()> {
        self.set_attr(OCI_ATTR_ACTION, action)
    }

    /**
    Sets the user identifier (`V$SESSION.CLIENT_IDENTIFIER`) in the session handle.
    Can be up to 64 bytes long.

    # Parameters

    # `id` - The user identifier.

    # Example

    🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
    to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest).

    ```
    # use sibyl::Result;
    # #[cfg(feature="blocking")]
    # fn main() -> Result<()> {
    # let session = sibyl::test_env::get_session()?;
    session.set_client_identifier("Test Wielder");

    let stmt = session.prepare("
        SELECT client_identifier
            FROM v$session
            WHERE sid = SYS_CONTEXT('USERENV', 'SID')
    ")?;
    let row = stmt.query_single(())?.unwrap();
    let client_identifier : &str = row.get(0)?;
    assert_eq!(client_identifier, "Test Wielder");
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() -> Result<()> {
    # sibyl::block_on(async {
    # let session = sibyl::test_env::get_session().await?;
    # session.set_client_identifier("Test Wielder");
    # let stmt = session.prepare("
    #     SELECT client_identifier
    #       FROM v$session
    #      WHERE sid = SYS_CONTEXT('USERENV', 'SID')
    # ").await?;
    # let row = stmt.query_single(()).await?.unwrap();
    # let client_identifier : &str = row.get(0)?;
    # assert_eq!(client_identifier, "Test Wielder");
    # Ok(()) })
    # }
    ```
    */
    pub fn set_client_identifier(&self, id: &str) -> Result<()> {
        self.set_attr(OCI_ATTR_CLIENT_IDENTIFIER, id)
    }

    /**
    Sets additional client application information (`V$SESSION.CLIENT_INFO`).
    Can be up to 64 bytes long.

    # Parameters

    * `info` - Additional client application information.

    # Example

    🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
    to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest).

    ```
    # use sibyl::Result;
    # #[cfg(feature="blocking")]
    # fn main() -> Result<()> {
    # let session = sibyl::test_env::get_session()?;
    session.set_client_info("Nothing to see here, move along folks");

    let stmt = session.prepare("
        SELECT client_info
          FROM v$session
         WHERE sid = SYS_CONTEXT('USERENV', 'SID')
    ")?;
    let row = stmt.query_single(())?.unwrap();
    let client_info : &str = row.get(0)?;
    assert_eq!(client_info, "Nothing to see here, move along folks");
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() -> Result<()> {
    # sibyl::block_on(async {
    # let session = sibyl::test_env::get_session().await?;
    # session.set_client_info("Nothing to see here, move along folks");
    # let stmt = session.prepare("
    #     SELECT client_info
    #       FROM v$session
    #      WHERE sid = SYS_CONTEXT('USERENV', 'SID')
    # ").await?;
    # let row = stmt.query_single(()).await?.unwrap();
    # let client_info : &str = row.get(0)?;
    # assert_eq!(client_info, "Nothing to see here, move along folks");
    # Ok(()) })
    # }
    ```
    */
    pub fn set_client_info(&self, info: &str) -> Result<()> {
        self.set_attr(OCI_ATTR_CLIENT_INFO, info)
    }

    /**
    Returns the current schema.

    # Example

    ```
    # use sibyl::Result;
    # #[cfg(feature="blocking")]
    # fn main() -> Result<()> {
    # let session = sibyl::test_env::get_session()?;
    let orig_name = session.current_schema()?;
    // Workaround for an isssue that was introduced by the instant client 19.15 -
    // `current_schema` returns empty string until set by `set_current_schema`
    // Client 19.13 and earlier return the schema's name upon connect.
    let dbuser = std::env::var("DBUSER").expect("user name");
    let orig_name = if orig_name.len() > 0 { orig_name } else { dbuser.as_str() };
    session.set_current_schema("HR")?;

    let current_schema = session.current_schema()?;

    assert_eq!(current_schema, "HR");
    session.set_current_schema(orig_name)?;
    let current_schema = session.current_schema()?;
    assert_eq!(current_schema, orig_name);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() -> Result<()> {
    # sibyl::block_on(async {
    # let session = sibyl::test_env::get_session().await?;
    # let orig_name = session.current_schema()?;
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let orig_name = if orig_name.len() > 0 { orig_name } else { dbuser.as_str() };
    # session.set_current_schema("HR")?;
    # let current_schema = session.current_schema()?;
    # assert_eq!(current_schema, "HR");
    # session.set_current_schema(orig_name)?;
    # let current_schema = session.current_schema()?;
    # assert_eq!(current_schema, orig_name);
    # Ok(()) })
    # }
    ```
    */
    pub fn current_schema(&self) -> Result<&str> {
        self.get_attr(OCI_ATTR_CURRENT_SCHEMA)
    }

    /**
    Sets the current schema. It has the same effect as the SQL command `ALTER SESSION SET CURRENT_SCHEMA`
    if the schema name and the session exist. The schema is altered on the next OCI call that does a
    round-trip to the server, avoiding an extra round-trip. If the new schema name does not exist, the
    same error is returned as the error returned from ALTER SESSION SET CURRENT_SCHEMA. The new schema
    name is placed before database objects in DML or DDL commands that you then enter.

    # Parameters

    * `schema_name` - The new schema name.

    # Example

    🛈 **Note** that this example is written for `blocking` mode execution. Add `await`s, where needed,
    to convert it to a nonblocking variant (or peek at the source to see the hidden nonblocking doctest).

    ```
    # use sibyl::Result;
    # #[cfg(feature="blocking")]
    # fn main() -> Result<()> {
    # let session = sibyl::test_env::get_session()?;
    let orig_name = session.current_schema()?;
    // Workaround for an isssue that was introduced by the instant client 19.15 -
    // `current_schema` returns empty string until set by `set_current_schema`
    // Client 19.13 and earlier return the schema's name upon connect.
    let dbuser = std::env::var("DBUSER").expect("user name");
    let orig_name = if orig_name.len() > 0 { orig_name } else { dbuser.as_str() };

    session.set_current_schema("HR")?;

    assert_eq!(session.current_schema()?, "HR", "current schema is HR now");
    let stmt = session.prepare("
        SELECT schemaname
          FROM v$session
         WHERE sid = SYS_CONTEXT('USERENV', 'SID')
    ")?;
    let row = stmt.query_single(())?.unwrap();
    let schema_name : &str = row.get(0)?;
    assert_eq!(schema_name, "HR", "v$session reports schema as HR");

    session.set_current_schema(orig_name)?;
    assert_eq!(session.current_schema()?, orig_name, "current schema is restored");
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() -> Result<()> {
    # sibyl::block_on(async {
    # let session = sibyl::test_env::get_session().await?;
    # let orig_name = session.current_schema()?;
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let orig_name = if orig_name.len() > 0 { orig_name } else { dbuser.as_str() };
    # session.set_current_schema("HR")?;
    # assert_eq!(session.current_schema()?, "HR");
    # let stmt = session.prepare("
    #     SELECT schemaname
    #       FROM v$session
    #      WHERE sid = SYS_CONTEXT('USERENV', 'SID')
    # ").await?;
    # let row = stmt.query_single(()).await?.unwrap();
    # let schema_name : &str = row.get(0)?;
    # assert_eq!(schema_name, "HR");
    # session.set_current_schema(orig_name)?;
    # assert_eq!(session.current_schema()?, orig_name);
    # Ok(()) })
    # }
    ```
    */
    pub fn set_current_schema(&self, schema_name: &str) -> Result<()> {
        self.set_attr(OCI_ATTR_CURRENT_SCHEMA, schema_name)
    }

    /**
    Sets the default prefetch buffer size for each LOB locator.

    This attribute value enables prefetching for all the LOB locators fetched in the session.
    The default value for this attribute is zero (no prefetch of LOB data). This option
    relieves the application developer from setting the prefetch LOB size for each LOB column
    in each prepared statement.
    */
    pub fn set_lob_prefetch_size(&self, size: u32) -> Result<()> {
        self.set_attr(OCI_ATTR_DEFAULT_LOBPREFETCH_SIZE, size)
    }

    /// Returns the default prefetch buffer size for each LOB locator.
    pub fn lob_prefetch_size(&self) -> Result<u32> {
        self.get_attr(OCI_ATTR_DEFAULT_LOBPREFETCH_SIZE)
    }
}
