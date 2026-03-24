#[cfg(any(test, target_os = "ios"))]
pub(crate) mod abi;
#[cfg(target_os = "ios")]
mod adapter;
#[cfg(any(test, target_os = "ios"))]
mod capability;
#[cfg(any(test, target_os = "ios"))]
pub(crate) mod ingress;
#[cfg(any(test, target_os = "ios"))]
pub(crate) mod request;
#[cfg(test)]
mod tests;
#[cfg(target_os = "ios")]
pub(crate) use adapter::IosHost;
