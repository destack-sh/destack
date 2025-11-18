use std::collections::HashMap;

use dyst_dir as dir;
use dyst_source::{DiagnosticOptions, SmallVec, Uri, smallvec};
use parking_lot::RwLock;

use crate::{JavaScriptFormatOptions, TranspilerArtifact, TranspilerUnit};

/// The transpilation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TranspilerMode {
    /// Retain the original file structure.
    Retained,
    /// Combine all files.
    Combined,
}

/// The options for transpiling.
#[derive(Debug, Default, Clone)]
pub struct TranspileOptions {
    /// The diagnostic options.
    pub diagnostic: DiagnosticOptions,
    /// The transpilation mode.
    pub mode: TranspilerMode = TranspilerMode::Retained,
    /// The target language.
    pub target: TranspilerTarget = TranspilerTarget::TypeScript,
    /// The ECMAScript level.
    pub es_version: EcmaScriptVersion = EcmaScriptVersion::ES2022,
    /// The TypeScript version.
    pub ts_version: TypeScriptVersion = TypeScriptVersion::TS5_0,
    /// The formatting options.
    pub formatting: JavaScriptFormatOptions,
}

/// The transpiler target for transpiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TranspilerTarget {
    /// Plain JavaScript (`.js`).
    JavaScript,
    /// TypeScript (`.ts`).
    TypeScript,
    /// Plain JavaScript with TypeScript declarations (.js and .d.ts).
    JavaScriptWithTypeScriptDeclarations,
    /// All languages.
    All,
}

impl TranspilerTarget {
    /// Get the language targets for transpiling.
    pub fn language_targets(&self) -> SmallVec<TranspilerLanguage, 3> {
        match self {
            TranspilerTarget::JavaScript => smallvec![TranspilerLanguage::JavaScript],
            TranspilerTarget::TypeScript => smallvec![TranspilerLanguage::TypeScript],
            TranspilerTarget::JavaScriptWithTypeScriptDeclarations => smallvec![
                TranspilerLanguage::JavaScript,
                TranspilerLanguage::TypeScriptDeclaration,
            ],
            TranspilerTarget::All => smallvec![
                TranspilerLanguage::JavaScript,
                TranspilerLanguage::TypeScript,
                TranspilerLanguage::TypeScriptDeclaration,
            ],
        }
    }
}

/// The target language for transpiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TranspilerLanguage {
    /// Plain JavaScript (like `.js`).
    JavaScript,
    /// TypeScript (like `.ts`).
    TypeScript,
    /// TypeScript declarations (like `.d.ts`).
    TypeScriptDeclaration,
}

/// The ECMAScript level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EcmaScriptVersion {
    /// ECMAScript 2022.
    ES2022,
}

/// The TypeScript version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeScriptVersion {
    /// TypeScript 5.0.
    TS5_0,
}

/// A transpiler for a Dyst package containing related Dyst sources.
#[derive(Debug)]
pub struct Transpiler<'a> {
    /// The session.
    pub session: &'a dir::Session<'a>,
    /// The options for transpiling.
    pub options: TranspileOptions,
    /// The transpiled modules (from the source modules).
    pub units: RwLock<HashMap<Uri, TranspilerUnit>>,
    /// The transpiled artifacts (from those units).
    pub artifacts: RwLock<HashMap<Uri, TranspilerArtifact>>,
}

impl<'a> Transpiler<'a> {
    /// Create a new Transpiler from a Compiler state.
    pub fn new(session: &'a dir::Session<'a>, options: TranspileOptions) -> Self {
        Self {
            session,
            options,
            units: RwLock::new(HashMap::new()),
            artifacts: RwLock::new(HashMap::new()),
        }
    }
}
