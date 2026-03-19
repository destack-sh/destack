pub(crate) mod ffi;
#[cfg(windows)]
pub(crate) mod message;
pub(crate) mod notify;

#[cfg(windows)]
pub(crate) use message::process_ingress_loop;
pub(crate) use notify::*;
