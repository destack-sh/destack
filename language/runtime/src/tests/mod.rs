#[cfg(test)]
mod bindings;
#[cfg(feature = "execution")]
pub mod execution;
#[cfg(all(test, not(feature = "execution")))]
pub(crate) mod execution;
#[cfg(any(test, feature = "execution"))]
pub(crate) mod platform;
#[cfg(feature = "execution")]
mod registry;
#[cfg(any(test, feature = "execution"))]
pub(crate) mod runtime;
