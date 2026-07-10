use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::FileType;

/// The Destack source form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
pub enum LanguageType {
    /// Destack implementation source (`.ds`).
    #[default]
    Destack,
    /// Destack declaration source (`.d.ds`).
    DestackDeclaration,
}

impl TryFrom<FileType> for LanguageType {
    /// The non-code file type that could not be converted.
    type Error = FileType;

    /// Try to convert one file type into its source language.
    fn try_from(file_type: FileType) -> Result<Self, Self::Error> {
        match file_type {
            FileType::Destack => Ok(Self::Destack),
            FileType::DestackDeclaration => Ok(Self::DestackDeclaration),
            _ => Err(file_type),
        }
    }
}

impl From<LanguageType> for FileType {
    /// Convert one source language into its corresponding file type.
    fn from(language: LanguageType) -> Self {
        match language {
            LanguageType::Destack => Self::Destack,
            LanguageType::DestackDeclaration => Self::DestackDeclaration,
        }
    }
}

impl LanguageType {
    /// Whether this is a declaration file.
    #[inline]
    pub fn is_declaration(&self) -> bool {
        matches!(self, Self::DestackDeclaration)
    }
}
