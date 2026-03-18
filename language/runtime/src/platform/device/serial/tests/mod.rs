#[cfg(any(unix, windows))]
mod core;
#[cfg(unix)]
mod errors;
#[cfg(unix)]
mod events;
#[cfg(unix)]
mod io;
#[cfg(any(unix, windows))]
mod list;
#[cfg(any(unix, windows))]
mod watch;
