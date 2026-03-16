pub(crate) mod app;
#[cfg(any(target_os = "macos", windows))]
pub(crate) mod document;

pub(crate) use app::*;
#[cfg(any(target_os = "macos", windows))]
pub(crate) use document::*;
