//! Connection pool blocking mode implementation

use super::ConnectionPool;
use crate::{Result, env::Env, oci::{self, *}, Environment, Connection};
use std::{ptr, marker::PhantomData};

impl<'a> ConnectionPool<'a> {
    pub(crate) fn new(env: &'a Environment, dbname: &str, username: &str, password: &str, min: usize, inc: usize, max: usize) -> Result<Self> {
        let err = Handle::<OCIError>::new(env.env_ptr())?;

        let pool = Handle::<OCICPool>::new(env.env_ptr())?;
        let mut pool_name_ptr = ptr::null::<u8>();
        let mut pool_name_len = 0u32;
        oci::connection_pool_create(
            env.env_ptr(), env.err_ptr(), pool.get(),
            &mut pool_name_ptr, &mut pool_name_len,
            dbname.as_ptr(), dbname.len() as u32,
            min as u32, max as u32, inc as u32,
            username.as_ptr(), username.len() as u32,
            password.as_ptr(), password.len() as u32,
            OCI_DEFAULT
        )?;
        let name = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(pool_name_ptr, pool_name_len as usize)) };

        Ok(Self {env: env.clone_env(), err, pool, name, phantom_env: PhantomData})
    }

    pub(crate) fn get_svc_ctx(&self, username: &str, password: &str) -> Result<Ptr<OCISvcCtx>> {
        let inf = Handle::<OCIAuthInfo>::new(self.env.get())?;
        inf.set_attr(OCI_ATTR_DRIVER_NAME, "sibyl", self.err.get())?;
        inf.set_attr(OCI_ATTR_USERNAME, username, self.err.get())?;
        inf.set_attr(OCI_ATTR_PASSWORD, password, self.err.get())?;
        let mut svc = Ptr::null();
        let mut found = 0u8;
        oci::session_get(
            self.env.get(), self.err.get(), svc.as_mut_ptr(), inf.get(), self.name.as_ptr(), self.name.len() as u32,
            ptr::null(), 0, ptr::null_mut(), ptr::null_mut(), &mut found,
            OCI_SESSGET_CPOOL | OCI_SESSGET_STMTCACHE
        )?;
        Ok(svc)
    }

    pub fn get_session(&self, user: &str, pass: &str) -> Result<Connection> {
        Connection::from_connection_pool(self, user, pass)
    }
}

impl Drop for ConnectionPool<'_> {
    fn drop(&mut self) {
        unsafe {
            OCIConnectionPoolDestroy(self.pool.get(), self.err.get(), OCI_DEFAULT);
        }
    }
}
