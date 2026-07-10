use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::FileType;

/// How source content is interpreted as a module.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, PartialOrd, Ord,
)]
pub enum Loader {
    /// Destack code.
    Destack,
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

impl TryFrom<FileType> for Loader {
    /// The output-only or unsupported file type.
    type Error = FileType;

    /// Try to convert one input file type into its default loader.
    fn try_from(file_type: FileType) -> Result<Self, Self::Error> {
        match file_type {
            FileType::Destack | FileType::DestackDeclaration => Ok(Self::Destack),
            FileType::Json => Ok(Self::Json),
            FileType::Toml => Ok(Self::Toml),
            FileType::Yaml => Ok(Self::Yaml),
            FileType::Env => Ok(Self::Text),
            FileType::Text
            | FileType::Markdown
            | FileType::Html
            | FileType::Css
            | FileType::Svg
            | FileType::SourceMap => Ok(Self::Text),
            FileType::Wasm
            | FileType::Node
            | FileType::Object
            | FileType::Image
            | FileType::Font
            | FileType::Audio
            | FileType::Video
            | FileType::Model
            | FileType::Neural
            | FileType::Document
            | FileType::Binary
            | FileType::Unknown => Ok(Self::Binary),
            FileType::Script => Err(file_type),
        }
    }
}

impl Loader {
    /// Return whether this loader produces code modules.
    pub fn is_code(self) -> bool {
        matches!(self, Self::Destack)
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
        let default = Self::try_from(file_type).ok();
        if default == Some(self) {
            return None;
        }

        Some(self.as_str())
    }

    /// Return the canonical extension for this loader when it has one.
    pub fn extension(self) -> Option<&'static str> {
        match self {
            Self::Destack => Some("ds"),
            Self::Json => Some("json"),
            Self::Toml => Some("toml"),
            Self::Yaml => Some("yaml"),
            Self::Text => Some("txt"),
            Self::Binary | Self::File | Self::Base64 => None,
        }
    }
}
