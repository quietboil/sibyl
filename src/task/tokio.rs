//! Abstraction over tokio task functions

use std::future::Future;

pub use tokio_rt::task::spawn;

use tokio_rt::{task, runtime};
use crate::{Result, Error};

pub(crate) async fn execute_blocking<F, R>(f: F) -> Result<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    match task::spawn_blocking(f).await {
        Ok(res) => Ok(res),
        Err(err) => Err(Error::msg(format!("blocking task {}", err))),
    }
}

/// Builds a new multi-thread Tokio runtime and runs a future to completion on it.
/// 
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
pub fn block_on<F: Future>(future: F) -> F::Output {
    runtime::Builder::new_multi_thread().enable_all().build().unwrap().block_on(async move {
        future.await
    })
}
