//! Abstraction over async-std task functions

use std::{future::Future, sync::{Weak, Arc}};

pub use async_rt::task::spawn;

use async_rt::task;
use parking_lot::Mutex;
use crate::{Result, oci::{Handle, OCIEnv}};

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

static OCI_ENVIRONMENTS : Mutex<Vec<Weak<Handle<OCIEnv>>>> = Mutex::new(Vec::new());

pub(crate) fn register_env(env: &Arc<Handle<OCIEnv>>) {
    OCI_ENVIRONMENTS.lock().push(Arc::downgrade(env));
}

fn any_oci_env_is_in_use() -> bool {
    OCI_ENVIRONMENTS.lock().iter().any(|rc| rc.strong_count() > 1)
}

/// Runs a future on async-std executor.
/// 
/// This function ensures that all async drops have run to completion.
///
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
pub fn block_on<F: Future>(future: F) -> F::Output {
    task::block_on(async move {
        let res = future.await;
        while any_oci_env_is_in_use() {
            task::yield_now().await;
        }
        res
    })
}
