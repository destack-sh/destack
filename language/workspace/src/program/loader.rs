use destack_source::FileType;

use crate::ModuleType;

/// How to load/interpret a file's content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Loader {
    // Code loaders → ModuleType::Code
    /// Destack code (.ds, .d.ds)
    Destack,
    /// TypeScript code (.ts, .tsx, .d.ts)
    TypeScript,
    /// JavaScript code (.js, .jsx)
    JavaScript,

    // Data loaders → ModuleType::Data
    /// JSON data (.json)
    Json,
    /// TOML data (.toml)
    Toml,
    /// YAML data (.yaml, .yml)
    Yaml,
    /// Environment variables (.env)
    Env,

    // Text loaders → ModuleType::Text
    /// Plain text content
    Text,

    // Binary loaders → ModuleType::Binary
    /// Raw binary bytes
    Binary,
    /// URL to bundled asset (for bundler output)
    File,
    /// Base64-encoded string
    Base64,
}

impl Loader {
    /// Get the default loader for a file type.
    pub fn from_file_type(file_type: FileType) -> Self {
        match file_type {
            // Destack code
            FileType::Destack | FileType::DestackDeclaration => Loader::Destack,

            // TypeScript code
            FileType::TypeScript
            | FileType::TypeScriptXml
            | FileType::TypeScriptDeclaration
            | FileType::DestackText => Loader::TypeScript,

            // JavaScript code
            FileType::JavaScript | FileType::JavaScriptXml => Loader::JavaScript,

            // Data formats
            FileType::Json => Loader::Json,
            FileType::Toml => Loader::Toml,
            FileType::Yaml => Loader::Yaml,
            FileType::Env => Loader::Env,

            // Text formats
            FileType::Text
            | FileType::Markdown
            | FileType::Html
            | FileType::Css
            | FileType::Svg => Loader::Text,

            // Binary formats
            FileType::Wasm
            | FileType::Node
            | FileType::SourceMap
            | FileType::Object
            | FileType::DestackBinary
            | FileType::DestackAst
            | FileType::DestackDir
            | FileType::DestackMir
            | FileType::Image
            | FileType::Font
            | FileType::Audio
            | FileType::Video
            | FileType::Model
            | FileType::Neural
            | FileType::Document
            | FileType::Binary
            | FileType::Unknown => Loader::Binary,
        }
    }

    /// Get the module type that this loader produces.
    pub fn module_type(&self) -> ModuleType {
        match self {
            Loader::Destack | Loader::TypeScript | Loader::JavaScript => ModuleType::Code,
            Loader::Json | Loader::Toml | Loader::Yaml | Loader::Env => ModuleType::Data,
            Loader::Text => ModuleType::Text,
            Loader::Binary | Loader::File | Loader::Base64 => ModuleType::Binary,
        }
    }

    /// Whether this loader produces code modules.
    pub fn is_code(&self) -> bool {
        matches!(
            self,
            Loader::Destack | Loader::TypeScript | Loader::JavaScript
        )
    }

    /// Whether this loader produces data modules.
    pub fn is_data(&self) -> bool {
        matches!(
            self,
            Loader::Json | Loader::Toml | Loader::Yaml | Loader::Env
        )
    }

    /// Whether this loader produces text modules.
    pub fn is_text(&self) -> bool {
        matches!(self, Loader::Text)
    }

    /// Whether this loader produces binary modules.
    pub fn is_binary(&self) -> bool {
        matches!(self, Loader::Binary | Loader::File | Loader::Base64)
    }
}
