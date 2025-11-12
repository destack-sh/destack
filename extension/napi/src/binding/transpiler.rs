use napi_derive::napi;

use super::source::{IndentStyle, LineEnding};

/// The transpilation mode.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranspilerMode {
    /// Retain the original file structure.
    Retained,
    /// Combine all files.
    Combined,
}

impl Default for TranspilerMode {
    fn default() -> Self {
        Self::Retained
    }
}

impl From<TranspilerMode> for dyst_javascript_transpiler::TranspilerMode {
    fn from(mode: TranspilerMode) -> Self {
        match mode {
            TranspilerMode::Retained => dyst_javascript_transpiler::TranspilerMode::Retained,
            TranspilerMode::Combined => dyst_javascript_transpiler::TranspilerMode::Combined,
        }
    }
}

/// The target language for transpiling.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranspilerTarget {
    /// Plain JavaScript (`.js`).
    JavaScript,
    /// TypeScript (`.ts`).
    TypeScript,
    /// Plain JavaScript with TypeScript declarations (.js and .d.ts).
    JavaScriptWithTypeScriptDeclarations,
}

impl Default for TranspilerTarget {
    fn default() -> Self {
        Self::TypeScript
    }
}

impl From<TranspilerTarget> for dyst_javascript_transpiler::TranspilerTarget {
    fn from(target: TranspilerTarget) -> Self {
        match target {
            TranspilerTarget::JavaScript => {
                dyst_javascript_transpiler::TranspilerTarget::JavaScript
            }
            TranspilerTarget::TypeScript => {
                dyst_javascript_transpiler::TranspilerTarget::TypeScript
            }
            TranspilerTarget::JavaScriptWithTypeScriptDeclarations => {
                dyst_javascript_transpiler::TranspilerTarget::JavaScriptWithTypeScriptDeclarations
            }
        }
    }
}

/// The target language for transpiling.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranspilerLanguage {
    /// Plain JavaScript (like `.js`).
    JavaScript,
    /// TypeScript (like `.ts`).
    TypeScript,
    /// TypeScript declarations (like `.d.ts`).
    TypeScriptDeclaration,
}

impl Default for TranspilerLanguage {
    fn default() -> Self {
        Self::TypeScript
    }
}

impl From<TranspilerLanguage> for dyst_javascript_transpiler::TranspilerLanguage {
    fn from(language: TranspilerLanguage) -> Self {
        match language {
            TranspilerLanguage::JavaScript => {
                dyst_javascript_transpiler::TranspilerLanguage::JavaScript
            }
            TranspilerLanguage::TypeScript => {
                dyst_javascript_transpiler::TranspilerLanguage::TypeScript
            }
            TranspilerLanguage::TypeScriptDeclaration => {
                dyst_javascript_transpiler::TranspilerLanguage::TypeScriptDeclaration
            }
        }
    }
}

/// The ECMAScript level.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcmaScriptVersion {
    /// ECMAScript 2022.
    ES2022,
}

impl Default for EcmaScriptVersion {
    fn default() -> Self {
        Self::ES2022
    }
}

impl From<EcmaScriptVersion> for dyst_javascript_transpiler::EcmaScriptVersion {
    fn from(version: EcmaScriptVersion) -> Self {
        match version {
            EcmaScriptVersion::ES2022 => dyst_javascript_transpiler::EcmaScriptVersion::ES2022,
        }
    }
}

/// The TypeScript version.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeScriptVersion {
    /// TypeScript 5.0.
    TS5_0,
}

impl Default for TypeScriptVersion {
    fn default() -> Self {
        Self::TS5_0
    }
}

impl From<TypeScriptVersion> for dyst_javascript_transpiler::TypeScriptVersion {
    fn from(version: TypeScriptVersion) -> Self {
        match version {
            TypeScriptVersion::TS5_0 => dyst_javascript_transpiler::TypeScriptVersion::TS5_0,
        }
    }
}

/// The formatting mode.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatMode {
    /// Pretty.
    Pretty,
    /// Minimal.
    Minimal,
}

impl Default for FormatMode {
    fn default() -> Self {
        Self::Pretty
    }
}

impl From<FormatMode> for dyst_javascript_transpiler::FormatMode {
    fn from(mode: FormatMode) -> Self {
        match mode {
            FormatMode::Pretty => dyst_javascript_transpiler::FormatMode::Pretty,
            FormatMode::Minimal => dyst_javascript_transpiler::FormatMode::Minimal,
        }
    }
}

/// The JavaScript format options.
#[napi(object)]
#[derive(Debug, Clone, Copy)]
pub struct FormatOptions {
    /// The formatting mode.
    pub mode: FormatMode,
    /// The language target.
    pub language: TranspilerLanguage,
    /// The type of line ending to apply to the printed input.
    pub line_ending: LineEnding,
    /// The indent style.
    pub indent_style: IndentStyle,
    /// Spaces per indent.
    pub indent_width: u8,
    /// Maximum line length (best effort).
    pub line_width: u8,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            mode: FormatMode::Pretty,
            language: TranspilerLanguage::JavaScript,
            line_ending: LineEnding::LineFeed,
            indent_style: IndentStyle::Space,
            indent_width: 4,
            line_width: 100,
        }
    }
}

impl From<FormatOptions> for dyst_javascript_transpiler::JavaScriptFormatOptions {
    fn from(options: FormatOptions) -> Self {
        Self {
            mode: options.mode.into(),
            language: options.language.into(),
            line_ending: options.line_ending.into(),
            indent_style: options.indent_style.into(),
            indent_width: options.indent_width,
            line_width: options.line_width,
        }
    }
}

/// The transpilation options.
#[napi(object)]
#[derive(Debug, Clone, Copy)]
pub struct TranspilerOptions {
    /// The transpilation mode.
    pub mode: TranspilerMode,
    /// The target language.
    pub target: TranspilerTarget,
    /// The ECMAScript level.
    pub es_version: EcmaScriptVersion,
    /// The TypeScript version.
    pub ts_version: TypeScriptVersion,
    /// The formatting options.
    pub formatting: FormatOptions,
}

impl Default for TranspilerOptions {
    fn default() -> Self {
        Self {
            mode: TranspilerMode::Retained,
            target: TranspilerTarget::TypeScript,
            es_version: EcmaScriptVersion::ES2022,
            ts_version: TypeScriptVersion::TS5_0,
            formatting: FormatOptions::default(),
        }
    }
}

impl From<TranspilerOptions> for dyst_javascript_transpiler::TranspilerOptions {
    fn from(options: TranspilerOptions) -> Self {
        Self {
            mode: options.mode.into(),
            target: options.target.into(),
            es_version: options.es_version.into(),
            ts_version: options.ts_version.into(),
            formatting: options.formatting.into(),
        }
    }
}

/// Get the default transpiler options.
#[napi(js_name = "defaultTranspilerOptions")]
pub fn default_transpiler_options() -> TranspilerOptions {
    TranspilerOptions::default()
}
