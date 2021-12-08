//! Session pool blocking mode implementation

use super::SessionPool;
use crate::{Result, env::Env, oci::{self, *}, Environment, Connection};
use std::{ptr, marker::PhantomData};

impl<'a> SessionPool<'a> {
    pub(crate) fn new(env: &'a Environment, dbname: &str, username: &str, password: &str, min: usize, inc: usize, max: usize) -> Result<Self> {
        let err = Handle::<OCIError>::new(env.env_ptr())?;

        let info = Handle::<OCIAuthInfo>::new(env.env_ptr())?;
        info.set_attr(OCI_ATTR_DRIVER_NAME, "sibyl", err.get())?;

        let pool = Handle::<OCISPool>::new(env.env_ptr())?;
        pool.set_attr(OCI_ATTR_SPOOL_AUTH, info.get(), err.get())?;

        let mut pool_name_ptr = ptr::null::<u8>();
        let mut pool_name_len = 0u32;
        oci::session_pool_create(
            env.env_ptr(), err.get(), pool.get(),
            &mut pool_name_ptr, &mut pool_name_len,
            dbname.as_ptr(), dbname.len() as u32,
            min as u32, max as u32, inc as u32,
            username.as_ptr(), username.len() as u32,
            password.as_ptr(), password.len() as u32,
            OCI_SPC_HOMOGENEOUS | OCI_SPC_STMTCACHE
        )?;
        let name = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(pool_name_ptr, pool_name_len as usize)) };
        Ok(Self {env: env.clone_env(), err, pool, name, phantom_env: PhantomData})
    }

    pub(crate) fn get_svc_ctx(&self) -> Result<Ptr<OCISvcCtx>> {
        let inf = Handle::<OCIAuthInfo>::new(self.env.get())?;
        let mut svc = Ptr::null();
        let mut found = 0u8;
        oci::session_get(
            self.env.get(), self.err.get(), svc.as_mut_ptr(), inf.get(), self.name.as_ptr(), self.name.len() as u32,
            ptr::null(), 0, ptr::null_mut(), ptr::null_mut(), &mut found,
            OCI_SESSGET_SPOOL | OCI_SESSGET_SPOOL_MATCHANY | OCI_SESSGET_PURITY_SELF
        )?;
        Ok(svc)
    }

    pub fn get_session(&self) -> Result<Connection> {
        Connection::from_session_pool(self)
    }
}

impl Drop for SessionPool<'_> {
    fn drop(&mut self) {
        unsafe {
            OCISessionPoolDestroy(self.pool.get(), self.err.get(), OCI_SPD_FORCE);
        }
    }
}
