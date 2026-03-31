#[cfg(any(unix, windows))]
mod basic;
#[cfg(all(feature = "execution", any(target_os = "macos", windows)))]
mod clipboard;
#[cfg(all(feature = "execution", any(target_os = "macos", windows)))]
mod clipboard_content;
#[cfg(any(unix, windows))]
mod tests;

#[cfg(all(feature = "execution", any(target_os = "macos", windows)))]
pub(crate) use clipboard_content::*;
#[cfg(any(unix, windows))]
pub(super) use tests::*;
