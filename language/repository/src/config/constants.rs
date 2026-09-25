/// Default maximum artifact cache size.
pub(crate) const DEFAULT_CACHE_MAXIMUM_BYTES: u64 = 10 * 1024 * 1024 * 1024;
/// Default package directory below the toolchain home.
pub(crate) const DEFAULT_PACKAGE_DIRECTORY: &str = "packages";
/// Default vendor directory below a workspace.
pub(crate) const DEFAULT_VENDOR_DIRECTORY: &str = "vendor";
/// Default package source include patterns.
pub(crate) const DEFAULT_SOURCE_INCLUDE: &[&str] = &["src/**"];
/// Default package source exclude patterns.
pub(crate) const DEFAULT_SOURCE_EXCLUDE: &[&str] = &[
    "**/.tspp/**",
    "**/.git/**",
    "**/docs/**",
    "**/node_modules/**",
    "**/target/**",
    "**/vendor/**",
];

/// Settings file name within the toolchain home.
pub(crate) const SETTINGS_FILE_NAME: &str = "settings.json";

/// Default number of Runtime diagnostic entries retained in memory.
pub(crate) const DEFAULT_RUNTIME_DIAGNOSTIC_CAPACITY: u64 = 1024;

/// Default toolchain home directory on Unix hosts.
pub(crate) const UNIX_HOME_DIRECTORY: &str = ".tspp";
/// Default toolchain home directory on Windows hosts.
pub(crate) const WINDOWS_HOME_DIRECTORY: &str = "tspp";
