#[cfg(target_os = "netbsd")]
mod adapter;
#[cfg(any(test, target_os = "netbsd"))]
mod ingress;
#[cfg(test)]
mod tests;

#[cfg(target_os = "netbsd")]
pub(crate) use adapter::NetBsdHost;
#[cfg(any(test, target_os = "netbsd"))]
pub use ingress::*;
