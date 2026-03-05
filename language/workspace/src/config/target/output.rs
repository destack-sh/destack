use serde::{Deserialize, Serialize};

use destack_builtin::BuiltinOutputFormat;

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

/// Output format for a build target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum OutputFormat {
    /// JavaScript (.js).
    #[default]
    Js,
    /// TypeScript (.ts).
    Ts,
    /// WebAssembly (.wasm).
    Wasm,
    /// Native binary.
    Native,
}

impl OutputFormat {
    /// Whether this format produces JavaScript output.
    pub fn is_js(&self) -> bool {
        matches!(self, Self::Js)
    }

    /// Whether this format produces TypeScript output.
    pub fn is_ts(&self) -> bool {
        matches!(self, Self::Ts)
    }

    /// Whether this format produces WebAssembly output.
    pub fn is_wasm(&self) -> bool {
        matches!(self, Self::Wasm)
    }

    /// Whether this format produces native binary output.
    pub fn is_native(&self) -> bool {
        matches!(self, Self::Native)
    }

    /// Whether this format typically produces a single output file.
    pub fn is_single_file(&self) -> bool {
        matches!(self, Self::Wasm | Self::Native)
    }
}

impl From<OutputFormat> for BuiltinOutputFormat {
    fn from(value: OutputFormat) -> Self {
        match value {
            OutputFormat::Js => BuiltinOutputFormat::Js,
            OutputFormat::Ts => BuiltinOutputFormat::Ts,
            OutputFormat::Wasm => BuiltinOutputFormat::Wasm,
            OutputFormat::Native => BuiltinOutputFormat::Native,
        }
    }
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

/// Output format for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum OutputFormatJson {
    /// JavaScript (.js).
    #[serde(alias = "javascript")]
    Js,
    /// TypeScript (.ts).
    #[serde(alias = "typescript")]
    Ts,
    /// WebAssembly (.wasm).
    #[serde(alias = "webassembly")]
    Wasm,
    /// Native binary.
    #[serde(alias = "binary")]
    Native,
}

impl From<OutputFormatJson> for OutputFormat {
    fn from(value: OutputFormatJson) -> Self {
        match value {
            OutputFormatJson::Js => OutputFormat::Js,
            OutputFormatJson::Ts => OutputFormat::Ts,
            OutputFormatJson::Wasm => OutputFormat::Wasm,
            OutputFormatJson::Native => OutputFormat::Native,
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
