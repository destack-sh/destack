#[cfg(windows)]
mod adapter;
#[cfg(any(test, windows))]
mod ingress;
#[cfg(windows)]
mod request;
#[cfg(test)]
mod tests;

#[cfg(windows)]
pub(crate) use adapter::WindowsHost;
#[cfg(windows)]
pub(crate) use ingress::process_ingress_loop;
#[cfg(any(test, windows))]
pub use ingress::*;
#[cfg(all(test, windows))]
pub(crate) use request::set_test_pick_hook;
#[cfg(windows)]
pub(crate) use request::{
    session_capabilities as windows_request_capabilities, submit_request as submit_windows_request,
};
