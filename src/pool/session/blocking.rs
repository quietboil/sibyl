//! Session pool blocking mode implementation

use super::SessionPool;
use crate::{Connection, Environment, Result, env::Env, oci::{self, *}};
use std::ptr;

impl<'a> SessionPool<'a> {
    pub(crate) fn new(env: &'a Environment, dbname: &str, username: &str, password: &str, min: usize, inc: usize, max: usize) -> Result<Self> {
        let info = Handle::<OCIAuthInfo>::new(env.env_ptr())?;
        info.set_attr(OCI_ATTR_DRIVER_NAME, "sibyl", env.err_ptr())?;
        let pool = Handle::<OCISPool>::new(env.env_ptr())?;
        pool.set_attr(OCI_ATTR_SPOOL_AUTH, info.get(), env.err_ptr())?;
        let mut pool_name_ptr = ptr::null::<u8>();
        let mut pool_name_len = 0u32;
        oci::session_pool_create(
            env.env_ptr(), env.err_ptr(), pool.get(),
            &mut pool_name_ptr, &mut pool_name_len,
            dbname.as_ptr(), dbname.len() as u32,
            min as u32, max as u32, inc as u32,
            username.as_ptr(), username.len() as u32,
            password.as_ptr(), password.len() as u32,
            OCI_SPC_HOMOGENEOUS | OCI_SPC_STMTCACHE
        )?;
        let name = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(pool_name_ptr, pool_name_len as usize)) };
        Ok(Self {env, pool, name})
    }

    pub fn get_session(&self) -> Result<Connection> {
        Connection::from_session_pool(&self.env, &self.name)
    }
}

impl Drop for SessionPool<'_> {
    fn drop(&mut self) {
        unsafe {
            OCISessionPoolDestroy(self.pool.get(), self.env.err_ptr(), OCI_SPD_FORCE);
        }
    }
}
