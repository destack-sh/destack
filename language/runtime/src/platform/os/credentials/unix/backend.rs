#[cfg(target_os = "android")]
use super::android;
#[cfg(any(target_os = "macos", target_os = "ios"))]
use super::apple;
#[cfg(target_os = "linux")]
use super::linux;
#[cfg(not(any(
    target_os = "android",
    target_os = "ios",
    target_os = "linux",
    target_os = "macos",
)))]
use super::posix;

#[cfg(target_os = "android")]
pub(crate) use android::{
    authenticate_credentials, contains_credentials, delete_credentials, read_credentials,
    write_credentials,
};
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(crate) use apple::{
    authenticate_credentials, contains_credentials, delete_credentials, read_credentials,
    write_credentials,
};
#[cfg(target_os = "linux")]
pub(crate) use linux::{
    authenticate_credentials, contains_credentials, delete_credentials, read_credentials,
    write_credentials,
};
#[cfg(not(any(
    target_os = "android",
    target_os = "ios",
    target_os = "linux",
    target_os = "macos",
)))]
pub(crate) use posix::{
    authenticate_credentials, contains_credentials, delete_credentials, read_credentials,
    write_credentials,
};
