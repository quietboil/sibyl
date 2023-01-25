//! Abstraction over async-std task functions

use std::{future::Future, sync::atomic::Ordering};

pub use async_rt::task::spawn;

use async_rt::task;
use crate::{Result, oci::futures::NUM_ACTIVE_ASYNC_DROPS};

pub(crate) async fn execute_blocking<F, R>(f: F) -> Result<R> 
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let res = task::spawn_blocking(f).await;
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
/// This function ensures that all async drops have run to completion.
///
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
pub fn block_on<F: Future>(future: F) -> F::Output {
    task::block_on(async move {
        let res = future.await;
        while NUM_ACTIVE_ASYNC_DROPS.load(Ordering::Acquire) > 0 {
            task::yield_now().await;
        }
        res
    })
}
