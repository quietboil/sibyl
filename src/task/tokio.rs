//! Abstraction over tokio task functions

use std::future::Future;

pub use tokio_rt::task::{spawn, spawn_blocking, yield_now, JoinError};

/// Builds a new multi-thread Tokio runtime and runs a future to completion on it.
/// 
/// This function is inteded to run Sibyl's tests and examples.
///
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
pub fn block_on<F: Future>(future: F) -> F::Output {
    tokio_rt::runtime::Builder::new_multi_thread().enable_all().build().unwrap().block_on(future)
}
