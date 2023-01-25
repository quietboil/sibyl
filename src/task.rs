//! Abstraction over async task functions

#[cfg(not(any(feature="tokio", feature="actix", feature="async-std", feature="async-global")))]
compile_error!("'nonblocking' requires an async runtime. Select 'tokio', 'actix', 'async-std', or 'async-global'");

#[cfg(
    any(
        all( feature="tokio", any(feature="actix", feature="async-std", feature="async-global") ),
        all( feature="actix", any(feature="tokio", feature="async-std", feature="async-global") ),
        all( feature="async-std", any(feature="tokio", feature="actix", feature="async-global") ),
        all( feature="async-global", any(feature="tokio", feature="actix", feature="async-std") ),
    )
)]
compile_error!("only one async runtime must be selected. Select 'tokio', 'actix', 'async-std', or 'async-global'");

#[cfg(feature="tokio")]
mod tokio;

#[cfg(feature="tokio")]
pub use self::tokio::{spawn, block_on};

#[cfg(feature="tokio")]
pub(crate) use self::tokio::{execute_blocking, spawn_detached};

#[cfg(feature="actix")]
mod actix;

#[cfg(feature="actix")]
pub use self::actix::{spawn, block_on};

#[cfg(feature="actix")]
pub(crate) use self::actix::{execute_blocking, spawn_detached};

#[cfg(feature="async-std")]
mod async_std;

#[cfg(feature="async-std")]
pub use self::async_std::{spawn, block_on};

#[cfg(feature="async-std")]
pub(crate) use self::async_std::{execute_blocking, spawn_detached};

#[cfg(feature="async-global")]
mod async_global;

#[cfg(feature="async-global")]
pub use self::async_global::{spawn, block_on};

#[cfg(feature="async-global")]
pub(crate) use self::async_global::{execute_blocking, spawn_detached};
