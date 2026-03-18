#[cfg(all(not(test), windows))]
mod service;
#[cfg(all(not(test), windows))]
mod submit;
#[cfg(all(test, windows))]
mod test;
#[cfg(all(test, not(windows)))]
mod unsupported;

#[cfg(all(not(test), windows))]
pub(crate) use service::unregister_location_runtime;
#[cfg(all(not(test), windows))]
pub(crate) use submit::request_location_permission;
#[cfg(all(not(test), windows))]
pub(crate) use submit::submit_location_request;
#[cfg(all(test, windows))]
pub(crate) use test::{
    request_location_permission, submit_location_request, unregister_location_runtime,
};
#[cfg(all(test, not(windows)))]
pub(crate) use unsupported::{
    request_location_permission, submit_location_request, unregister_location_runtime,
};
