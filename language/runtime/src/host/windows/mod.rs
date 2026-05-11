#[cfg(any(test, windows))]
pub(crate) mod abi;
#[cfg(windows)]
mod action;
#[cfg(windows)]
mod adapter;
#[cfg(windows)]
pub(crate) mod identity;
#[cfg(any(test, windows))]
pub(crate) mod ingress;
#[cfg(windows)]
pub(crate) mod request;
#[cfg(test)]
pub(crate) mod tests;

#[cfg(windows)]
pub(crate) use adapter::WindowsHost;
