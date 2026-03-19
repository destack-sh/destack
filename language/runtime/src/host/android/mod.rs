#[cfg(any(test, target_os = "android"))]
mod abi;
#[cfg(target_os = "android")]
mod adapter;
#[cfg(any(test, target_os = "android"))]
pub(crate) mod bluetooth;
#[cfg(any(test, target_os = "android"))]
pub(crate) mod bridge;
#[cfg(any(test, target_os = "android"))]
pub(crate) mod camera;
#[cfg(any(test, target_os = "android"))]
mod ingress;
#[cfg(any(test, target_os = "android"))]
mod request;
#[cfg(test)]
mod tests;
#[cfg(any(test, target_os = "android"))]
pub(crate) mod usb;

#[cfg(target_os = "android")]
pub(crate) use adapter::AndroidHost;
#[cfg(test)]
pub(crate) use bridge::bindings::AndroidHostBindings;
#[cfg(any(test, target_os = "android"))]
pub(crate) use bridge::unregister_android_bindings;
#[cfg(any(test, target_os = "android"))]
pub(crate) use ingress::*;
#[cfg(any(test, target_os = "android"))]
pub(crate) use request::*;
