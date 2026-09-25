use std::path::Path;

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::FileType;

/// The TS++ source form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
pub enum LanguageType {
    /// TS++ implementation source (`.tspp`).
    #[default]
    Tspp,
    /// TS++ declaration source (`.d.tspp`).
    TsppDeclaration,
}

impl TryFrom<FileType> for LanguageType {
    /// The non-code file type that could not be converted.
    type Error = FileType;

    /// Try to convert one file type into its source language.
    fn try_from(file_type: FileType) -> Result<Self, Self::Error> {
        match file_type {
            FileType::Tspp => Ok(Self::Tspp),
            FileType::TsppDeclaration => Ok(Self::TsppDeclaration),
            _ => Err(file_type),
        }
    }
}

impl From<LanguageType> for FileType {
    /// Convert one source language into its corresponding file type.
    fn from(language: LanguageType) -> Self {
        match language {
            LanguageType::Tspp => Self::Tspp,
            LanguageType::TsppDeclaration => Self::TsppDeclaration,
        }
    }
}

impl LanguageType {
    /// Return the source language for one extension when supported.
    pub fn from_extension(extension: &str) -> Option<Self> {
        let file_type = FileType::from_extension(extension)?;

        Self::try_from(file_type).ok()
    }

    /// Return the source language for one path when supported.
    pub fn from_path(path: &Path) -> Option<Self> {
        let file_type = FileType::from_path(path)?;

        Self::try_from(file_type).ok()
    }

    /// Return whether this is declaration source.
    #[inline]
    pub fn is_declaration(self) -> bool {
        matches!(self, Self::TsppDeclaration)
    }
}
