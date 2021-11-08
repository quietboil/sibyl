//! Nonblocking mode Connection methods.
 
use super::{connect, Connection};
use crate::{Environment, Error, Result, Statement, catch, env::Env, oci::*, task};
use libc::c_void;

impl Drop for Connection<'_> {
    fn drop(&mut self) {
        if let Ok(ptr) = self.svc.get_attr::<*mut c_void>(OCI_ATTR_SESSION, self.err_ptr()) {
            if !ptr.is_null() {
                unsafe {
                    OCISessionEnd(self.svc_ptr(), self.err_ptr(), self.usr_ptr(), OCI_DEFAULT);
                }
            }
        }
        if let Ok(ptr) = self.svc.get_attr::<*mut c_void>(OCI_ATTR_SERVER, self.err_ptr()) {
            if !ptr.is_null() {
                unsafe {
                    OCIServerDetach(self.srv_ptr(), self.err_ptr(), OCI_DEFAULT);
                }
            }
        }
    }
}

impl<'a> Connection<'a> {
    pub(crate) fn new(env: &'a Environment, addr: &str, user: &str, pass: &str) -> Result<Self> {
        let _x = task::spawn_blocking(move || 
            connect(env, addr, user,pass)
        );
        Err(Error::new("not implemented"))
    }

}
