//! Session and Connection Pools

pub(crate) mod session;

pub use session::{SessionPool, SessionPoolGetMode};

#[cfg(feature="blocking")]
#[cfg_attr(docsrs, doc(cfg(feature="blocking")))]
mod connection;

#[cfg(feature="blocking")]
#[cfg_attr(docsrs, doc(cfg(feature="blocking")))]
pub use connection::ConnectionPool;
