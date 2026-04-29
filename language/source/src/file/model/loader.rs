use serde::{Deserialize, Serialize};

use crate::FileType;

/// How source content is interpreted as a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Loader {
    /// Destack code.
    Destack,
    /// TypeScript code.
    TypeScript,
    /// JavaScript code.
    JavaScript,
    /// JSON data.
    Json,
    /// TOML data.
    Toml,
    /// YAML data.
    Yaml,
    /// Plain text content.
    Text,
    /// Raw binary bytes.
    Binary,
    /// URL to bundled asset.
    File,
    /// Base64 encoded string.
    Base64,
}

impl Loader {
    /// Return the default loader for a file type.
    pub fn from_file_type(file_type: FileType) -> Self {
        match file_type {
            FileType::Destack | FileType::DestackDeclaration => Self::Destack,
            FileType::TypeScript
            | FileType::TypeScriptXml
            | FileType::TypeScriptDeclaration
            | FileType::DestackText => Self::TypeScript,
            FileType::JavaScript | FileType::JavaScriptXml => Self::JavaScript,
            FileType::Json => Self::Json,
            FileType::Toml => Self::Toml,
            FileType::Yaml => Self::Yaml,
            FileType::Env => Self::Text,
            FileType::Text
            | FileType::Markdown
            | FileType::Html
            | FileType::Css
            | FileType::Svg
            | FileType::SourceMap
            | FileType::DestackMir => Self::Text,
            FileType::Wasm
            | FileType::Node
            | FileType::Object
            | FileType::DestackBinary
            | FileType::Image
            | FileType::Font
            | FileType::Audio
            | FileType::Video
            | FileType::Model
            | FileType::Neural
            | FileType::Document
            | FileType::Binary
            | FileType::Unknown => Self::Binary,
        }
    }

    /// Return whether this loader produces code modules.
    pub fn is_code(self) -> bool {
        matches!(self, Self::Destack | Self::TypeScript | Self::JavaScript)
    }

    /// Return whether this loader produces data modules.
    pub fn is_data(self) -> bool {
        matches!(self, Self::Json | Self::Toml | Self::Yaml)
    }

    /// Return whether this loader produces text modules.
    pub fn is_text(self) -> bool {
        matches!(self, Self::Text | Self::Base64)
    }

    /// Return whether this loader produces asset reference modules.
    pub fn is_file(self) -> bool {
        matches!(self, Self::File)
    }

    /// Return whether this loader produces binary modules.
    pub fn is_binary(self) -> bool {
        matches!(self, Self::Binary)
    }

    /// Parse a loader from an import attribute type string.
    pub fn from_type_attribute(value: &str) -> Option<Self> {
        match value {
            "json" => Some(Self::Json),
            "toml" => Some(Self::Toml),
            "yaml" => Some(Self::Yaml),
            "text" => Some(Self::Text),
            "binary" => Some(Self::Binary),
            "file" => Some(Self::File),
            "base64" => Some(Self::Base64),
            _ => None,
        }
    }

    /// Return the stable loader name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Destack => "destack",
            Self::TypeScript => "typescript",
            Self::JavaScript => "javascript",
            Self::Json => "json",
            Self::Toml => "toml",
            Self::Yaml => "yaml",
            Self::Text => "text",
            Self::Binary => "binary",
            Self::File => "file",
            Self::Base64 => "base64",
        }
    }

    /// Return the module key salt when this loader differs from the file default.
    pub fn key_for_file_type(self, file_type: FileType) -> Option<&'static str> {
        let default = Self::from_file_type(file_type);
        if self == default {
            return None;
        }

        Some(self.as_str())
    }
}
