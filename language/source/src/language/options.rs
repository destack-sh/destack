use crate::{
    FileType, FormatterOptions, IndentStyle, LanguageFeature, LanguageFeatureSet, LineEnding,
};

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

    /// Whether this is a compatibility mode (JS/TS, not Destack).
    #[inline]
    pub fn is_compatibility_mode(&self) -> bool {
        !self.is_destack()
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

/// The language version.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub enum LanguageVersion {
    /// The first version of the language.
    #[default]
    V1,
}

/// Options for working with the Destack language.
#[derive(Debug, Copy, Clone, Default)]
pub struct LanguageOptions {
    /// The version of the language.
    pub version: LanguageVersion = LanguageVersion::V1,
    /// The source language type (determines compatibility behavior).
    pub ty: LanguageType = LanguageType::Destack,
    /// The enabled language features (for post-parse feature gating).
    pub features: LanguageFeatureSet = LanguageFeatureSet::all(),
    /// The formatting options.
    pub formatting: FormatterOptions,
}

impl LanguageOptions {
    /// Whether the language type is Destack-compatible.
    #[inline]
    pub fn is_destack_compatible(&self) -> bool {
        self.ty.is_destack()
    }

    /// Whether the language type is JavaScript-compatible.
    #[inline]
    pub fn is_javascript_compatible(&self) -> bool {
        self.ty.is_javascript()
    }

    /// Whether the language type is TypeScript-compatible.
    #[inline]
    pub fn is_typescript_compatible(&self) -> bool {
        self.ty.is_typescript()
    }

    /// Whether the language supports JSX/tree literal syntax.
    #[inline]
    pub fn supports_tree_literal(&self) -> bool {
        self.ty.supports_jsx()
    }

    /// Set the language version.
    pub fn with_version(mut self, version: LanguageVersion) -> Self {
        self.version = version;
        self
    }

    /// Set the language type.
    pub fn with_type(mut self, language_type: LanguageType) -> Self {
        self.ty = language_type;
        self
    }

    /// Set the formatting options.
    pub fn with_formatting(mut self, formatting: FormatterOptions) -> Self {
        self.formatting = formatting;
        self
    }

    /// Set the line ending type.
    pub fn with_line_ending(mut self, line_ending: LineEnding) -> Self {
        self.formatting.line_ending = line_ending;
        self
    }

    /// Set the indent style.
    pub fn with_indent_style(mut self, indent_style: IndentStyle) -> Self {
        self.formatting.indent_style = indent_style;
        self
    }

    /// Set the indent width.
    pub fn with_indent_width(mut self, indent_width: u8) -> Self {
        self.formatting.indent_width = indent_width;
        self
    }

    /// Set the line width.
    pub fn with_line_width(mut self, line_width: u8) -> Self {
        self.formatting.line_width = line_width;
        self
    }

    /// Set the enabled features.
    pub fn with_features(mut self, features: LanguageFeatureSet) -> Self {
        self.features = features;
        self
    }

    /// Enable a specific feature.
    pub fn with_feature(mut self, feature: LanguageFeature) -> Self {
        self.features.enable(feature);
        self
    }

    /// Disable a specific feature.
    pub fn without_feature(mut self, feature: LanguageFeature) -> Self {
        self.features.disable(feature);
        self
    }

    /// Check if a feature is enabled.
    #[inline]
    pub fn is_feature_enabled(&self, feature: LanguageFeature) -> bool {
        self.features.is_enabled(feature)
    }
}
