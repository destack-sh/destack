mod backend;
mod core;
mod event;
mod monitor;
mod window;

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "macos")]
mod appkit;
#[cfg(target_os = "ios")]
mod ios;
#[cfg(target_os = "linux")]
mod wayland;
#[cfg(target_os = "linux")]
mod x11;

#[cfg(target_os = "macos")]
pub(crate) use appkit::AppKitRuntimeState;
pub(crate) use backend::display_backend_descriptors;
pub(crate) use event::*;
pub(crate) use monitor::*;
#[cfg(target_os = "linux")]
pub(crate) use wayland::WaylandRuntimeState;
pub(crate) use window::*;
#[cfg(target_os = "linux")]
pub(crate) use x11::X11RuntimeState;
