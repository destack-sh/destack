use crate::{OutputFormat, Target, TargetDiscovery};
use destack_source::{FileContent, FileType, Uri};
use serde::{Deserialize, Serialize};

/// One derived output entry payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputContent {
    /// Text output.
    Text {
        /// The emitted text.
        code: String,
        /// The emitted file type.
        file_type: FileType,
    },
    /// JSON output.
    Json {
        /// The serialized JSON text.
        content: String,
        /// The parsed JSON value.
        value: serde_json::Value,
        /// The emitted file type.
        file_type: FileType,
    },
    /// Binary output.
    Binary {
        /// The emitted bytes.
        bytes: Vec<u8>,
        /// The emitted file type.
        file_type: FileType,
    },
}

impl OutputContent {
    /// Create JavaScript content.
    pub fn javascript(code: String) -> Self {
        Self::Text {
            code,
            file_type: FileType::JavaScript,
        }
    }

    /// Create TypeScript content.
    pub fn typescript(code: String) -> Self {
        Self::Text {
            code,
            file_type: FileType::TypeScript,
        }
    }

    /// Create TypeScript declaration content.
    pub fn declaration(code: String) -> Self {
        Self::Text {
            code,
            file_type: FileType::TypeScriptDeclaration,
        }
    }

    /// Create source map content.
    pub fn source_map(value: serde_json::Value) -> Self {
        let content = serde_json::to_string(&value).unwrap_or_default();
        Self::Json {
            content,
            value,
            file_type: FileType::SourceMap,
        }
    }

    /// Create WebAssembly content.
    pub fn wasm(bytes: Vec<u8>) -> Self {
        Self::Binary {
            bytes,
            file_type: FileType::Wasm,
        }
    }

    /// Create object content.
    pub fn object(bytes: Vec<u8>) -> Self {
        Self::Binary {
            bytes,
            file_type: FileType::Object,
        }
    }

    /// Return the file type for this content.
    pub fn file_type(&self) -> FileType {
        match self {
            Self::Text { file_type, .. }
            | Self::Json { file_type, .. }
            | Self::Binary { file_type, .. } => *file_type,
        }
    }

    /// Convert this content to file content.
    pub fn to_file_content(&self) -> FileContent {
        match self {
            Self::Text { code, .. } => FileContent::Text {
                content: code.clone(),
            },
            Self::Json { content, value, .. } => FileContent::Json {
                content: content.clone(),
                value: value.clone(),
            },
            Self::Binary { bytes, .. } => FileContent::Binary {
                content: bytes.clone(),
            },
        }
    }
}

/// One derived output entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputEntry {
    /// The output URI.
    pub uri: Uri,
    /// The output content.
    pub content: OutputContent,
    /// The related source URI when one exists.
    pub source: Option<Uri>,
}

/// One module output contract.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModuleOutputKind {
    /// One JavaScript module output, with optional declaration sidecars.
    #[default]
    JavaScript,
    /// One TypeScript module output.
    TypeScript,
    /// One WebAssembly module output.
    WebAssembly,
    /// One native object output.
    NativeObject,
}

impl ModuleOutputKind {
    /// Resolve one module output kind for one target.
    pub fn for_target(target: &Target) -> Self {
        match target.output {
            OutputFormat::Js => Self::JavaScript,
            OutputFormat::Ts => Self::TypeScript,
            OutputFormat::Wasm => Self::WebAssembly,
            OutputFormat::Native => Self::NativeObject,
        }
    }
}

/// One module target output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModuleOutput {
    /// The published module output contract.
    pub kind: ModuleOutputKind,
    /// The output entries.
    pub entries: Vec<OutputEntry>,
}

impl ModuleOutput {
    /// Create one module output.
    pub fn new(kind: ModuleOutputKind, entries: Vec<OutputEntry>) -> Self {
        Self { kind, entries }
    }

    /// Create one module output from one target.
    pub fn for_target(target: &Target, entries: Vec<OutputEntry>) -> Self {
        Self::new(ModuleOutputKind::for_target(target), entries)
    }
}

/// One package output contract.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PackageOutputKind {
    /// One package JavaScript output tree.
    #[default]
    JavaScript,
    /// One package TypeScript output tree.
    TypeScript,
    /// One bundled JavaScript output.
    JavaScriptBundle,
    /// One bundled TypeScript output.
    TypeScriptBundle,
    /// One package WebAssembly output.
    WebAssembly,
    /// One native library output.
    NativeLibrary,
    /// One native executable output.
    Executable,
}

impl PackageOutputKind {
    /// Resolve one package output kind for one target.
    pub fn for_target(target: &Target) -> Self {
        match target.output {
            OutputFormat::Js => {
                if target.is_single_file() {
                    Self::JavaScriptBundle
                } else {
                    Self::JavaScript
                }
            }
            OutputFormat::Ts => {
                if target.is_single_file() {
                    Self::TypeScriptBundle
                } else {
                    Self::TypeScript
                }
            }
            OutputFormat::Wasm => Self::WebAssembly,
            OutputFormat::Native => match target.discovery {
                TargetDiscovery::Entry => Self::Executable,
                TargetDiscovery::Include => Self::NativeLibrary,
            },
        }
    }
}

/// One package target output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackageOutput {
    /// The published package output contract.
    pub kind: PackageOutputKind,
    /// The output entries.
    pub entries: Vec<OutputEntry>,
}

impl PackageOutput {
    /// Create one package output.
    pub fn new(kind: PackageOutputKind, entries: Vec<OutputEntry>) -> Self {
        Self { kind, entries }
    }

    /// Create one package output from one target.
    pub fn for_target(target: &Target, entries: Vec<OutputEntry>) -> Self {
        Self::new(PackageOutputKind::for_target(target), entries)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{ModuleOutputKind, PackageOutputKind};
    use crate::{Target, TargetDiscovery};

    /// Match module output kinds to target output formats.
    #[test]
    fn test_module_output_kind_matches_target_output() {
        assert_eq!(
            ModuleOutputKind::for_target(&Target::js("web")),
            ModuleOutputKind::JavaScript
        );
        assert_eq!(
            ModuleOutputKind::for_target(&Target::ts("types")),
            ModuleOutputKind::TypeScript
        );
        assert_eq!(
            ModuleOutputKind::for_target(&Target::wasm_js("wasm")),
            ModuleOutputKind::WebAssembly
        );
        assert_eq!(
            ModuleOutputKind::for_target(&Target::native("native")),
            ModuleOutputKind::NativeObject
        );
    }

    /// Match package output kinds to current target packaging behavior.
    #[test]
    fn test_package_output_kind_matches_target_shape() {
        let mut js_bundle = Target::js("bundle");
        js_bundle.out_file = Some(PathBuf::from("bundle.js"));

        let mut ts_bundle = Target::ts("bundle");
        ts_bundle.out_file = Some(PathBuf::from("bundle.ts"));

        let native_library = Target::native("library");

        let mut native_executable = Target::native("app");
        native_executable.discovery = TargetDiscovery::Entry;

        assert_eq!(
            PackageOutputKind::for_target(&Target::js("web")),
            PackageOutputKind::JavaScript
        );
        assert_eq!(
            PackageOutputKind::for_target(&js_bundle),
            PackageOutputKind::JavaScriptBundle
        );
        assert_eq!(
            PackageOutputKind::for_target(&Target::ts("types")),
            PackageOutputKind::TypeScript
        );
        assert_eq!(
            PackageOutputKind::for_target(&ts_bundle),
            PackageOutputKind::TypeScriptBundle
        );
        assert_eq!(
            PackageOutputKind::for_target(&Target::wasm_js("wasm")),
            PackageOutputKind::WebAssembly
        );
        assert_eq!(
            PackageOutputKind::for_target(&native_library),
            PackageOutputKind::NativeLibrary
        );
        assert_eq!(
            PackageOutputKind::for_target(&native_executable),
            PackageOutputKind::Executable
        );
    }
}
