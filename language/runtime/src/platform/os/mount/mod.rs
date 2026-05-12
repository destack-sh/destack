mod core;
mod target;
#[cfg(all(unix, not(target_os = "android")))]
mod unix;
#[cfg(not(any(all(unix, not(target_os = "android")), windows)))]
mod unsupported;
#[cfg(windows)]
mod windows;

pub(crate) use core::{read_mount_entries, read_mount_entries_vm};
