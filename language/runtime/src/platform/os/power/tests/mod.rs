#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    windows
))]
mod common;
#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    windows
))]
mod state;
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
mod suspend;
