pub(crate) mod background;
pub(crate) mod calendar;
pub(crate) mod contact;
#[cfg(target_os = "android")]
mod dispatch;
#[cfg(target_os = "android")]
pub(crate) mod document;
pub(crate) mod intent;
pub(crate) mod location;
pub(crate) mod media;
pub(crate) mod notification;
#[cfg(target_os = "android")]
pub(crate) mod permission;

#[cfg(target_os = "android")]
pub(crate) use dispatch::submit_request;
