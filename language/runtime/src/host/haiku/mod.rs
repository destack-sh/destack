#[cfg(target_os = "haiku")]
mod adapter;
#[cfg(any(test, target_os = "haiku"))]
mod ingress;
#[cfg(test)]
mod tests;

#[cfg(target_os = "haiku")]
pub(crate) use adapter::HaikuHost;
#[cfg(any(test, target_os = "haiku"))]
pub use ingress::*;
