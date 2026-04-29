use std::hash::{Hash, Hasher};

use destack_artifact::{EmitFormat, TargetOutputKind};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// How modules are discovered for a build target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
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

/// Extra artifacts to emit for debugging or inspection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EmitArtifact {
    /// Lowered MIR for the module.
    Mir,
    /// Backend IR (LLVM/Cranelift).
    Ir,
    /// Assembly output.
    Asm,
    /// Object file output.
    Object,
    /// Symbol table output.
    Symbols,
}

impl std::str::FromStr for EmitArtifact {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "mir" => Ok(Self::Mir),
            "ir" | "llvm_ir" | "llvm" | "cranelift_ir" | "clif" => Ok(Self::Ir),
            "asm" | "assembly" => Ok(Self::Asm),
            "object" | "obj" => Ok(Self::Object),
            "symbols" | "sym" | "symtab" => Ok(Self::Symbols),
            _ => Err(()),
        }
    }
}

impl EmitArtifact {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Formal target output group options.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct TargetOutputOptions {
    /// Published output kind.
    pub kind: TargetOutputKind,
    /// Output topology.
    pub topology: TargetOutputTopology,
    /// Whether the output is intended for publication.
    pub is_public: bool,
}

/// Formal target output topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum TargetOutputTopology {
    /// One single output file.
    File,
    /// One mirrored directory tree of outputs.
    Directory,
    /// One logical collection of related output files.
    #[default]
    Collection,
}

impl TargetOutputTopology {
    /// Return the default topology for one output kind.
    pub fn default_for_kind(kind: TargetOutputKind) -> Self {
        match kind {
            TargetOutputKind::Module
            | TargetOutputKind::Entry
            | TargetOutputKind::Document
            | TargetOutputKind::Types
            | TargetOutputKind::Maps => Self::Directory,
            TargetOutputKind::Assets | TargetOutputKind::Manifest | TargetOutputKind::Metadata => {
                Self::Collection
            }
            TargetOutputKind::Binary => Self::File,
        }
    }
}

/// Formal named target outputs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TargetOutputs {
    /// Named output groups.
    pub groups: IndexMap<String, TargetOutputOptions>,
}

impl TargetOutputs {
    /// Create one empty target output set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert one named output group.
    pub fn insert(&mut self, name: impl Into<String>, output: TargetOutputOptions) {
        self.groups.insert(name.into(), output);
    }

    /// Return one output group by name.
    pub fn get(&self, name: &str) -> Option<&TargetOutputOptions> {
        self.groups.get(name)
    }

    /// Return whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    /// Return an iterator over the named output groups.
    pub fn iter(&self) -> indexmap::map::Iter<'_, String, TargetOutputOptions> {
        self.groups.iter()
    }

    /// Return whether any output matches the requested kind.
    pub fn contains_kind(&self, kind: TargetOutputKind) -> bool {
        self.groups.values().any(|output| output.kind == kind)
    }
}

impl Hash for TargetOutputs {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.groups.len().hash(state);

        for (name, output) in &self.groups {
            name.hash(state);
            output.hash(state);
        }
    }
}

/// Emit family for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum EmitFormatJson {
    /// JavaScript (.js).
    #[serde(alias = "javascript")]
    Js,
    /// TypeScript (.ts).
    #[serde(alias = "typescript")]
    Ts,
    /// HTML document output.
    Html,
    /// WebAssembly (.wasm).
    #[serde(alias = "webassembly")]
    Wasm,
    /// Native binary.
    #[serde(alias = "binary")]
    Native,
}

impl From<EmitFormatJson> for EmitFormat {
    fn from(value: EmitFormatJson) -> Self {
        match value {
            EmitFormatJson::Js => EmitFormat::Js,
            EmitFormatJson::Ts => EmitFormat::Ts,
            EmitFormatJson::Html => EmitFormat::Html,
            EmitFormatJson::Wasm => EmitFormat::Wasm,
            EmitFormatJson::Native => EmitFormat::Native,
        }
    }
}

/// Extra artifacts to emit for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum EmitArtifactJson {
    /// Lowered MIR.
    Mir,
    /// Backend IR.
    Ir,
    /// Assembly output.
    Asm,
    /// Object file output.
    Object,
    /// Symbol table output.
    Symbols,
}

impl From<EmitArtifactJson> for EmitArtifact {
    fn from(value: EmitArtifactJson) -> Self {
        match value {
            EmitArtifactJson::Mir => EmitArtifact::Mir,
            EmitArtifactJson::Ir => EmitArtifact::Ir,
            EmitArtifactJson::Asm => EmitArtifact::Asm,
            EmitArtifactJson::Object => EmitArtifact::Object,
            EmitArtifactJson::Symbols => EmitArtifact::Symbols,
        }
    }
}
