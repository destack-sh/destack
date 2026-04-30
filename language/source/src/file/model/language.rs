use serde::{Deserialize, Serialize};

use crate::FileType;

/// The source language type determines parsing and compatibility behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LanguageType {
    /// Full Destack language (`.ds`) with all features enabled.
    #[default]
    Destack,
    /// Destack declaration file (`.d.ds`).
    DestackDeclaration,
    /// JavaScript (`.js`) compatibility mode.
    JavaScript,
    /// JavaScript with JSX (`.jsx`) compatibility mode.
    JavaScriptXml,
    /// TypeScript (`.ts`) compatibility mode.
    TypeScript,
    /// TypeScript declaration (`.d.ts`) compatibility mode.
    TypeScriptDeclaration,
    /// TypeScript with JSX (`.tsx`) compatibility mode.
    TypeScriptXml,
}

impl TryFrom<FileType> for LanguageType {
    /// The non-code file type that could not be converted.
    type Error = FileType;

    /// Try to convert one file type into its source language.
    fn try_from(file_type: FileType) -> Result<Self, Self::Error> {
        match file_type {
            FileType::Destack | FileType::DestackText | FileType::DestackBinary => {
                Ok(Self::Destack)
            }
            FileType::DestackDeclaration => Ok(Self::DestackDeclaration),
            FileType::JavaScript => Ok(Self::JavaScript),
            FileType::JavaScriptXml => Ok(Self::JavaScriptXml),
            FileType::TypeScript => Ok(Self::TypeScript),
            FileType::TypeScriptDeclaration => Ok(Self::TypeScriptDeclaration),
            FileType::TypeScriptXml => Ok(Self::TypeScriptXml),
            _ => Err(file_type),
        }
    }
}

impl LanguageType {
    /// Whether this is a declaration file.
    #[inline]
    pub fn is_declaration(&self) -> bool {
        matches!(self, Self::DestackDeclaration | Self::TypeScriptDeclaration)
    }

    /// Whether this is a Destack language type (not compatibility mode).
    #[inline]
    pub fn is_destack(&self) -> bool {
        matches!(self, Self::Destack | Self::DestackDeclaration)
    }

    /// Whether this is JavaScript (JS or JSX).
    #[inline]
    pub fn is_javascript(&self) -> bool {
        matches!(self, Self::JavaScript | Self::JavaScriptXml)
    }

    /// Whether this is TypeScript (TS or TSX).
    #[inline]
    pub fn is_typescript(&self) -> bool {
        matches!(
            self,
            Self::TypeScript | Self::TypeScriptDeclaration | Self::TypeScriptXml
        )
    }

    /// Whether this language type supports JSX/tree literal syntax.
    #[inline]
    pub fn supports_jsx(&self) -> bool {
        matches!(
            self,
            Self::Destack | Self::DestackDeclaration | Self::JavaScriptXml | Self::TypeScriptXml
        )
    }

    /// Whether this language type supports private identifiers.
    #[inline]
    pub fn supports_private_identifiers(&self) -> bool {
        !self.is_destack()
    }

    /// Whether this language type supports declaration merging.
    #[inline]
    pub fn supports_declaration_merging(&self) -> bool {
        matches!(
            self,
            Self::DestackDeclaration
                | Self::TypeScript
                | Self::TypeScriptDeclaration
                | Self::TypeScriptXml
        )
    }

    /// Whether this language type supports module declarations.
    #[inline]
    pub fn supports_module_declaration(&self) -> bool {
        matches!(
            self,
            Self::Destack
                | Self::DestackDeclaration
                | Self::TypeScript
                | Self::TypeScriptDeclaration
                | Self::TypeScriptXml
        )
    }
}
