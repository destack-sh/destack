#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod desktop;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub(crate) use desktop::{is_launcher_available, open_path_target, open_url_target};
