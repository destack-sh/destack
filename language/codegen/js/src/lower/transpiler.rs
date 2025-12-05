use std::num::NonZero;
use std::sync::Arc;
use std::thread;

use dashmap::DashMap;
use destack_source::{DiagnosticCollector, DiagnosticOptions, Uri};
use destack_workspace::Program;
use smallvec::{SmallVec, smallvec};

use crate::{
    JavaScriptFormatOptions, TranspileDiagnostic, TranspileError, TranspileWarning,
    TranspilerArtifact, TranspilerUnit,
};

/// Get the default number of worker threads (available parallelism, or 1 if unknown).
pub fn default_workers() -> u16 {
    thread::available_parallelism()
        .unwrap_or(NonZero::new(1).unwrap())
        .get() as u16
}

/// The transpilation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TranspilerMode {
    /// Retain the original file structure.
    Retained,
    /// Combine all files.
    Combined,
}

/// The options for transpiling.
#[derive(Debug, Clone)]
pub struct TranspileOptions {
    /// The diagnostic options.
    pub diagnostic: DiagnosticOptions,
    /// The number of worker threads to use.
    pub workers: u16,
    /// The transpilation mode.
    pub mode: TranspilerMode,
    /// The target language.
    pub target: TranspileTarget,
    /// The ECMAScript level.
    pub es_version: EcmaScriptVersion,
    /// The TypeScript version.
    pub ts_version: TypeScriptVersion,
    /// The formatting options.
    pub formatting: JavaScriptFormatOptions,
}

impl Default for TranspileOptions {
    fn default() -> Self {
        Self {
            diagnostic: DiagnosticOptions::default(),
            workers: default_workers(),
            mode: TranspilerMode::Retained,
            target: TranspileTarget::TypeScript,
            es_version: EcmaScriptVersion::ES2022,
            ts_version: TypeScriptVersion::TS5_0,
            formatting: JavaScriptFormatOptions::default(),
        }
    }
}

/// The transpiler target for transpiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TranspileTarget {
    /// Plain JavaScript (`.js`).
    JavaScript,
    /// TypeScript (`.ts`).
    TypeScript,
    /// Plain JavaScript with TypeScript declarations (.js and .d.ts).
    JavaScriptWithTypeScriptDeclarations,
    /// All languages.
    All,
}

impl TranspileTarget {
    /// Get the language targets for transpiling.
    pub fn language_targets(&self) -> SmallVec<[TranspilerLanguage; 3]> {
        match self {
            TranspileTarget::JavaScript => smallvec![TranspilerLanguage::JavaScript],
            TranspileTarget::TypeScript => smallvec![TranspilerLanguage::TypeScript],
            TranspileTarget::JavaScriptWithTypeScriptDeclarations => smallvec![
                TranspilerLanguage::JavaScript,
                TranspilerLanguage::TypeScriptDeclaration,
            ],
            TranspileTarget::All => smallvec![
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

/// A transpiler for a Destack package containing related Destack sources.
/// nocheckin: plug Transpiler into Generate phase of Compiler? (per unit TranspileTask?)
#[derive(Debug)]
pub struct Transpiler {
    /// The program.
    pub program: Arc<Program>,
    /// The options for transpiling.
    pub options: TranspileOptions,
    /// The pending transpiler diagnostics.
    pub pending_diagnostics: DiagnosticCollector,
    /// The transpiled modules (from the source modules).
    pub units: DashMap<Uri, TranspilerUnit>,
    /// The transpiled artifacts (from those units).
    pub artifacts: DashMap<Uri, TranspilerArtifact>,
}

impl Transpiler {
    /// Create a new Transpiler from a Compiler state.
    pub fn new(program: Arc<Program>, options: TranspileOptions) -> Self {
        Self {
            program,
            options,
            pending_diagnostics: DiagnosticCollector::new(),
            units: DashMap::new(),
            artifacts: DashMap::new(),
        }
    }

    /// Add an error to the transpiler.
    pub fn error(&self, error: TranspileError) {
        let diagnostic: TranspileDiagnostic = error.into();
        let diagnostic = diagnostic.to_diagnostic(self.program.as_ref());
        self.pending_diagnostics.insert(diagnostic);
    }

    /// Add a warning to the transpiler.
    pub fn warning(&self, warning: TranspileWarning) {
        let diagnostic: TranspileDiagnostic = warning.into();
        let diagnostic = diagnostic.to_diagnostic(self.program.as_ref());
        self.pending_diagnostics.insert(diagnostic);
    }

    /// Flush pending diagnostics into the program.
    pub fn flush_diagnostics(&self) {
        self.program
            .diagnostics
            .take_from(&self.pending_diagnostics);
    }
}
