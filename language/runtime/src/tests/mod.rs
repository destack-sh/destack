#[cfg(target_os = "macos")]
pub mod affinity;
#[cfg(not(target_os = "macos"))]
pub(crate) mod affinity;
#[cfg(test)]
mod bindings;
#[cfg(any(test, target_os = "macos"))]
pub(crate) mod platform;
#[cfg(any(test, target_os = "macos"))]
pub(crate) mod runtime;
