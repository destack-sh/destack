use crate::{FormattingOptions, IndentStyle, LanguageFeature, LanguageFeatureSet, LineEnding};

/// The mode we're working in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LanguageMode {
    /// Lenient mode with relaxed checking, conversion, cloning, boxing and more.
    #[default]
    Lenient,
    /// Strict mode with explicit context, defaults, typing, behavior and more.
    Strict,
}

/// The type of language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageType {
    /// The Destack language (like `.ds`, `.dst`).
    Destack,
    /// The Destack data language (like `.d.ds`).
    DestackDeclaration,
    /// JavaScript (like `.js`).
    JavaScript,
    /// JavaScript XML (like `.jsx`).
    JavaScriptXml,
    /// TypeScript (like `.ts`).
    TypeScript,
    /// TypeScript XML (like `.tsx`).
    TypeScriptXml,
}

/// The language compatibility mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageCompatibility {
    /// The JavaScript compatibility mode (like `.js`).
    JavaScript,
    /// The JavaScript XML compatibility mode (like `.jsx`).
    JavaScriptXml,
    /// The TypeScript compatibility mode (like `.ts`).
    TypeScript,
    /// The TypeScript XML compatibility mode (like `.tsx`).
    TypeScriptXml,
}

impl LanguageCompatibility {
    /// Whether the language compatibility is TypeScript-related.
    #[inline]
    pub fn is_typescript(&self) -> bool {
        matches!(
            self,
            LanguageCompatibility::TypeScript | LanguageCompatibility::TypeScriptXml
        )
    }

    /// Whether the language compatibility is JavaScript-related.
    #[inline]
    pub fn is_javascript(&self) -> bool {
        matches!(
            self,
            LanguageCompatibility::JavaScript | LanguageCompatibility::JavaScriptXml
        )
    }

    /// Whether the language compatibility is XML-related.
    #[inline]
    pub fn supports_tree_literal(&self) -> bool {
        matches!(
            self,
            LanguageCompatibility::JavaScriptXml | LanguageCompatibility::TypeScriptXml
        )
    }
}

/// The language version.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum LanguageVersion {
    /// The first version of the language.
    V1,
}

/// The options for working with the Destack language.
#[derive(Debug, Copy, Clone, Default)]
pub struct LanguageOptions {
    /// The version of the language.
    pub version: LanguageVersion = LanguageVersion::V1,
    /// The mode we're operating Destack in.
    pub mode: LanguageMode = LanguageMode::Lenient,
    /// The compatibility mode.
    pub compatibility: Option<LanguageCompatibility> = None,
    /// The enabled language features.
    pub features: LanguageFeatureSet = LanguageFeatureSet::all(),
    /// The formatting options.
    pub formatting: FormattingOptions = FormattingOptions::DEFAULT,
}

impl LanguageOptions {
    /// Whether the language compatibility is XML-related.
    #[inline]
    pub fn is_compatible_with_tree_literal(&self) -> bool {
        self.compatibility
            .is_some_and(|compatibility| compatibility.supports_tree_literal())
    }

    /// Whether the language compatibility is JavaScript-related.
    #[inline]
    pub fn is_compatible_with_javascript(&self) -> bool {
        self.compatibility
            .is_some_and(|compatibility| compatibility.is_javascript())
    }

    /// Whether the language compatibility is TypeScript-related.
    #[inline]
    pub fn is_compatible_with_typescript(&self) -> bool {
        self.compatibility
            .is_some_and(|compatibility| compatibility.is_typescript())
    }

    /// Whether we support XML-related syntax.
    #[inline]
    pub fn supports_tree_literal(&self) -> bool {
        self.compatibility.is_none() || self.is_compatible_with_tree_literal()
    }

    /// Whether we support standalone maybe operator (like `x?`).
    #[inline]
    pub fn supports_standalone_maybe(&self) -> bool {
        self.compatibility.is_none()
    }

    /// Set the language version.
    pub fn with_version(mut self, version: LanguageVersion) -> Self {
        self.version = version;
        self
    }

    /// Set the language mode.
    pub fn with_mode(mut self, mode: LanguageMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set the language compatibility.
    pub fn with_compatibility(mut self, compatibility: LanguageCompatibility) -> Self {
        self.compatibility = Some(compatibility);
        self
    }

    /// Set the language compatibility.
    pub fn without_compatibility(mut self) -> Self {
        self.compatibility = None;
        self
    }

    /// Set the formatting options.
    pub fn with_formatting(mut self, formatting: FormattingOptions) -> Self {
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
