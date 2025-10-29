//! Blocking mode OCI environment methods.
 
use super::Environment;
use crate::{Session, ConnectionPool, Result, SessionPool, oci::{OCI_DEFAULT, OCI_SESSGET_SYSDBA}};

impl Environment {
    /**
    Creates and begins a session.

    # Parameters

    * `dbname` - The TNS alias of the database to connect to.
    * `username` - The user ID with which to start the sessions.
    * `password` - The password for the corresponding `username`.

    # Example
    ```
    let oracle = sibyl::env()?;

    let dbname = std::env::var("DBNAME")?;
    let dbuser = std::env::var("DBUSER")?;
    let dbpass = std::env::var("DBPASS")?;

    let session = oracle.connect(&dbname, &dbuser, &dbpass)?;

    assert!(!session.is_async()?);
    assert!(session.is_connected()?);
    assert!(session.ping().is_ok());

    let stmt = session.prepare("
        SELECT DISTINCT client_driver
          FROM v$session_connect_info
         WHERE sid = SYS_CONTEXT('USERENV', 'SID')
    ")?;
    let row = stmt.query_single(())?.unwrap();
    let client_driver : &str = row.get(0)?;
    assert_eq!(client_driver, "sibyl");
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn connect(&self, dbname: &str, username: &str, password: &str) -> Result<Session<'_>> {
        Session::new(self, dbname, username, password, OCI_DEFAULT)
    }

    /**
    Creates and begins a SYSDBA session.

    # Parameters

    * `dbname` - The TNS alias of the database to connect to.
    * `username` - The userid with which to start the sessions.
    * `password` - The password for the corresponding `username`.

    ## Note

    The specified user must have SYSDBA role granted.

    # Example

    ```
    let oracle = sibyl::env()?;

    let dbname = std::env::var("DBNAME").expect("database name");
    let dbuser = std::env::var("DBAUSER").expect("name of the user with SYSDBA role");
    let dbpass = std::env::var("DBAPASS").expect("SYSDBA user password");

    let session = oracle.connect_as_sysdba(&dbname, &dbuser, &dbpass)?;

    let stmt = session.prepare("SELECT Count(*) FROM TS$")?;
    let row = stmt.query_single(())?.expect("single row");
    let num_ts: u32 = row.get(0)?;

    assert!(num_ts > 0);
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn connect_as_sysdba(&self, dbname: &str, username: &str, password: &str) -> Result<Session<'_>> {
        Session::new(self, dbname, username, password, OCI_SESSGET_SYSDBA)
    }

    /**
    Creates new session pool.

    # Parameters

    * `dbname` - The TNS alias of the database to connect to.
    * `username` - The username with which to start the sessions.
    * `password` - The password for the corresponding `username`.
    * `min` - The minimum number of sessions in the session pool. This number of sessions will be started
        during pool creation. After `min` sessions are started, sessions are opened only when necessary.
    * `inc` - The next increment for sessions to be started if the current number of sessions is less
        than `max`. The valid values are 0 and higher.
    * `max` - The maximum number of sessions that can be opened in the session pool. After this value is
        reached, no more sessions are opened. The valid values are 1 and higher.

    # Example

    ```
    let oracle = sibyl::env()?;

    let dbname = std::env::var("DBNAME")?;
    let dbuser = std::env::var("DBUSER")?;
    let dbpass = std::env::var("DBPASS")?;

    // Create a session pool where each session will connect to the database
    // `dbname` and authenticate itself as `dbuser` with password `dbpass`.
    // Pool will have no open sessions initially. It will create 1 new session
    // at a time, up to the maximum of 10 sessions, when they are requested
    // and there are no idle sessions in the pool.
    let pool = oracle.create_session_pool(&dbname, &dbuser, &dbpass, 0, 1, 10)?;

    let session = pool.get_session()?;
    let stmt = session.prepare("
        SELECT DISTINCT client_driver
          FROM v$session_connect_info
         WHERE sid = SYS_CONTEXT('USERENV', 'SID')
    ")?;
    let row = stmt.query_single(())?.unwrap();
    let client_driver : &str = row.get(0)?;
    assert_eq!(client_driver, "sibyl");
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn create_session_pool(&self, dbname: &str, username: &str, password: &str, min: usize, inc: usize, max: usize) -> Result<SessionPool<'_>> {
        SessionPool::new(self, dbname, username, password, min, inc, max, "")
    }

    /**
    Creates new session pool that uses a PL/SQL callback to fix the returned session state on the server
    before it is returned, thus avoiding, potentially multiple, roundtrips to the database for the fix-up
    logic that would be normally used when [`SessionPool::get_tagged_session()`] returns a default session.

    # Parameters

    * `dbname`   - The TNS alias of the database to connect to.
    * `username` - The username with which to start the sessions.
    * `password` - The password for the corresponding `username`.
    * `min`      - The minimum number of sessions in the session pool. This number of sessions will be started
                   during pool creation. After `min` sessions are started, sessions are opened only when necessary.
    * `inc`      - The next increment for sessions to be started if the current number of sessions is less
                   than `max`. The valid values are 0 and higher.
    * `max`      - The maximum number of sessions that can be opened in the session pool. After this value is
                   reached, no more sessions are opened. The valid values are 1 and higher.
    * `session_state_fixup_callback` - [PL/SQL Callback for Session State Fix Up](https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/session-and-connection-pooling.html#GUID-B853A020-752F-494A-8D88-D0396EF57177)
                   provided in the format `schema.package.callback_function`.

    # Example

    ```
    # let oracle = sibyl::env()?;
    # let dbname = std::env::var("DBNAME").expect("database name");
    # let dbuser = std::env::var("DBUSER").expect("user name");
    # let dbpass = std::env::var("DBPASS").expect("password");
    let pool = oracle.create_session_pool_with_session_state_fixup(
        &dbname, &dbuser, &dbpass, 0, 1, 10,
        "sibyl.SessionCustomizer.FixSessionState" // see package definition below
    )?;

    let (session, found) = pool.get_tagged_session("TIME_ZONE=UTC;NLS_DATE_FORMAT=YYYY-MM-DD")?;
    // Session will always be found
    assert!(found,"pool fixed up the returned session to match the requested tag");

    let stmt = session.prepare("SELECT SessionTimeZone FROM dual")?;
    let row = stmt.query_single(())?.expect("one row");
    let stz: &str = row.get(0)?;
    assert_eq!(stz, "UTC", "Session is in UTC");

    let stmt = session.prepare("SELECT value FROM nls_session_parameters WHERE parameter=:PARAM_NAME")?;
    let row = stmt.query_single("NLS_DATE_FORMAT")?.expect("one row");
    let fmt: &str = row.get(0)?;
    assert_eq!(fmt, "YYYY-MM-DD", "Session uses custom NLS_DATE_FORMAT");
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```

    Create the following package in the `sibyl` schema (for the example above) before running the test:

    ```text
    --
    -- This package expects tag properties to be restricted to names and values
    -- that can be directly used in the ALTER SESSION statement, such as
    -- TIME_ZONE=UTC;NLS_DATE_FORMAT=YYYY-MM-DD
    --
    CREATE OR REPLACE PACKAGE SessionCustomizer AS
      TYPE prop_t IS TABLE OF VARCHAR2(256) INDEX BY VARCHAR2(120);
      PROCEDURE ParseTag (tag VARCHAR2, properties OUT prop_t);
      PROCEDURE FixSessionState (requested_tag VARCHAR2, actual_tag VARCHAR2);
    END;
    /

    CREATE OR REPLACE PACKAGE BODY SessionCustomizer AS
      PROCEDURE ParseTag (tag VARCHAR2, properties OUT prop_t) IS
        semi_pos  INT := 0;
        name_pos  INT;
        equal_pos INT;
        name      VARCHAR2(120);
        value     VARCHAR2(256);
      BEGIN
        WHILE semi_pos <= Length(tag) LOOP
          name_pos := semi_pos + 1;
          semi_pos := InStr(tag, ';', semi_pos + 1);
          IF semi_pos = 0 THEN
            semi_pos := Length(tag) + 1;
          END IF;
          equal_pos := InStr(tag, '=', name_pos + 1);
          IF equal_pos != 0 AND equal_pos + 1 < semi_pos THEN
            name  := SubStr(tag, name_pos, equal_pos - name_pos);
            value := SubStr(tag, equal_pos + 1, semi_pos - equal_pos - 1);
            properties(name) := value;
          END IF;
        END LOOP;
      END;

      PROCEDURE FixSessionState (requested_tag VARCHAR2, actual_tag VARCHAR2) IS
        req_props prop_t;
        act_props prop_t;
        prop_name VARCHAR2(120);
      BEGIN
        ParseTag(requested_tag, req_props);
        ParseTag(actual_tag, act_props);

        prop_name := req_props.FIRST;
        WHILE prop_name IS NOT NULL LOOP
          IF NOT act_props.EXISTS(prop_name) OR act_props(prop_name) != req_props(prop_name) THEN
            EXECUTE IMMEDIATE 'ALTER SESSION SET ' || prop_name || '=''' || req_props(prop_name) || '''';
          END IF;
          prop_name := req_props.NEXT(prop_name);
        END LOOP;

        -- Maybe also reset to default act_props that are not in req_props
      END;
    END;
    /
    ```
    */
    pub fn create_session_pool_with_session_state_fixup(&self, dbname: &str, username: &str, password: &str, min: usize, inc: usize, max: usize, session_state_fixup_callback: &str) -> Result<SessionPool<'_>> {
        SessionPool::new(self, dbname, username, password, min, inc, max, session_state_fixup_callback)
    }

    /**
    Creates new connection pool.

    # Parameters

    * `dbname` - The TNS alias of the database to connect to.
    * `username` - The username with which to start the sessions.
    * `password` - The password for the corresponding `username`.
    * `min` - The minimum number of connections to be opened when the pool is created. After the connection pool is created,
        connections are opened only when necessary. Generally, this parameter should be set to the number of concurrent statements
        that the application is planning or expecting to run.
    * `inc` - incremental number of connections to be opened when all the connections are busy and a call needs a connection.
        This increment is used only when the total number of open connections is less than the maximum number of connections
        that can be opened in that pool.
    * `max` - The maximum number of connections that can be opened to the database. When the maximum number of connections
        are open and all the connections are busy, if a call needs a connection, it waits until it gets one.

    # Example

    ```
    let oracle = sibyl::env()?;

    let dbname = std::env::var("DBNAME")?;
    let dbuser = std::env::var("DBUSER")?;
    let dbpass = std::env::var("DBPASS")?;

    let pool = oracle.create_connection_pool(&dbname, &dbuser, &dbpass, 1, 1, 10)?;

    let session = pool.get_session(&dbuser, &dbpass)?;
    let stmt = session.prepare("
        SELECT DISTINCT client_driver
          FROM v$session_connect_info
         WHERE sid = SYS_CONTEXT('USERENV', 'SID')
    ")?;
    let row = stmt.query_single(())?.unwrap();
    let client_driver : &str = row.get(0)?;
    assert_eq!(client_driver, "sibyl");
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn create_connection_pool(&self, dbname: &str, username: &str, password: &str, min: usize, inc: usize, max: usize) -> Result<ConnectionPool<'_>> {
        ConnectionPool::new(self, dbname, username, password, min, inc, max)
    }
}
