//! Data Type Mapping and Manipulation Functions

pub(crate) mod raw;
pub(crate) mod date;
pub(crate) mod number;
pub(crate) mod varchar;
pub(crate) mod timestamp;
pub(crate) mod interval;
pub(crate) mod rowid;

pub use date::Date;
pub use number::Number;
pub use raw::Raw;
pub use varchar::Varchar;

use crate::env::Env;
use libc::c_void;

/**
    Both OCIDateTime and OCIInterval can be invoked in an OCI environment or
    a user session context. This trait specifies protocol that Timestamp and
    Interval use to function in either context.
*/
pub trait Ctx: Env {
    /// Returns pointer to the current context - either environment or session.
    fn ctx_ptr(&self) -> *mut c_void;
}
