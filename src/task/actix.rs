//! Abstraction over actix task functions

use std::future::Future;

pub use actix_rt::spawn;
pub use actix_rt::task::{spawn_blocking, yield_now, JoinError};

/// Builds a new actix runtime and runs a future to completion on it.
/// 
/// This function is inteded to run Sibyl's tests and examples.
///
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
pub fn block_on<F: Future>(future: F) -> F::Output {
    actix_rt::Runtime::new().unwrap().block_on(future)
}
