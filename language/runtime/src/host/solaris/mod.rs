#[cfg(target_os = "solaris")]
mod adapter;
#[cfg(any(test, target_os = "solaris"))]
mod ingress;
#[cfg(test)]
mod tests;

#[cfg(target_os = "solaris")]
pub(crate) use adapter::SolarisHost;
#[cfg(any(test, target_os = "solaris"))]
pub use ingress::*;
