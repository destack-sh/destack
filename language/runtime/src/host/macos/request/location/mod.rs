#[cfg(all(not(test), target_os = "macos"))]
mod core;
#[cfg(all(not(test), target_os = "macos"))]
mod delegate;
#[cfg(all(not(test), target_os = "macos"))]
mod permission;
#[cfg(all(not(test), target_os = "macos"))]
mod sample;
#[cfg(all(not(test), target_os = "macos"))]
mod service;
#[cfg(all(not(test), target_os = "macos"))]
pub(crate) use core::request_location_permission;
#[cfg(all(test, target_os = "macos"))]
mod test;
#[cfg(all(test, not(target_os = "macos")))]
mod unsupported;

#[cfg(all(not(test), target_os = "macos"))]
pub(crate) use core::submit_location_request;
#[cfg(all(not(test), target_os = "macos"))]
pub(crate) use core::unregister_location_runtime;
#[cfg(all(test, target_os = "macos"))]
pub(crate) use test::{
    MacosLocationHooks, request_location_permission, set_macos_location_test_hooks,
    submit_location_request, unregister_location_runtime,
};
#[cfg(all(test, not(target_os = "macos")))]
pub(crate) use unsupported::{
    request_location_permission, submit_location_request, unregister_location_runtime,
};
