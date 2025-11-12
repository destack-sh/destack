use std::collections::HashMap;

use dyst_compiler::Compiler;
use dyst_dir as dir;
use dyst_source::{
    DiagnosticCollector, FileType, LanguageOptions, SmallVec, StringPool, Uri, smallvec,
};

use crate::{JavaScriptFormatOptions, TranspilerArtifact, TranspilerUnit};

/// The transpilation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TranspilerMode {
    /// Retain the original file structure.
    Retained,
    /// Combine all files.
    Combined,
}

/// The options for transpiling a Workspace.
#[derive(Debug, Default, Clone, Copy)]
pub struct TranspilerOptions {
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
    /// The language options.
    pub language: LanguageOptions,
    /// The options for transpiling.
    pub options: TranspilerOptions,

    /// The node tree of the compiled DIR.
    pub tree: &'a dir::NodeTree,
    /// The modules.
    pub modules: &'a dir::ModuleRegistry,
    /// The string pool.
    pub strings: &'a StringPool,

    /// The diagnostic collector.
    pub diagnostics: DiagnosticCollector,
    /// The transpiled modules (from the source modules).
    pub units: HashMap<Uri, TranspilerUnit>,
    /// The transpiled artifacts (from those units).
    pub artifacts: HashMap<Uri, TranspilerArtifact>,
}

impl<'a> Transpiler<'a> {
    /// Create a new Transpiler from a Compiler state.
    pub fn from_compiled(
        compiler: &'a Compiler<'_>,
        language: LanguageOptions,
        options: TranspilerOptions,
    ) -> Self {
        Self {
            language,
            options,
            tree: &compiler.tree,
            modules: &compiler.modules,
            strings: &compiler.strings,
            diagnostics: DiagnosticCollector::new(),
            units: HashMap::new(),
            artifacts: HashMap::new(),
        }
    }

    /// Get the transpiler artifacts for a given unit.
    pub fn get_artifacts_for_unit(&self, uri: Uri) -> Vec<&TranspilerArtifact> {
        let Some(unit) = self.units.get(&uri) else {
            return Vec::new();
        };
        unit.artifacts
            .iter()
            .filter_map(|uri| self.artifacts.get(uri))
            .collect()
    }

    /// Get the transpiler artifact of a certain type for a given unit.
    pub fn get_artifact_for_unit(&self, uri: Uri, ty: FileType) -> Option<&TranspilerArtifact> {
        let unit = self.units.get(&uri)?;
        unit.artifacts
            .iter()
            .filter_map(|uri| self.artifacts.get(uri))
            .find(|artifact| artifact.ty == ty)
    }
}
