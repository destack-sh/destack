#[cfg(any(unix, windows))]
mod basic;
#[cfg(any(unix, windows))]
mod edge;
#[cfg(any(unix, windows))]
mod message;
#[cfg(any(unix, windows))]
mod options;
#[cfg(any(unix, windows))]
mod resolve;
#[cfg(any(unix, windows))]
mod shutdown;
#[cfg(any(unix, windows))]
mod tests;
#[cfg(any(unix, windows))]
mod udp;
#[cfg(any(unix, windows))]
mod uds;

#[cfg(any(unix, windows))]
pub(super) use tests::*;
