#[cfg(any(test, target_os = "dragonfly"))]
pub(crate) mod abi;
#[cfg(target_os = "dragonfly")]
mod adapter;
#[cfg(any(test, target_os = "dragonfly"))]
mod ingress;
#[cfg(test)]
mod tests;

#[cfg(target_os = "dragonfly")]
pub(crate) use adapter::DragonflyHost;
