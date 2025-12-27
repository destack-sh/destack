use std::path::PathBuf;

use destack_source::Uri;

use crate::{EnvSnapshot, Platform, Runtime};

#[derive(Debug, Clone)]
pub struct ImportMeta {
    /// The URL of the current module.
    pub url: Uri,
    /// The file path of the current module.
    pub file: Option<PathBuf>,
    /// The directory containing the current module.
    pub dir: Option<PathBuf>,
    /// The target platform (OS) being compiled for.
    pub platform: Platform,
    /// The runtime environment that will execute the code.
    pub runtime: Runtime,
    /// True if this is a debug build.
    pub debug: bool,
    /// Environment snapshot exposed to import.meta.env.
    pub env: EnvSnapshot,
}
