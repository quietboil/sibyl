//! Connection pool nonblocking mode implementation

use super::ConnectionPool;
use crate::{Connection, Result, env::Env, oci::{self, *}, Environment, ptr::ScopedPtr, task};
use std::{ptr, slice, str, marker::PhantomData};

impl<'a> ConnectionPool<'a> {
    pub(crate) async fn new(env: &'a Environment, dbname: &str, username: &str, password: &str, min: usize, inc: usize, max: usize) -> Result<ConnectionPool<'a>> {
        let err = Handle::<OCIError>::new(env.env_ptr())?;
        let pool = Handle::<OCICPool>::new(env.env_ptr())?;

        let env_ptr = env.get_env_ptr();
        let err_ptr = env.get_err_ptr();
        let pool_ptr = pool.get_ptr();
        let dbname_ptr = ScopedPtr::new(dbname.as_ptr());
        let dbname_len = dbname.len() as u32;
        let username_ptr = ScopedPtr::new(username.as_ptr());
        let username_len = username.len() as u32;
        let password_ptr = ScopedPtr::new(password.as_ptr());
        let password_len = password.len() as u32;

        let name = task::spawn_blocking(move || -> Result<&str> {
            let mut pool_name_ptr = ptr::null::<u8>();
            let mut pool_name_len = 0u32;
            oci::connection_pool_create(
                env_ptr.get(), err_ptr.get(), pool_ptr.get(),
                &mut pool_name_ptr, &mut pool_name_len,
                dbname_ptr.get(), dbname_len,
                min as u32, max as u32, inc as u32,
                username_ptr.get(), username_len,
                password_ptr.get(), password_len,
                OCI_DEFAULT
            )?;
            let name = unsafe {
                // `name` is just a container that we'll be passing back to OCI without interpreting it
                str::from_utf8_unchecked(
                    slice::from_raw_parts(pool_name_ptr, pool_name_len as usize)
                )
            };
            Ok(name)
        }).await??;
        Ok(Self {env: env.clone_env(), err, pool, name, phantom_env: PhantomData})
    }

    pub(crate) async fn get_svc_ctx(&self, username: &str, password: &str) -> Result<Ptr<OCISvcCtx>> {
        let inf = Handle::<OCIAuthInfo>::new(self.env.get())?;
        inf.set_attr(OCI_ATTR_DRIVER_NAME, "sibyl", self.err.get())?;
        inf.set_attr(OCI_ATTR_USERNAME, username, self.err.get())?;
        inf.set_attr(OCI_ATTR_PASSWORD, password, self.err.get())?;

        let env_ptr = self.env.get_ptr();
        let err_ptr = self.err.get_ptr();
        let pool_name_ptr = ScopedPtr::new(self.name.as_ptr());
        let pool_name_len = self.name.len() as u32;

        task::spawn_blocking(move || -> Result<Ptr<OCISvcCtx>> {
            let mut svc = Ptr::null();
            let mut found = 0u8;
                oci::session_get(
                env_ptr.get(), err_ptr.get(), svc.as_mut_ptr(), inf.get(), pool_name_ptr.get(), pool_name_len,
                ptr::null(), 0, ptr::null_mut(), ptr::null_mut(), &mut found,
                OCI_SESSGET_CPOOL | OCI_SESSGET_STMTCACHE
            )?;
            Ok(svc)
        }).await?
    }


    pub async fn get_session(&self, user: &str, pass: &str) -> Result<Connection<'_>> {
        Connection::from_connection_pool(self, user, pass).await
    }
}

impl Drop for ConnectionPool<'_> {
    fn drop(&mut self) {
        let pool = Handle::take_over(&mut self.pool);
        let err = Handle::take_over(&mut self.err);
        let env = self.env.clone();
        task::spawn(oci::futures::ConnectionPoolDestroy::new(pool, err, env));
    }
}
