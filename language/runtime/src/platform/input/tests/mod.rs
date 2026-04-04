#[cfg(any(unix, windows))]
mod basic;
#[cfg(any(target_os = "macos", windows))]
mod clipboard;
#[cfg(any(target_os = "macos", windows))]
mod clipboard_content;
#[cfg(any(unix, windows))]
mod tests;

#[cfg(any(target_os = "macos", windows))]
pub(crate) use clipboard_content::*;
#[cfg(any(unix, windows))]
pub(super) use tests::*;
