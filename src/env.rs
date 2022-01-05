//! OCI environment

#[cfg(feature="blocking")]
#[cfg_attr(docsrs, doc(cfg(feature="blocking")))]
mod blocking;

#[cfg(feature="nonblocking")]
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
mod nonblocking;

use std::{ptr, sync::Arc};

use crate::{Error, Result, oci::*, types::Ctx};

/// Represents an OCI environment.
pub struct Environment {
    // `OCIEnv` handle must be behind Arc as it needs to survive the Environment drop,
    // so that `OCIEnv` is still available to async-drop used, for example, in `Session`.
    env: Arc<Handle<OCIEnv>>,
    err: Handle<OCIError>,
}

impl AsRef<OCIEnv> for Environment {
    fn as_ref(&self) -> &OCIEnv {
        &*self.env
    }
}

impl AsRef<OCIError> for Environment {
    fn as_ref(&self) -> &OCIError {
        &*self.err
    }
}

impl Ctx for Environment {
    fn try_as_session(&self) -> Option<&OCISession> {
        None
    }
}

impl Environment {
    /**
    Returns a new environment handle, which is then used by the OCI functions.

    # Example

    ```
    use sibyl::Environment;

    let oracle = Environment::new()?;

    # Ok::<(),sibyl::Error>(())
    ```
    */
    pub fn new() -> Result<Self> {
        let mut env = Ptr::<OCIEnv>::null();
        let res = unsafe {
            OCIEnvNlsCreate(
                env.as_mut_ptr(), OCI_OBJECT | OCI_THREADED,
                ptr::null(), ptr::null(), ptr::null(), ptr::null(), 0, ptr::null(),
                AL32UTF8, UTF8
            )
        };
        if res != OCI_SUCCESS {
            return Err( Error::new("Cannot create OCI environment") );
        }
        let env = Handle::from(env);
        let err = Handle::<OCIError>::new(&env)?;
        let env = Arc::new(env);
        Ok(Self { env, err })
    }

    pub(crate) fn get_env(&self) -> Arc<Handle<OCIEnv>> {
        self.env.clone()
    }

    fn get_attr<V: attr::AttrGet>(&self, attr_type: u32) -> Result<V> {
        self.env.get_attr(attr_type, self.as_ref())
    }

    fn get_attr_into<V: attr::AttrGetInto>(&self, attr_type: u32, into: &mut V) -> Result<()> {
        self.env.get_attr_into(attr_type, into, self.as_ref())
    }

    fn set_attr<V: attr::AttrSet>(&self, attr_type: u32, attr_val: V) -> Result<()> {
        self.env.set_attr(attr_type, attr_val, self.as_ref())
    }

    /**
    Returns the maximum size (high watermark) for the client-side object cache
    as a percentage of the optimal size.

    # Example

    ```
    let oracle = sibyl::env()?;

    let max_size_percentage = oracle.max_cache_size()?;

    assert_eq!(max_size_percentage, 10);
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn max_cache_size(&self) -> Result<u32> {
        self.get_attr(OCI_ATTR_CACHE_MAX_SIZE)
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

    ```
    # fn example(optimal_size: u32, max_size_percentage: u32) -> u32 {
    let maximum_cache_size = optimal_size + optimal_size * max_size_percentage / 100;
    # maximum_cache_size }
    ```

    # Parameters

    * `size` - The maximum size for the client-side object cache as a oercentage of the cache optimal size.

    # Example

    ```
    let oracle = sibyl::env()?;

    oracle.set_cache_max_size(30)?;

    let max_size_percentage = oracle.max_cache_size()?;
    assert_eq!(max_size_percentage, 30);
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn set_cache_max_size(&self, size: u32) -> Result<()> {
        self.set_attr(OCI_ATTR_CACHE_MAX_SIZE, size)
    }

    /**
    Returns the optimal size for the client-side object cache in bytes.

    # Example

    ```
    let oracle = sibyl::env()?;

    let optimal_size = oracle.optimal_cache_size()?;

    assert_eq!(optimal_size, 8*1024*1024);
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn optimal_cache_size(&self) -> Result<u32> {
        self.get_attr(OCI_ATTR_CACHE_OPT_SIZE)
    }

    /**
        Sets the optimal size for the client-side object cache in bytes. The default value is 8 megabytes (MB).
        Setting this attribute to 0 results in a value of 8 MB being used.

        # Parameters

        * `size` - The optimal size of the client-side object cache in bytes

        # Example

        ```
        let oracle = sibyl::env()?;

        oracle.set_cache_opt_size(64*1024*1024)?;

        let optimal_size = oracle.optimal_cache_size()?;
        assert_eq!(optimal_size, 64*1024*1024);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn set_cache_opt_size(&self, size: u32) -> Result<()> {
        self.set_attr(OCI_ATTR_CACHE_OPT_SIZE, size)
    }

    /**
    Returns the name of the language used for the database sessions created in the current environment.

    See [Database Globalization Support Guide / Locale Data / Languages][1]

    [1]: https://docs.oracle.com/en/database/oracle/oracle-database/19/nlspg/appendix-A-locale-data.html#GUID-D2FCFD55-EDC3-473F-9832-AAB564457830

    # Example

    ```
    let oracle = sibyl::env()?;

    let lang = oracle.nls_language()?;

    assert_eq!(lang, "AMERICAN");
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn nls_language(&self) -> Result<String> {
        let mut lang = String::with_capacity(32);
        self.get_attr_into(OCI_ATTR_ENV_NLS_LANGUAGE, &mut lang)?;
        Ok(lang)
    }

    /**
    Sets the language used for the database sessions created in the current environment.

    # Parameters

    * `lang` - The name of the language used for the database sessions

    # Example

    ```
    let oracle = sibyl::env()?;

    oracle.set_nls_language("ENGLISH")?;

    let lang = oracle.nls_language()?;
    assert_eq!(lang, "ENGLISH");
    # Ok::<(),Box<dyn std::error::Error>>(())
    */
    pub fn set_nls_language(&self, lang: &str) -> Result<()> {
        self.set_attr(OCI_ATTR_ENV_NLS_LANGUAGE, lang)
    }

    /**
    Returns the name of the territory used for the database sessions created in the current environment.

    See [Database Globalization Support Guide / Locale Data / Territories][1]

    [1]: https://docs.oracle.com/en/database/oracle/oracle-database/19/nlspg/appendix-A-locale-data.html#GUID-550D6A25-DB53-4911-9419-8065A73FFB06

    # Example

    ```
    let oracle = sibyl::env()?;

    let territory = oracle.nls_territory()?;

    assert_eq!(territory, "AMERICA");
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn nls_territory(&self) -> Result<String> {
        let mut territory = String::with_capacity(24);
        self.get_attr_into(OCI_ATTR_ENV_NLS_TERRITORY, &mut territory)?;
        Ok(territory)
    }

    /**
    Sets the name of the territory used for the database sessions created in the current environment.

    # Parameters

    * `territory` - The name of the territory used for the database sessions

    # Example

    ```
    let oracle = sibyl::env()?;
    oracle.set_nls_territory("CANADA")?;
    let territory = oracle.nls_territory()?;

    assert_eq!(territory, "CANADA");
    # Ok::<(),Box<dyn std::error::Error>>(())
    ```
    */
    pub fn set_nls_territory(&self, territory: &str) -> Result<()> {
        self.set_attr(OCI_ATTR_ENV_NLS_TERRITORY, territory)
    }
}
