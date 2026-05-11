#[cfg(all(unix, not(target_vendor = "apple"), not(target_os = "linux")))]
mod action;
pub(crate) mod backend;
#[cfg(all(unix, not(target_vendor = "apple"), not(target_os = "linux")))]
mod category;
#[cfg(all(unix, not(target_vendor = "apple"), not(target_os = "linux")))]
mod core;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(all(unix, not(target_vendor = "apple"), not(target_os = "linux")))]
mod response;
mod submit;

pub(crate) use submit::submit_notification_request;
