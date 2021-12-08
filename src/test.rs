//! Unittest helpers

use std::future::Future;

pub fn on_single_thread<F: Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(future)
}

pub fn on_multi_threads<F: Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap().block_on(future)
}
