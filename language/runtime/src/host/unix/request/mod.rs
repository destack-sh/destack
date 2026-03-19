#[cfg(all(unix, not(target_vendor = "apple")))]
mod calendar;
#[cfg(all(unix, not(target_vendor = "apple")))]
mod contact;
mod dispatch;
#[cfg(all(unix, not(target_vendor = "apple")))]
mod document;
mod intent;
mod location;

pub(crate) use dispatch::*;
