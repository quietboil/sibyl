//! Abstraction over async task functions

#[cfg(not(any(feature="tokio",feature="actix")))]
compile_error!("'nonblocking' requires an async runtime. Select either 'tokio' or 'actix'");

#[cfg(all(feature="tokio",feature="actix"))]
compile_error!("only one async runtime must be selected. Select either 'tokio' or 'actix'");

#[cfg(feature="tokio")]
mod tokio;

#[cfg(feature="tokio")]
pub use self::tokio::{spawn, spawn_blocking, JoinError, block_on};

#[cfg(feature="actix")]
mod actix;

#[cfg(feature="actix")]
pub use self::actix::{spawn, spawn_blocking, JoinError, block_on};
