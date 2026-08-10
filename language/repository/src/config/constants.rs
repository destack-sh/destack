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

/// Default package source include patterns.
pub(crate) const DEFAULT_SOURCE_INCLUDE: &[&str] = &["src/**"];
/// Default package source exclude patterns.
pub(crate) const DEFAULT_SOURCE_EXCLUDE: &[&str] = &[
    ".destack/**",
    ".git/**",
    "node_modules/**",
    "target/**",
    "vendor/**",
];

/// Settings file name within the Destack home.
pub(crate) const SETTINGS_FILE_NAME: &str = "settings.json";

/// Default number of Runtime diagnostic entries retained in memory.
pub(crate) const DEFAULT_RUNTIME_DIAGNOSTIC_CAPACITY: u64 = 1024;

/// Default Destack home directory on Unix hosts.
pub(crate) const UNIX_DESTACK_HOME_DIRECTORY: &str = ".destack";
/// Default Destack home directory on Windows hosts.
pub(crate) const WINDOWS_DESTACK_HOME_DIRECTORY: &str = "Destack";
