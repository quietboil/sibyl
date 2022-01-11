//! Data Type Mapping and Manipulation Functions

pub(crate) mod date;
pub(crate) mod raw;
pub(crate) mod number;
pub(crate) mod varchar;
pub(crate) mod timestamp;
pub(crate) mod interval;
pub(crate) mod rowid;

pub use date::Date;
pub use raw::Raw;
pub use number::Number;
pub use varchar::Varchar;
pub use rowid::RowID;
pub use timestamp::DateTime;
pub use interval::Interval;

use libc::c_void;
use crate::oci::{OCIError, OCIEnv, OCISession};

pub trait Ctx : AsRef<OCIEnv> + AsRef<OCIError> + Send + Sync {
    fn as_context(&self) -> *const c_void {
        if let Some(session) = self.try_as_session() {
            session as *const OCISession as _
        } else {            
            self.as_ref() as &OCIEnv as *const OCIEnv as _
        }
    }
    fn try_as_session(&self) -> Option<&OCISession>;
}
