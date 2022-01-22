//! Abstraction over async task functions

#[cfg(not(any(feature="tokio",feature="actix",feature="async-std")))]
compile_error!("'nonblocking' requires an async runtime. Select 'tokio', 'actix', or 'async-std'");

#[cfg(
    any(
        all( feature="tokio", any(feature="actix", feature="async-std") ),
        all( feature="actix", any(feature="tokio", feature="async-std") ),
        all( feature="async-std", any(feature="tokio", feature="actix") )
    )
)]
compile_error!("only one async runtime must be selected. Select 'tokio', 'actix', or 'async-std'");

#[cfg(feature="tokio")]
mod tokio;

#[cfg(feature="tokio")]
pub use self::tokio::{spawn, block_on};

#[cfg(feature="tokio")]
pub(crate) use self::tokio::execute_blocking;

#[cfg(feature="actix")]
mod actix;

#[cfg(feature="actix")]
pub use self::actix::{spawn, block_on};

#[cfg(feature="actix")]
pub(crate) use self::actix::execute_blocking;

#[cfg(feature="async-std")]
mod async_std;

#[cfg(feature="async-std")]
pub use self::async_std::{spawn, block_on};

#[cfg(feature="async-std")]
pub(crate) use self::async_std::execute_blocking;
