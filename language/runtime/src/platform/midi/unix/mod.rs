#[cfg(target_os = "macos")]
mod coremidi;
#[cfg(target_os = "macos")]
pub(crate) use coremidi::*;

#[cfg(not(target_os = "macos"))]
pub(crate) use super::unsupported::*;
