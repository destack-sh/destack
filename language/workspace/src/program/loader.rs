use destack_source::FileType;
use serde::{Deserialize, Serialize};

use crate::ModuleType;

/// How to load/interpret a file's content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
            Loader::Text | Loader::Base64 => ModuleType::Text,
            Loader::Binary | Loader::File => ModuleType::Binary,
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
        matches!(self, Loader::Text | Loader::Base64)
    }

    /// Whether this loader produces binary modules.
    pub fn is_binary(&self) -> bool {
        matches!(self, Loader::Binary | Loader::File)
    }

    /// Parse a loader from an import attribute type value string.
    ///
    /// Supports the `type` attribute values used in import attributes:
    /// ```text
    /// import data from "./file" with { type: "json" }
    /// ```
    pub fn from_type_attribute(value: &str) -> Option<Self> {
        match value {
            "json" => Some(Loader::Json),
            "toml" => Some(Loader::Toml),
            "yaml" => Some(Loader::Yaml),
            "text" => Some(Loader::Text),
            "binary" => Some(Loader::Binary),
            "file" => Some(Loader::File),
            "base64" => Some(Loader::Base64),
            "env" => Some(Loader::Env),
            _ => None,
        }
    }

    /// Get the loader name as a string (for hashing/salting).
    pub fn as_str(&self) -> &'static str {
        match self {
            Loader::Destack => "destack",
            Loader::TypeScript => "typescript",
            Loader::JavaScript => "javascript",
            Loader::Json => "json",
            Loader::Toml => "toml",
            Loader::Yaml => "yaml",
            Loader::Env => "env",
            Loader::Text => "text",
            Loader::Binary => "binary",
            Loader::File => "file",
            Loader::Base64 => "base64",
        }
    }

    /// Get the loader key for ModuleId hashing.
    ///
    /// Returns `Some(key)` if the loader differs from the default for the given file type.
    /// Returns `None` if this is the default loader (no salting needed).
    pub fn key_for_file_type(&self, file_type: FileType) -> Option<&'static str> {
        let default = Self::from_file_type(file_type);
        if *self == default {
            None
        } else {
            Some(self.as_str())
        }
    }
}
