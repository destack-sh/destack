#[cfg(target_os = "macos")]
mod coremidi;
#[cfg(target_os = "macos")]
pub(crate) use coremidi::*;

#[cfg(target_os = "linux")]
mod alsa;
#[cfg(target_os = "linux")]
pub(crate) use alsa::*;

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub(crate) use super::super::unsupported::*;
