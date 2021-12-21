//! Abstraction over tokio task functions

use std::future::Future;

pub use tokio_rt::task::{spawn, spawn_blocking, JoinError};

/// Runs a future to completion on the current thread Tokio runtime.
pub fn current_thread_block_on<F: Future>(future: F) -> F::Output {
    tokio_rt::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(future)
}

/// Runs a future to completion on the multi-thread Tokio runtime.
pub fn multi_thread_block_on<F: Future>(future: F) -> F::Output {
    tokio_rt::runtime::Builder::new_multi_thread().enable_all().build().unwrap().block_on(future)
}
