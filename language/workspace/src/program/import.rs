use std::path::PathBuf;

use destack_source::Uri;

use crate::{Platform, ProfileEnv, Runtime};

/// Metadata about the current module and build configuration.
#[derive(Debug, Clone)]
pub struct ImportMeta {
    /// The URL of the current module.
    pub url: Uri,
    /// The file system path of the current module.
    pub path: Option<PathBuf>,
    /// The file system path of the current module.
    pub file: Option<PathBuf>,
    /// Alias of `file`.
    pub filename: Option<PathBuf>,
    /// The directory containing the current module.
    pub dir: Option<PathBuf>,
    /// The directory containing the current module.
    pub dirname: Option<PathBuf>,
    /// The target platform (OS) being compiled for.
    pub platform: Platform,
    /// The runtime environment that will execute the code.
    pub runtime: Runtime,
    /// True if this is a debug build.
    pub debug: bool,
    /// True if this is a test build.
    pub test: bool,
    /// Environment values exposed to import.meta.env.
    pub env: ProfileEnv,
}
