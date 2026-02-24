#[cfg(target_os = "android")]
#[path = "android/mod.rs"]
mod android;
#[cfg(target_os = "ios")]
#[path = "ios/mod.rs"]
mod ios;
#[cfg(target_os = "macos")]
#[path = "macos/mod.rs"]
mod macos;
#[cfg(not(any(target_os = "android", target_os = "ios", target_os = "macos")))]
#[path = "posix/mod.rs"]
mod posix;

#[cfg(target_os = "android")]
pub(crate) use android::*;
#[cfg(target_os = "ios")]
pub(crate) use ios::*;
#[cfg(target_os = "macos")]
pub(crate) use macos::*;
#[cfg(not(any(target_os = "android", target_os = "ios", target_os = "macos")))]
pub(crate) use posix::*;
