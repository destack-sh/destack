mod backend;
mod core;
#[cfg(test)]
mod tests;
#[cfg(unix)]
mod unix;
#[cfg(not(any(unix, windows)))]
mod unsupported;
#[cfg(windows)]
mod windows;

pub(crate) use core::{read_mount_entries, read_mount_entries_vm};
