#[cfg(target_os = "ios")]
mod action;
#[cfg(target_os = "ios")]
mod adapter;
#[cfg(target_os = "ios")]
pub(crate) mod request;
#[cfg(all(test, target_os = "ios"))]
mod tests;
#[cfg(target_os = "ios")]
pub(crate) use adapter::IosHost;
