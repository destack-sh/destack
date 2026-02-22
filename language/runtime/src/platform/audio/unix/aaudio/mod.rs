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

pub(crate) use device::*;
pub(crate) use stream::*;
