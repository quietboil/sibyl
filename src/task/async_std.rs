//! Abstraction over async-std task functions

use std::future::Future;

pub use async_rt::task::spawn;

use async_rt::task;
use crate::Result;

pub(crate) async fn execute_blocking<F, R>(f: F) -> Result<R> 
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let res = task::spawn_blocking(f).await;
    Ok(res)
}

/// Runs a future on async-std executor.
/// 
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
pub fn block_on<F: Future>(future: F) -> F::Output {
    task::block_on(async move {
        future.await
    })
}
