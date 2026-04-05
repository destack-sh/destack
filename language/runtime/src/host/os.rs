#[cfg(any(test, target_os = "android"))]
#[path = "android/mod.rs"]
pub mod android;

#[cfg(target_vendor = "apple")]
#[path = "apple/mod.rs"]
pub mod apple;

#[cfg(target_os = "ios")]
#[path = "ios/mod.rs"]
pub mod ios;

#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
pub mod linux;

#[cfg(target_os = "macos")]
#[path = "macos/mod.rs"]
pub mod macos;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "ios"))]
#[path = "unix/mod.rs"]
pub mod unix;

#[cfg(not(any(
    target_os = "android",
    target_os = "ios",
    target_os = "linux",
    target_os = "macos",
    windows,
)))]
#[path = "unsupported.rs"]
pub mod unsupported;

#[cfg(any(test, windows))]
#[path = "windows/mod.rs"]
pub mod windows;
