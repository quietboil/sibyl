//! Session pool blocking mode implementation

use super::SessionPool;
use crate::{Result, oci::{self, *}, Environment, Connection};
use std::{ptr, marker::PhantomData};

impl<'a> SessionPool<'a> {
    pub(crate) fn new(env: &'a Environment, dbname: &str, username: &str, password: &str, min: usize, inc: usize, max: usize) -> Result<Self> {
        let err = Handle::<OCIError>::new(env)?;
        let info = Handle::<OCIAuthInfo>::new(env)?;
        info.set_attr(OCI_ATTR_DRIVER_NAME, "sibyl", &err)?;

        let pool = Handle::<OCISPool>::new(env)?;
        pool.set_attr(OCI_ATTR_SPOOL_AUTH, info.get_ptr(), &err)?;

        let mut pool_name_ptr = ptr::null::<u8>();
        let mut pool_name_len = 0u32;
        oci::session_pool_create(
            env.as_ref(), &err, &pool,
            &mut pool_name_ptr, &mut pool_name_len,
            dbname.as_ptr(), dbname.len() as u32,
            min as u32, max as u32, inc as u32,
            username.as_ptr(), username.len() as u32,
            password.as_ptr(), password.len() as u32,
            OCI_SPC_HOMOGENEOUS | OCI_SPC_STMTCACHE
        )?;
        let name = unsafe { std::slice::from_raw_parts(pool_name_ptr, pool_name_len as usize) };
        Ok(Self {env: env.get_env(), err, pool, name, phantom_env: PhantomData})
    }

    pub(crate) fn get_svc_ctx(&self) -> Result<Ptr<OCISvcCtx>> {
        let inf = Handle::<OCIAuthInfo>::new(self.env.as_ref())?;
        let mut svc = Ptr::<OCISvcCtx>::null();
        let mut found = 0u8;
        oci::session_get(
            self.env.as_ref(), &self.err, svc.as_mut_ptr(), &inf,
            self.name.as_ptr(), self.name.len() as u32, &mut found,
            OCI_SESSGET_SPOOL | OCI_SESSGET_PURITY_SELF
        )?;
        Ok(svc)
    }

    /**
        Returns a new session with a new underlyng connection from this pool.
    */
    pub fn get_session(&self) -> Result<Connection> {
        Connection::from_session_pool(self)
    }
}

impl Drop for SessionPool<'_> {
    fn drop(&mut self) {
        oci_session_pool_destroy(&self.pool, &self.err);
    }
}
