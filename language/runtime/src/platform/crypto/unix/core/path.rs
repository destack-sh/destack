use std::env;
use std::path::{Path, PathBuf};

use crate::platform::crypto::CryptoStoreKind;
use crate::runtime::BindingCallContext;

/// Return configured or default system certificate bundle files.
pub(crate) fn configured_system_certificate_files(
    binding: &BindingCallContext,
    default_files: &[&str],
) -> Vec<PathBuf> {
    // prefer explicit runtime option overrides
    let configured_files = &binding.worker().options.crypto.system_certificate_files;
    if !configured_files.is_empty() {
        return configured_files.clone();
    }

    // fall back to host-default bundle file candidates
    default_files.iter().map(PathBuf::from).collect()
}

/// Return configured or default system certificate directories.
pub(crate) fn configured_system_certificate_directories(
    binding: &BindingCallContext,
    default_directories: &[&str],
) -> Vec<PathBuf> {
    // prefer explicit runtime option overrides
    let configured_directories = &binding
        .worker()
        .options
        .crypto
        .system_certificate_directories;
    if !configured_directories.is_empty() {
        return configured_directories.clone();
    }

    // fall back to host-default directory candidates
    default_directories.iter().map(PathBuf::from).collect()
}

/// Return one configured host-store snapshot path for one lane.
pub(crate) fn configured_keystore_path(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
) -> Option<PathBuf> {
    match kind {
        CryptoStoreKind::User => binding
            .worker()
            .options
            .crypto
            .host_store_paths
            .user
            .clone(),
        CryptoStoreKind::Machine => binding
            .worker()
            .options
            .crypto
            .host_store_paths
            .machine
            .clone(),
        _ => None,
    }
}

/// Return one host key-store snapshot path for one lane when available.
pub(crate) fn keystore_path(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    user_relative_path: &str,
    machine_absolute_path: &str,
) -> Option<PathBuf> {
    // user-lane path can be configured or derived from HOME
    if kind == CryptoStoreKind::User {
        if let Some(path) = configured_keystore_path(binding, CryptoStoreKind::User) {
            return Some(path);
        }

        let home_directory = env::var_os("HOME")?;
        return Some(Path::new(&home_directory).join(user_relative_path));
    }

    // machine-lane path can be configured or use one default absolute path
    if kind == CryptoStoreKind::Machine {
        if let Some(path) = configured_keystore_path(binding, CryptoStoreKind::Machine) {
            return Some(path);
        }

        return Some(PathBuf::from(machine_absolute_path));
    }

    None
}
