#[cfg(any(test, target_os = "android"))]
mod abi;
#[cfg(target_os = "android")]
mod adapter;
#[cfg(any(test, target_os = "android"))]
mod bridge;
#[cfg(any(test, target_os = "android"))]
mod ingress;
#[cfg(any(test, target_os = "android"))]
mod request;
#[cfg(test)]
mod tests;

#[cfg(target_os = "android")]
pub(crate) use adapter::AndroidHost;
#[cfg(any(test, target_os = "android"))]
pub(crate) use bridge::unregister_android_bindings;
#[cfg(any(test, target_os = "android"))]
pub use bridge::*;
#[cfg(any(test, target_os = "android"))]
pub use ingress::*;
#[cfg(any(test, target_os = "android"))]
pub use request::*;
