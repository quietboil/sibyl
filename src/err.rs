//! Errors returned by Sibyl

use crate::oci::*;
use std::{ptr, cmp, fmt, error, io, ffi::CStr};
use libc::c_void;

fn get_oracle_error(rc: i32, errhp: *mut c_void, htype: u32) -> (i32, String) {
    let mut errcode = rc;
    let mut errmsg : Vec<u8> = Vec::with_capacity(OCI_ERROR_MAXMSG_SIZE);
    let errmsg_ptr = errmsg.as_mut_ptr();
    let res = unsafe {
        *errmsg_ptr = 0;
        OCIErrorGet(errhp, 1, ptr::null(), &mut errcode, errmsg_ptr, OCI_ERROR_MAXMSG_SIZE as u32, htype)
    };
    let msg = if res == OCI_SUCCESS {
        let msg = unsafe { CStr::from_ptr(errmsg_ptr as *const i8) };
        msg.to_string_lossy().trim_end().to_string()
    } else {
        match errcode {
            OCI_NO_DATA   => String::from("No Data"),
            OCI_NEED_DATA => String::from("Need Data"),
            _ => format!("Error {}", errcode),
        }
    };
    (errcode, msg)
}

/// Represents possible errors returned from Sibyl
#[derive(Debug)]
pub enum Error {
    Interface(String),
    Oracle(i32,String),
    #[cfg(feature="nonblocking")]
    JoinError(JoinError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Oracle(errcode, errmsg) => write!(f, "ORA-{:05}: {}", errcode, errmsg),
            Error::Interface(errmsg) => write!(f, "{}", errmsg),
            #[cfg(feature="nonblocking")]
            Error::JoinError(src) => src.fmt(f)
        }
    }
}

impl error::Error for Error {}

impl cmp::PartialEq for Error {
    fn eq(&self, other: &Error) -> bool {
        match (self, other) {
            (Error::Oracle(this_code, _), Error::Oracle(other_code, _)) => this_code == other_code,
            (Error::Interface(this_msg),  Error::Interface(other_msg))  => this_msg  == other_msg,
            _ => false,
        }
    }
}

impl From<Error> for io::Error {
    fn from(err: Error) -> Self {
        io::Error::new(io::ErrorKind::Other, err)
    }
}

impl Error {
    pub(crate) fn new(msg: &str) -> Self {
        Error::Interface( msg.to_owned() )
    }

    pub(crate) fn env(env: *mut OCIEnv, rc: i32) -> Self {
        let (code, msg) = get_oracle_error(rc, env as *mut c_void, OCI_HTYPE_ENV);
        Error::Oracle(code, msg)
    }

    pub(crate) fn oci(err: *mut OCIError, rc: i32) -> Self {
        let (code, msg) = get_oracle_error(rc, err as *mut c_void, OCI_HTYPE_ERROR);
        Error::Oracle(code, msg)
    }
}

#[cfg(feature="nonblocking")]
use crate::task::JoinError;

#[cfg(feature="nonblocking")]
impl From<JoinError> for Error {
    fn from(err: JoinError) -> Self {
        Error::JoinError(err)
    }
}