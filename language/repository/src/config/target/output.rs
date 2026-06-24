use std::path::PathBuf;

use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Target output paths and metadata options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct TargetOutputOptions {
    /// Output directory for this target.
    pub directory: PathBuf,
    /// Output file for single-file targets.
    pub file: Option<PathBuf>,
    /// Whether to emit declaration files.
    pub declaration: bool,
    /// Separate directory for declaration files.
    pub declaration_directory: Option<PathBuf>,
    /// Source map emission mode.
    pub source_map: Option<SourceMapMode>,
}

impl Default for TargetOutputOptions {
    fn default() -> Self {
        Self {
            directory: PathBuf::from("dist"),
            file: None,
            declaration: false,
            declaration_directory: None,
            source_map: None,
        }
    }
}

/// How one target chooses its root module set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TargetRoot {
    /// Use explicit entry modules and follow imports.
    Entry,
    /// Use source files selected by include patterns.
    Include,
}

/// Output mode for build targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OutputMode {
    /// One output file per source file, preserving directory structure.
    /// Uses `out_dir` for the output directory.
    #[default]
    Directory,
    /// Single bundled/compiled output file.
    /// Uses `out_file` for the output path.
    File,
}

/// Source map emission mode for one target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum SourceMapMode {
    /// Emit external source map files and reference them from outputs.
    #[default]
    External,
    /// Inline source maps into the emitted output text.
    Inline,
    /// Emit external source map files without referencing them from outputs.
    Hidden,
}

impl SourceMapMode {
    /// Return whether this mode emits standalone map outputs.
    pub fn emits_output(self) -> bool {
        matches!(self, Self::External | Self::Hidden)
    }

    /// Return whether this mode inlines maps into emitted text.
    pub fn is_inline(self) -> bool {
        matches!(self, Self::Inline)
    }
}
