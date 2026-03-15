#[cfg(target_os = "macos")]
mod adapter;
#[cfg(any(test, target_os = "macos"))]
mod ingress;
#[cfg(any(test, target_os = "macos"))]
mod request;
#[cfg(test)]
mod tests;

#[cfg(target_os = "macos")]
pub(crate) use adapter::MacosHost;
#[cfg(any(test, target_os = "macos"))]
pub use ingress::*;
#[cfg(all(test, target_os = "macos"))]
pub(crate) use request::set_test_pick_hook;
#[cfg(target_os = "macos")]
pub(crate) use request::{
    session_capabilities as macos_request_capabilities, submit_request as submit_macos_request,
};
