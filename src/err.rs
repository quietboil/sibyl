
use crate::*;
use libc::c_void;
use std::{ ptr, cmp, fmt, error, io, ffi::CStr };

const OCI_ERROR_MAXMSG_SIZE : usize = 3072;
const OCI_HTYPE_ENV         : u32 = 1;
const OCI_HTYPE_ERROR       : u32 = 2;

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/miscellaneous-functions.html#GUID-4B99087C-74F6-498A-8310-D6645172390A
    fn OCIErrorGet(
        hndlp:      *const c_void,
        recordno:   u32,
        sqlstate:   *const c_void,
        errcodep:   *mut i32,
        bufp:       *mut u8,
        bufsiz:     u32,
        hnd_type:   u32,
    ) -> i32;
}

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

macro_rules! catch {
    ( $err:expr => $( $stmt:stmt );+ ) => {{
        let res = unsafe { $($stmt)+ };
        match res {
            OCI_ERROR | OCI_INVALID_HANDLE => { return Err( crate::Error::oci($err, res) ); },
            _ => {}
        }
    }};
}

/// Represents possible errors returned from Sibyl
#[derive(Debug)]
pub enum Error {
    Interface(String),
    Oracle(i32,String)
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Oracle(errcode, errmsg) => write!(f, "ORA-{:05}: {}", errcode, errmsg),
            Error::Interface(errmsg) => write!(f, "{}", errmsg),
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
