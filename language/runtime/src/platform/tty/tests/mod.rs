#[cfg(any(unix, windows))]
mod basic;
#[cfg(any(unix, windows))]
mod handle;
#[cfg(any(unix, windows))]
mod io;
#[cfg(any(unix, windows))]
mod mode;
#[cfg(any(unix, windows))]
mod pty;
#[cfg(any(unix, windows))]
mod size;
#[cfg(unix)]
mod termios;
#[cfg(any(unix, windows))]
mod tests;

#[cfg(any(unix, windows))]
pub(super) use tests::*;
