#[cfg(any(test, target_os = "openbsd"))]
pub(crate) mod abi;
#[cfg(target_os = "openbsd")]
mod adapter;
#[cfg(any(test, target_os = "openbsd"))]
mod ingress;
#[cfg(test)]
mod tests;

#[cfg(target_os = "openbsd")]
pub(crate) use adapter::OpenBsdHost;
