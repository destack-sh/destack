use destack_source::FileType;
use serde::{Deserialize, Serialize};

use crate::Loader;

/// The semantic kind of one source module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum ModuleKind {
    /// A code module with language semantics.
    Code,
    /// An HTML document module.
    Html,
    /// A CSS stylesheet module.
    Css,
    /// An SVG document module.
    Svg,
    /// A data-like module with no structured dependency surface.
    Data,
    /// A resource module emitted or referenced as an asset.
    Resource,
}

impl ModuleKind {
    /// Classify one source file type into a source module kind.
    pub fn from_file_type(file_type: FileType) -> Self {
        Self::from_file_type_and_loader(file_type, Loader::from_file_type(file_type))
    }

    /// Classify one source file type under one effective loader.
    pub fn from_file_type_and_loader(file_type: FileType, loader: Loader) -> Self {
        let default_loader = Loader::from_file_type(file_type);

        if loader == default_loader {
            return match file_type {
                FileType::Html => Self::Html,
                FileType::Css => Self::Css,
                FileType::Svg => Self::Svg,
                _ => Self::from_loader(loader),
            };
        }

        Self::from_loader(loader)
    }

    /// Classify one source module directly from its effective loader.
    pub fn from_loader(loader: Loader) -> Self {
        match loader {
            Loader::Destack | Loader::TypeScript | Loader::JavaScript => Self::Code,
            Loader::File => Self::Resource,
            Loader::Json
            | Loader::Toml
            | Loader::Yaml
            | Loader::Text
            | Loader::Binary
            | Loader::Base64 => Self::Data,
        }
    }

    /// Return whether this module kind participates in the code pipeline.
    pub fn is_code(self) -> bool {
        self == Self::Code
    }

    /// Return whether this module kind is a document surface.
    pub fn is_document(self) -> bool {
        matches!(self, Self::Html | Self::Svg)
    }
}
