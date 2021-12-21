//! Abstraction over async task functions

mod tokio;

pub use self::tokio::{spawn, spawn_blocking, JoinError, current_thread_block_on, multi_thread_block_on};
