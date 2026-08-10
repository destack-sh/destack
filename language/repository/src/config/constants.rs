/// Artifact directory below one repository language cache.
#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
pub(crate) const ARTIFACT_DIRECTORY: &str = "artifacts";
/// Blob directory below the language cache.
#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
pub(crate) const BLOB_DIRECTORY: &str = "blobs";
/// Cache directory below the selected Destack root.
#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
pub(crate) const CACHE_DIRECTORY: &str = "cache";
/// Language directory below the Destack cache.
#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
pub(crate) const LANGUAGE_DIRECTORY: &str = "language";
/// Repository partition directory below a shared language cache.
#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
pub(crate) const WORKSPACE_DIRECTORY: &str = "workspaces";

/// Default package directory below the Destack home.
pub(crate) const DEFAULT_PACKAGE_DIRECTORY: &str = "packages";
/// Default vendor directory below a workspace.
pub(crate) const DEFAULT_VENDOR_DIRECTORY: &str = "vendor";
/// Default workspace cache directory.
pub(crate) const DEFAULT_WORKSPACE_CACHE_DIRECTORY: &str = ".destack";

/// Default Destack home directory on Unix hosts.
pub(crate) const UNIX_DESTACK_HOME_DIRECTORY: &str = ".destack";
/// Default Destack home directory on Windows hosts.
pub(crate) const WINDOWS_DESTACK_HOME_DIRECTORY: &str = "Destack";
