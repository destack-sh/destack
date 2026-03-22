use std::path::PathBuf;

use destack_source::Uri;
use serde::{Deserialize, Serialize};

use crate::{EmitFormat, Platform, ProfileEnv, Runtime};

/// The resolution semantics for one import edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ImportEdgeKind {
    /// Import-like edge semantics (`import`, `export from`, `import()`).
    #[default]
    Import,
    /// Require-like edge semantics (`require`, `import = require`).
    Require,
}

/// Structured target metadata exposed to `import.meta.target`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportMetaTarget {
    /// Coarse target family tag.
    pub family: String,
    /// Target vendor tag.
    pub vendor: String,
    /// Target environment tag when one exists.
    pub env: Option<String>,
    /// Target ABI tag when one exists.
    pub abi: Option<String>,
    /// Target architecture tag when one exists.
    pub arch: Option<String>,
}

/// Metadata about the current module and build configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// The emit format being compiled.
    pub emit: EmitFormat,
    /// The target platform (OS) being compiled for.
    pub platform: Platform,
    /// The runtime environment that will execute the code.
    pub runtime: Runtime,
    /// Structured target metadata.
    pub target: ImportMetaTarget,
    /// True if this is a debug build.
    pub debug: bool,
    /// True if this is a test build.
    pub test: bool,
    /// Environment values exposed to import.meta.env.
    pub env: ProfileEnv,
}
