#[cfg(any(unix, windows))]
mod backend;
#[cfg(target_os = "macos")]
mod coremidi;
#[cfg(any(unix, windows))]
mod event;
#[cfg(any(unix, windows))]
mod input;
#[cfg(any(unix, windows))]
mod output;
#[cfg(any(unix, windows))]
mod tests;
#[cfg(any(unix, windows))]
mod transport;
#[cfg(any(unix, windows))]
mod virtual_ports;
#[cfg(windows)]
mod winrt;

#[cfg(any(unix, windows))]
pub(super) use tests::*;
