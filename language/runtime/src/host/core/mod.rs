pub(crate) mod adapter;
#[cfg_attr(not(any(target_os = "android", target_os = "ios")), allow(dead_code))]
pub(crate) mod callback;
pub(crate) mod error;
pub(crate) mod event;
pub(crate) mod queue;
pub(crate) mod registry;
pub(crate) mod request;
pub(crate) mod session;
pub(crate) mod status;
pub(crate) mod target;
