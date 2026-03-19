#[cfg(target_os = "linux")]
mod alsa;
#[cfg(target_os = "android")]
mod android;
#[cfg(any(unix, windows))]
mod backend;
#[cfg(target_os = "macos")]
mod coremidi;
#[cfg(any(unix, windows))]
mod event;
#[cfg(any(unix, windows))]
mod input;
#[cfg(target_os = "linux")]
mod jack;
#[cfg(any(unix, windows))]
mod output;
#[cfg(any(unix, windows))]
mod tests;
#[cfg(any(unix, windows))]
mod transport;
#[cfg(any(unix, windows))]
mod virtual_ports;
#[cfg(any(unix, windows))]
mod webmidi;
#[cfg(windows)]
mod winmidi;
#[cfg(windows)]
mod winmm;
#[cfg(windows)]
mod winrt;

#[cfg(any(unix, windows))]
pub(super) use tests::*;
