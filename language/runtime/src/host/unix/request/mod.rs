#[cfg(all(unix, not(target_vendor = "apple")))]
mod calendar;
#[cfg(all(unix, not(target_vendor = "apple")))]
mod contact;
#[cfg(all(unix, not(target_vendor = "apple")))]
mod document;
mod intent;
mod location;
mod submit;

pub(crate) use submit::*;
