#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "android")]
pub(crate) use android::*;

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod coremidi;
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(crate) use coremidi::*;

#[cfg(target_os = "linux")]
mod alsa;
#[cfg(target_os = "linux")]
mod backend;
#[cfg(target_os = "linux")]
mod jack;
#[cfg(target_os = "linux")]
pub(crate) use alsa::{AlsaService, alsa_service};
#[cfg(target_os = "linux")]
pub(crate) use backend::*;
#[cfg(target_os = "linux")]
pub(crate) use jack::{JackService, jack_service};

#[cfg(not(any(
    target_os = "android",
    target_os = "macos",
    target_os = "ios",
    target_os = "linux"
)))]
#[path = "../unsupported.rs"]
mod unsupported;
#[cfg(not(any(
    target_os = "android",
    target_os = "macos",
    target_os = "ios",
    target_os = "linux"
)))]
pub(crate) use unsupported::*;
