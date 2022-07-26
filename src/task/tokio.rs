//! Abstraction over tokio task functions

use std::future::Future;

use parking_lot::Mutex;
pub use tokio_rt::task::spawn;

use tokio_rt::task::{self, JoinHandle};
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

/// List of currently active async drop tasks.
struct AsyncDrops(Vec<JoinHandle<()>>);

impl AsyncDrops {
    const fn new() -> Self {
        Self(Vec::new())
    }

    fn push<F>(&mut self, async_drop: F) where F: Future<Output = ()> + Send + 'static {
        self.0.retain(|task| !task.is_finished());
        self.0.push(spawn(async_drop))
    }

    fn is_empty(&mut self) -> bool {
        self.0.retain(|task| !task.is_finished());
        self.0.is_empty()
    }
}

static ASYNC_DROPS : Mutex<AsyncDrops> = Mutex::new(AsyncDrops::new());

pub(crate) fn spawn_detached<F>(f: F) where F: Future<Output = ()> + Send + 'static
{
    ASYNC_DROPS.lock().push(f);
}

/// Builds a new multi-thread Tokio runtime and runs a future to completion on it.
///
/// This function is included to run Sibyl's tests and examples.
///
#[cfg_attr(docsrs, doc(cfg(feature="nonblocking")))]
pub fn block_on<F: Future>(future: F) -> F::Output {
    tokio_rt::runtime::Builder::new_multi_thread().enable_all().build().unwrap().block_on(async move {
        let res = future.await;
        while !ASYNC_DROPS.lock().is_empty() {
            task::yield_now().await;
        }
        res
    })
}
