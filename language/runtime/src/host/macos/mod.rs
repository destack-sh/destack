#[cfg(any(test, target_os = "macos"))]
pub(crate) mod abi;
#[cfg(any(test, target_os = "macos"))]
mod action;
#[cfg(target_os = "macos")]
mod adapter;
#[cfg(any(test, target_os = "macos"))]
pub(crate) mod identity;
#[cfg(any(test, target_os = "macos"))]
pub(crate) mod ingress;
#[cfg(any(test, target_os = "macos"))]
pub(crate) mod request;
#[cfg(test)]
mod tests;

#[cfg(target_os = "macos")]
pub(crate) use adapter::MacosHost;
