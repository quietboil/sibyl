//! Abstraction over tokio task functions

use tokio::task;

pub(crate) fn spawn_blocking<F,R>(f: F) -> task::JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static
{
    task::spawn_blocking(f)
}

