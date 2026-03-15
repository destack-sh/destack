#[cfg(target_os = "ios")]
mod adapter;
#[cfg(any(test, target_os = "ios"))]
mod bridge;
#[cfg(any(test, target_os = "ios"))]
mod ingress;
#[cfg(any(test, target_os = "ios"))]
mod request;
#[cfg(test)]
mod tests;

#[cfg(target_os = "ios")]
pub(crate) use adapter::IosHost;
#[cfg(any(test, target_os = "ios"))]
pub(crate) use bridge::unregister_ios_bindings;
#[cfg(any(test, target_os = "ios"))]
pub use bridge::*;
#[cfg(any(test, target_os = "ios"))]
pub use ingress::*;
#[cfg(any(test, target_os = "ios"))]
pub use request::*;
