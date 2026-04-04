#[cfg(any(test, target_os = "android"))]
#[path = "android/mod.rs"]
pub mod android;

#[cfg(target_vendor = "apple")]
#[path = "apple/mod.rs"]
pub mod apple;

#[cfg(target_os = "dragonfly")]
#[path = "dragonfly/mod.rs"]
pub mod dragonfly;

#[cfg(target_os = "freebsd")]
#[path = "freebsd/mod.rs"]
pub mod freebsd;

#[cfg(target_os = "haiku")]
#[path = "haiku/mod.rs"]
pub mod haiku;

#[cfg(target_os = "illumos")]
#[path = "illumos/mod.rs"]
pub mod illumos;

#[cfg(target_os = "ios")]
#[path = "ios/mod.rs"]
pub mod ios;

#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
pub mod linux;

#[cfg(target_os = "macos")]
#[path = "macos/mod.rs"]
pub mod macos;

#[cfg(target_os = "netbsd")]
#[path = "netbsd/mod.rs"]
pub mod netbsd;

#[cfg(target_os = "openbsd")]
#[path = "openbsd/mod.rs"]
pub mod openbsd;

#[cfg(target_os = "solaris")]
#[path = "solaris/mod.rs"]
pub mod solaris;

#[cfg(any(
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "haiku",
    target_os = "illumos",
    target_os = "linux",
    target_os = "macos",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "solaris"
))]
#[path = "unix/mod.rs"]
pub mod unix;

#[cfg(not(any(
    target_os = "android",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "haiku",
    target_os = "illumos",
    target_os = "ios",
    target_os = "linux",
    target_os = "macos",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "solaris",
    windows,
)))]
#[path = "unsupported.rs"]
pub mod unsupported;

#[cfg(any(test, windows))]
#[path = "windows/mod.rs"]
pub mod windows;
