#[cfg(any(test, target_os = "android"))]
pub mod abi;
#[cfg(any(test, target_os = "android"))]
mod action;
#[cfg(target_os = "android")]
mod adapter;
#[cfg(any(test, target_os = "android"))]
pub(crate) mod ingress;
#[cfg(any(test, target_os = "android"))]
pub(crate) mod request;
#[cfg(test)]
mod tests;
#[cfg(target_os = "android")]
pub(crate) use adapter::AndroidHost;
