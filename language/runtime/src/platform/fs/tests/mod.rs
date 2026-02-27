#[cfg(any(unix, windows))]
mod advanced;
#[cfg(any(unix, windows))]
mod at;
#[cfg(any(unix, windows))]
mod attrs;
#[cfg(any(unix, windows))]
mod basic;
#[cfg(any(unix, windows))]
mod dir;
#[cfg(any(unix, windows))]
mod edge;
#[cfg(any(unix, windows))]
mod file;
#[cfg(any(unix, windows))]
mod mmap;
#[cfg(any(unix, windows))]
mod path;
#[cfg(unix)]
mod privileged;
#[cfg(any(unix, windows))]
mod stat;
#[cfg(any(unix, windows))]
mod tests;
#[cfg(any(unix, windows))]
mod utf16;
#[cfg(any(unix, windows))]
mod watch;
#[cfg(any(unix, windows))]
mod xattr;

#[cfg(any(unix, windows))]
pub(super) use tests::*;
