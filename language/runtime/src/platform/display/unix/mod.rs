mod core;
mod monitor;
mod monitor_event;
mod window;
mod window_event;

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "ios")]
mod ios;
#[cfg(target_os = "linux")]
mod linux_wayland;
#[cfg(target_os = "linux")]
mod linux_x11;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(all(
    unix,
    not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos"
    ))
))]
mod other;

pub(crate) use monitor::*;
pub(crate) use monitor_event::*;
pub(crate) use window::*;
pub(crate) use window_event::*;
