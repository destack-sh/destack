#[cfg(target_os = "linux")]
mod adapter;
#[cfg(any(test, target_os = "linux"))]
mod ingress;
#[cfg(test)]
mod tests;

#[cfg(target_os = "linux")]
pub(crate) use adapter::LinuxHost;
#[cfg(any(test, target_os = "linux"))]
pub use ingress::*;
