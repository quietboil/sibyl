//! Nonblocking mode Connection methods.
 
use super::{connect, Connection};
use crate::{Environment, Result, env::Env, oci::{*, futures::*}, task};
use libc::c_void;

impl Drop for Connection<'_> {
    fn drop(&mut self) {
        let usr = Handle::from_handle(&mut self.usr);
        let svc = Handle::from_handle(&mut self.svc);
        let srv = Handle::from_handle(&mut self.srv);
        let err = Handle::from_handle(&mut self.err);
        let session_ptr = svc.get_attr::<*mut c_void>(OCI_ATTR_SESSION, err.get()).unwrap_or(std::ptr::null_mut());
        let server_ptr = svc.get_attr::<*mut c_void>(OCI_ATTR_SERVER, err.get()).unwrap_or(std::ptr::null_mut());

        let async_drop = async move {
            if !session_ptr.is_null() {
                SessionEnd::new(svc.get(), err.get(), usr.get()).await;
            }
            if !server_ptr.is_null() {
                ServerDetach::new(srv.get(), err.get()).await;
            }
        };
        task::spawn(async_drop);
    }
}

impl<'a> Connection<'a> {
    pub(crate) async fn new(env: &'static Environment, addr: &'static str, user: &'static str, pass: &'static str) -> Result<Connection<'a>> {
        task::spawn_blocking(move || connect(env, addr, user, pass)).await?
    }

}
