mod config;
mod core;
mod identity;
mod io;
mod operations;
mod service;
mod state;
mod watch;

pub(super) use super::core::SERIAL_PORT_RESOURCE_LABEL;
pub(crate) use operations::*;
pub(crate) use service::{WindowsSerialService, windows_serial_service};
pub(crate) use watch::*;
