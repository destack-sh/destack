mod config;
mod core;
mod identity;
mod io;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
mod operations;
mod state;
mod watch;

pub(super) use super::core::SERIAL_PORT_RESOURCE_LABEL;
#[cfg(target_os = "linux")]
pub(crate) use linux::*;
#[cfg(target_os = "macos")]
pub(crate) use macos::*;
pub(crate) use operations::*;
pub(crate) use watch::*;
