//! Data Type Mapping and Manipulation Functions

pub(crate) mod raw;
pub(crate) mod date;
pub(crate) mod number;
pub(crate) mod varchar;
pub(crate) mod timestamp;
pub(crate) mod interval;

use crate::*;
use crate::env::Env;
use crate::conn::Conn;
use libc::c_void;

/**
    Both OCIDateTime and OCIInterval can be invoked in an OCI environment or
    a user session context. This trait specifies protocol that Timestamp and
    Interval use to function in either context.
*/
pub trait UsrEnv : Env {
    /// Returns pointer to the current context - either environment or session.
    fn as_ptr(&self) -> *mut c_void;
    /// Returns a `Conn` trait object if possible
    fn as_conn(&self) -> Option<&dyn Conn>;
}

impl UsrEnv for Environment {
    fn as_ptr(&self) -> *mut c_void {
        self.env_ptr() as *mut c_void
    }
    fn as_conn(&self) -> Option<&dyn Conn> {
        None
    }
}

impl UsrEnv for Connection<'_> {
    fn as_ptr(&self) -> *mut c_void {
        self.usr_ptr() as *mut c_void
    }
    fn as_conn(&self) -> Option<&dyn Conn> {
        Some( self )
    }
}
