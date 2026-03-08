#[cfg(feature = "affinity")]
pub mod affinity;
#[cfg(all(test, not(feature = "affinity")))]
pub(crate) mod affinity;
#[cfg(test)]
mod bindings;
#[cfg(any(test, feature = "affinity"))]
pub(crate) mod platform;
#[cfg(feature = "affinity")]
mod registry;
#[cfg(any(test, feature = "affinity"))]
pub(crate) mod runtime;
