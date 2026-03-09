#[cfg(target_os = "android")]
mod abi;
#[cfg(target_os = "android")]
mod constants;
#[cfg(target_os = "android")]
mod core;
#[cfg(target_os = "android")]
mod descriptor;
mod device;
#[cfg(target_os = "android")]
mod ids;
#[cfg(target_os = "android")]
mod runtime;
mod stream;

pub(crate) use device::{enumerate_host_devices, is_backend_supported};
pub(crate) use stream::open_host_stream;
