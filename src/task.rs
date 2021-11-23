//! Abstraction over async task functions

mod tokio;

pub(crate) use self::tokio::{spawn, spawn_blocking, JoinError};
