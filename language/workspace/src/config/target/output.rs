use serde::{Deserialize, Serialize};

/// How modules are discovered for a build target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum TargetDiscovery {
    /// Start from entry points and follow imports.
    /// Requires `entry` to be set. Used for bundles/executables.
    Entry,
    /// Compile all files matching `include` patterns.
    /// Each file becomes a separate output. Used for libraries.
    #[default]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
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
