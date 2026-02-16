#[path = "statfs.rs"]
mod statfs;

use super::core::{nanos_from_secs_and_nanos, statfs_u64};

#[cfg(any(target_os = "linux", target_os = "android"))]
#[path = "linux.rs"]
mod platform;

#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod platform;

#[cfg(target_os = "ios")]
#[path = "ios.rs"]
mod platform;

#[cfg(target_os = "freebsd")]
#[path = "freebsd.rs"]
mod platform;

#[cfg(target_os = "openbsd")]
#[path = "openbsd.rs"]
mod platform;

#[cfg(target_os = "netbsd")]
#[path = "netbsd.rs"]
mod platform;

#[cfg(target_os = "dragonfly")]
#[path = "dragonfly.rs"]
mod platform;

#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
pub(super) use platform::*;
