#[cfg(target_os = "linux")]
mod background;
#[cfg(all(unix, not(target_vendor = "apple")))]
mod calendar;
#[cfg(all(unix, not(target_vendor = "apple")))]
mod contact;
mod dispatch;
#[cfg(all(unix, not(target_vendor = "apple")))]
mod document;
mod intent;
#[cfg(target_os = "linux")]
mod linux;
mod location;
mod media;
#[cfg(all(unix, not(target_vendor = "apple")))]
pub(crate) mod notification;

pub(crate) use dispatch::{request_actions, submit_request};
