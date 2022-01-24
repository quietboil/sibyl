//! Abstraction over tokio task functions

use std::future::Future;

pub use tokio_rt::task::spawn;

use tokio_rt::task::spawn_blocking;
use crate::{Result, Error};

pub(crate) async fn execute_blocking<F, R>(f: F) -> Result<R> 
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    match spawn_blocking(f).await {
        Ok(res) => Ok(res),
        Err(err) => Err(Error::msg(format!("blocking task {}", err))),
    }
}

pub fn spawn_detached<F>(f: F)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let _ = spawn(f);
}

/// Builds a new multi-thread Tokio runtime and runs a future to completion on it.
/// 
/// This function is included to run Sibyl's tests and examples.
///
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
pub fn block_on<F: Future>(future: F) -> F::Output {
    tokio_rt::runtime::Builder::new_multi_thread().enable_all().build().unwrap().block_on(future)
}
