//! Session pool blocking mode implementation

use super::ConnectionPool;
use crate::{Connection, Environment, Result, env::Env, oci::{self, *}};
use std::ptr;

impl<'a> ConnectionPool<'a> {
    pub(crate) fn new(env: &'a Environment, dbname: &str, username: &str, password: &str, min: usize, inc: usize, max: usize) -> Result<Self> {
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
        Ok(Self {env, pool, name, user: username.to_string(), pass: password.to_string()})
    }

    pub fn get_session(&self) -> Result<Connection> {
        Connection::from_connection_pool(&self.env, &self.name, &self.user, &self.pass)
    }
}

impl Drop for ConnectionPool<'_> {
    fn drop(&mut self) {
        unsafe {
            OCIConnectionPoolDestroy(self.pool.get(), self.env.err_ptr(), OCI_DEFAULT);
        }
    }
}
