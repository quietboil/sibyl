//! Abstraction over tokio task functions

use std::{future::Future, sync::{Weak, Arc}};

use parking_lot::Mutex;
pub use tokio_rt::task::spawn;

use tokio_rt::{task, runtime};
use crate::{Result, Error, oci::{Handle, OCIEnv}};

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

static OCI_ENVIRONMENTS : Mutex<Vec<Weak<Handle<OCIEnv>>>> = Mutex::new(Vec::new());

pub(crate) fn register_env(env: &Arc<Handle<OCIEnv>>) {
    OCI_ENVIRONMENTS.lock().push(Arc::downgrade(env));
}

fn any_oci_env_is_in_use() -> bool {
    OCI_ENVIRONMENTS.lock().iter().any(|rc| rc.strong_count() > 1)
}

/// Builds a new multi-thread Tokio runtime and runs a future to completion on it.
/// 
/// This function ensures that all async drops have run to completion.
///
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
pub fn block_on<F: Future>(future: F) -> F::Output {
    runtime::Builder::new_multi_thread().enable_all().build().unwrap().block_on(async move {
        let res = future.await;
        while any_oci_env_is_in_use() {
            task::yield_now().await;
        }
        res
    })
}
