//! Abstraction over async task functions

mod tokio;

pub use self::tokio::{spawn, spawn_blocking, JoinError};
