#[cfg(any(unix, windows))]
mod basic;
#[cfg(unix)]
mod exec;
#[cfg(unix)]
mod exit;
#[cfg(unix)]
mod identity;
#[cfg(unix)]
mod isolation;
#[cfg(unix)]
mod limits;
#[cfg(unix)]
mod privileged;
#[cfg(unix)]
mod session;
#[cfg(unix)]
mod signal;
#[cfg(any(unix, windows))]
mod spawn;
#[cfg(any(unix, windows))]
mod tests;
#[cfg(any(unix, windows))]
mod wait;

#[cfg(any(unix, windows))]
pub(super) use tests::*;
