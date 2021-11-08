//! Abstraction over tokio or async-std task functions

mod tokio;

pub(crate) use self::tokio::spawn_blocking;
