//! OCI environment

use crate::*;
use crate::types::Ctx;
use libc::{ c_void, size_t };
use std::{ mem, ptr };

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/connect-authorize-and-initialize-functions.html#GUID-16BDA1F1-7DAF-41CA-9EE1-C9A4CB467244
    fn OCIEnvNlsCreate(
        envhpp:     *mut *mut  OCIEnv,
        mode:       u32,
        ctxp:       *const c_void,
        malocfp:    *const c_void,
        ralocfp:    *const c_void,
        mfreefp:    *const c_void,
        xtramemsz:  size_t,
        usrmempp:   *const c_void,
        charset:    u16,
        ncharset:   u16
    ) -> i32;
}

// Initialization Modes
const OCI_THREADED : u32 = 1;
const OCI_OBJECT   : u32 = 2;

const OCI_ATTR_CACHE_OPT_SIZE    : u32 = 34;
const OCI_ATTR_CACHE_MAX_SIZE    : u32 = 35;
const OCI_ATTR_ENV_NLS_LANGUAGE  : u32 = 424;
const OCI_ATTR_ENV_NLS_TERRITORY : u32 = 425;

fn create_environment() -> Result<Handle<OCIEnv>> {
    let mut env = mem::MaybeUninit::<*mut OCIEnv>::uninit();
    let res = unsafe {
        OCIEnvNlsCreate(
            env.as_mut_ptr(), OCI_OBJECT | OCI_THREADED,
            ptr::null(), ptr::null(), ptr::null(), ptr::null(), 0, ptr::null(),
            AL32UTF8, UTF8
        )
    };
    if res != OCI_SUCCESS {
        Err( Error::new("Cannot create OCI environment") )
    } else {
        let env = unsafe { env.assume_init() };
        Ok( Handle::from(env) )
    }
}

/// Represents an OCI environment.
pub struct Environment {
    err: Handle<OCIError>,
    env: Handle<OCIEnv>
}

impl Environment {
    pub(crate) fn new() -> Result<Self> {
        let env = create_environment()?;
        let err: Handle<OCIError> = Handle::new(env.get())?;
        Ok( Environment { env, err } )
    }
}

impl Environment {
    /**
        Returns the maximum size (high watermark) for the client-side object cache
        as a percentage of the optimal size.

        # Example
        ```
        let oracle = sibyl::env()?;
        let max_size_percentage = oracle.get_cache_max_size()?;

        assert_eq!(max_size_percentage, 10);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn get_cache_max_size(&self) -> Result<u32> {
        self.env.get_attr::<u32>(OCI_ATTR_CACHE_MAX_SIZE, self.err_ptr())
    }

    /**
        Sets the maximum size (high watermark) for the client-side object cache as a percentage
        of the optimal size. Usually, you can set this value at 10%, the default, of the optimal size.
        Setting this attribute to 0 results in a value of 10 being used. The object cache uses the
        maximum and optimal values for freeing unused memory in the object cache.

        If the memory occupied by the objects currently in the cache reaches or exceeds the maximum
        cache size, the cache automatically begins to free (or ages out) unmarked objects that have
        a pin count of zero. The cache continues freeing such objects until memory usage in the cache
        reaches the optimal size, or until it runs out of objects eligible for freeing. Note that the
        cache can grow beyond the specified maximum cache size.

        The maximum object cache size (in bytes) is computed by incrementing `optimal_size` by the
        `max_size_percentage`, using the following algorithm:
        ```ignore
        maximum_cache_size = optimal_size + optimal_size * max_size_percentage / 100
        ```
        # Example
        ```
        let oracle = sibyl::env()?;
        oracle.set_cache_max_size(30)?;
        let max_size_percentage = oracle.get_cache_max_size()?;

        assert_eq!(max_size_percentage, 30);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn set_cache_max_size(&self, size: u32) -> Result<()> {
        self.env.set_attr(OCI_ATTR_CACHE_MAX_SIZE, size, self.err_ptr())
    }

    /**
        Returns the optimal size for the client-side object cache in bytes.

        # Example
        ```
        let oracle = sibyl::env()?;
        let optimal_size = oracle.get_cache_opt_size()?;

        assert_eq!(optimal_size, 8*1024*1024);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn get_cache_opt_size(&self) -> Result<u32> {
        self.env.get_attr::<u32>(OCI_ATTR_CACHE_OPT_SIZE, self.err_ptr())
    }

    /**
        Sets the optimal size for the client-side object cache in bytes. The default value is 8 megabytes (MB).
        Setting this attribute to 0 results in a value of 8 MB being used.

        # Example
        ```
        let oracle = sibyl::env()?;
        oracle.set_cache_opt_size(64*1024*1024)?;
        let optimal_size = oracle.get_cache_opt_size()?;

        assert_eq!(optimal_size, 64*1024*1024);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn set_cache_opt_size(&self, size: u32) -> Result<()> {
        self.env.set_attr(OCI_ATTR_CACHE_OPT_SIZE, size, self.err_ptr())
    }

    /**
        Returns the name of the language used for the database sessions created in the current environment.

        See [Database Globalization Support Guide / Locale Data / Languages ][1]

        [1]: https://docs.oracle.com/en/database/oracle/oracle-database/19/nlspg/appendix-A-locale-data.html#GUID-D2FCFD55-EDC3-473F-9832-AAB564457830

        # Example
        ```
        let oracle = sibyl::env()?;
        let lang = oracle.get_nls_language()?;

        assert_eq!(lang, "AMERICAN");
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn get_nls_language(&self) -> Result<String> {
        let mut lang = String::with_capacity(32);
        self.env.get_attr_into(OCI_ATTR_ENV_NLS_LANGUAGE, &mut lang, self.err_ptr())?;
        Ok( lang )
    }

    /**
        Sets the language used for the database sessions created in the current environment.
        # Example
        ```
        let oracle = sibyl::env()?;
        oracle.set_nls_language("ENGLISH")?;
        let lang = oracle.get_nls_language()?;

        assert_eq!(lang, "ENGLISH");
        # Ok::<(),Box<dyn std::error::Error>>(())
    */
    pub fn set_nls_language(&self, lang: &str) -> Result<()> {
        self.env.set_attr(OCI_ATTR_ENV_NLS_LANGUAGE, lang, self.err_ptr())
    }

    /**
        Returns the name of the territory used for the database sessions created in the current environment.

        See [Database Globalization Support Guide / Locale Data / Territories ][1]

        [1]: https://docs.oracle.com/en/database/oracle/oracle-database/19/nlspg/appendix-A-locale-data.html#GUID-550D6A25-DB53-4911-9419-8065A73FFB06

        # Example
        ```
        let oracle = sibyl::env()?;
        let territory = oracle.get_nls_territory()?;

        assert_eq!(territory, "AMERICA");
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn get_nls_territory(&self) -> Result<String> {
        let mut territory = String::with_capacity(24);
        self.env.get_attr_into(OCI_ATTR_ENV_NLS_TERRITORY, &mut territory, self.err_ptr())?;
        Ok( territory )
    }

    /**
        Sets the name of the territory used for the database sessions created in the current environment.

        # Example
        ```
        let oracle = sibyl::env()?;
        oracle.set_nls_territory("CANADA")?;
        let territory = oracle.get_nls_territory()?;

        assert_eq!(territory, "CANADA");
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn set_nls_territory(&self, territory: &str) -> Result<()> {
        self.env.set_attr(OCI_ATTR_ENV_NLS_TERRITORY, territory, self.err_ptr())
    }

    /**
        Creates and begins a user session for a given server.
        # Example
        ```
        let oracle = sibyl::env()?;
        let dbname = std::env::var("DBNAME")?;
        let dbuser = std::env::var("DBUSER")?;
        let dbpass = std::env::var("DBPASS")?;
        let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;

        assert!(!conn.is_async()?);
        assert!(conn.is_connected()?);
        assert!(conn.ping().is_ok());

        let stmt = conn.prepare("
            SELECT DISTINCT client_driver
              FROM v$session_connect_info
             WHERE sid = SYS_CONTEXT('USERENV', 'SID')
        ")?;
        let mut rows = stmt.query(&[])?;
        let row = rows.next()?.unwrap();
        let client_driver : &str = row.get(0)?.unwrap();
        assert_eq!(client_driver, "sibyl");
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn connect(&self, dbname: &str, username: &str, password: &str) -> Result<Connection> {
        let conn = Connection::new(self)?;
        conn.attach(dbname)?;
        conn.login(username, password)?;
        Ok(conn)
    }
}

pub trait Env {
    fn env_ptr(&self) -> *mut OCIEnv;
    fn err_ptr(&self) -> *mut OCIError;
}

impl Env for Environment {
    fn env_ptr(&self) -> *mut OCIEnv {
        self.env.get()
    }

    fn err_ptr(&self) -> *mut OCIError {
        self.err.get()
    }
}

impl Ctx for Environment {
    fn as_ptr(&self) -> *mut c_void {
        self.env_ptr() as *mut c_void
    }
}
