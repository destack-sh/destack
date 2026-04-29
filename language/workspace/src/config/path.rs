use std::path::PathBuf;

use crate::config::{DESTACK_CACHE_DIR, HOME, LOCAL_APPDATA, USERPROFILE, XDG_CACHE_HOME};

const UNIX_HOME_CACHE_DIRECTORY: &str = ".cache";

/// Return an explicit cache directory from the environment.
pub fn cache_dir_from_env() -> Option<PathBuf> {
    // read override value
    let value = std::env::var_os(DESTACK_CACHE_DIR)?;

    // skip empty values
    if value.is_empty() {
        return None;
    }

    // build path from override
    Some(PathBuf::from(value))
}

/// Resolve a global cache root directory for the given name.
pub fn resolve_global_cache_root(dir_name: &str) -> Option<PathBuf> {
    // prefer explicit override
    if let Some(dir) = cache_dir_from_env() {
        return Some(dir);
    }

    // use xdg cache home when set
    if let Some(xdg) = std::env::var_os(XDG_CACHE_HOME) {
        return Some(PathBuf::from(xdg).join(dir_name));
    }

    // fallback to home cache directory on unix
    if let Some(home) = std::env::var_os(HOME) {
        return Some(
            PathBuf::from(home)
                .join(UNIX_HOME_CACHE_DIRECTORY)
                .join(dir_name),
        );
    }

    // use local app data on windows
    if let Some(local) = std::env::var_os(LOCAL_APPDATA) {
        return Some(PathBuf::from(local).join(dir_name));
    }

    // fallback to user profile on windows
    if let Some(profile) = std::env::var_os(USERPROFILE) {
        return Some(
            PathBuf::from(profile)
                .join(UNIX_HOME_CACHE_DIRECTORY)
                .join(dir_name),
        );
    }

    None
}
