//! Abstraction over actix task functions

use std::{future::Future, sync::atomic::Ordering};

pub use actix_rt::spawn;

use actix_rt::{task, Runtime};
use crate::{Result, Error, oci::futures::NUM_ACTIVE_ASYNC_DROPS};

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

pub fn spawn_detached<F>(f: F)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let _ = spawn(f);
}

/// Builds a new Actix runtime and runs a future to completion on it.
/// 
/// This function ensures that all async drops have run to completion.
///
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
pub fn block_on<F: Future>(future: F) -> F::Output {
    Runtime::new().unwrap().block_on(async move {
        let res = future.await;
        while NUM_ACTIVE_ASYNC_DROPS.load(Ordering::Acquire) > 0 {
            task::yield_now().await;
        }
        res
    })
}
