//! Abstraction over async-std task functions

use std::future::Future;

pub use async_rt::task::spawn;

use async_rt::task::spawn_blocking;
use crate::Result;

pub(crate) async fn execute_blocking<F, R>(f: F) -> Result<R> 
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let res = spawn_blocking(f).await;
    Ok(res)
}

pub fn spawn_detached<F>(f: F)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let _ = spawn(f);
}


/// Runs a future on async-std executor.
/// 
/// This function is included to run Sibyl's tests and examples.
///
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
pub fn block_on<F: Future>(future: F) -> F::Output {
    async_rt::task::block_on(future)
}
