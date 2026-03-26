mod backend;
mod core;
mod event;
mod monitor;
mod window;

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "macos")]
pub(crate) mod appkit;
#[cfg(target_os = "ios")]
mod ios;
#[cfg(target_os = "linux")]
mod wayland;
#[cfg(target_os = "linux")]
mod x11;

#[cfg(target_os = "macos")]
pub(crate) use appkit::{AppKitDisplayService, AppKitRuntimeState, appkit_display_service};
pub(crate) use backend::display_backend_descriptors;
pub(crate) use event::*;
pub(crate) use monitor::*;
#[cfg(target_os = "linux")]
pub(crate) use wayland::{WaylandDisplayService, WaylandRuntimeState, wayland_display_service};
pub(crate) use window::*;
#[cfg(target_os = "linux")]
pub(crate) use x11::{X11DisplayService, X11RuntimeState, x11_display_service};
