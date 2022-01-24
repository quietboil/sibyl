//! Abstraction over async-global-executor task functions

use std::future::Future;

pub use async_global_executor::spawn;

use async_global_executor::spawn_blocking;
use crate::Result;

pub(crate) async fn execute_blocking<F, R>(f: F) -> Result<R> 
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    Ok(spawn_blocking(f).await)
}

pub fn spawn_detached<F>(f: F)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    spawn(f).detach()
}

/// Runs a future on async-global-executor.
/// 
/// This function is included to run Sibyl's tests and examples.
///
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
pub fn block_on<F: Future>(future: F) -> F::Output {
    async_global_executor::block_on(future)
}
