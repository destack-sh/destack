use crate::FileType;

/// The source language type determines parsing and compatibility behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LanguageType {
    /// Full Destack language (.ds, .dst) with all features enabled.
    #[default]
    Destack,
    /// Destack declaration file (.d.ds).
    DestackDeclaration,
    /// JavaScript (.js) in compatibility mode.
    JavaScript,
    /// JavaScript with JSX (.jsx) in compatibility mode.
    JavaScriptXml,
    /// TypeScript (.ts) in compatibility mode.
    TypeScript,
    /// TypeScript with JSX (.tsx) in compatibility mode.
    TypeScriptXml,
}

impl LanguageType {
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
        matches!(self, Self::TypeScript | Self::TypeScriptXml)
    }

    /// Whether this language type supports JSX/tree literal syntax.
    #[inline]
    pub fn supports_jsx(&self) -> bool {
        matches!(
            self,
            Self::Destack | Self::DestackDeclaration | Self::JavaScriptXml | Self::TypeScriptXml
        )
    }
}

impl From<FileType> for LanguageType {
    fn from(file_type: FileType) -> Self {
        match file_type {
            FileType::Destack | FileType::DestackText | FileType::DestackBinary => {
                LanguageType::Destack
            }
            FileType::DestackDeclaration => LanguageType::DestackDeclaration,
            FileType::JavaScript => LanguageType::JavaScript,
            FileType::JavaScriptXml => LanguageType::JavaScriptXml,
            FileType::TypeScript | FileType::TypeScriptDeclaration => LanguageType::TypeScript,
            FileType::TypeScriptXml => LanguageType::TypeScriptXml,
            // default to Destack for other file types
            _ => LanguageType::Destack,
        }
    }
}
