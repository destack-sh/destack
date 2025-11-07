use std::collections::HashMap;

use dyst_compiler::Compiler;
use dyst_javascript_ast::NodeTree;
use dyst_source::{DiagnosticCollector, FileId, LanguageOptions, StringPool};

use crate::TranspilerArtifact;

/// The options for transpiling a Workspace.
#[derive(Debug, Default, Clone)]
pub struct TranspilerOptions {
    /// The target language.
    pub target: LanguageTarget = LanguageTarget::TypeScript,
    /// The ECMAScript level.
    pub es_version: EcmaScriptVersion = EcmaScriptVersion::ES2022,
    /// The TypeScript version.
    pub ts_version: TypeScriptVersion = TypeScriptVersion::TS5_0,
}

/// The target language for transpiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageTarget {
    /// Plain JavaScript (like `.js`).
    JavaScript,
    /// JavaScript XML (like `.jsx`).
    JavaScriptXml,
    /// TypeScript (like `.ts`).
    TypeScript,
    /// Typescript XML (like `.tsx`).
    TypeScriptXml,
    /// TypeScript declarations (like `.d.ts`).
    TypeScriptDeclaration,
}

impl LanguageTarget {
    /// Whether this includes type annotations.
    #[inline]
    pub fn includes_type_annotations(&self) -> bool {
        matches!(
            self,
            LanguageTarget::TypeScript
                | LanguageTarget::TypeScriptXml
                | LanguageTarget::TypeScriptDeclaration
        )
    }

    /// Whether this includes XML.
    #[inline]
    pub fn includes_xml(&self) -> bool {
        matches!(
            self,
            LanguageTarget::JavaScriptXml | LanguageTarget::TypeScriptXml
        )
    }
}

/// The ECMAScript level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EcmaScriptVersion {
    ES2022,
}

/// The TypeScript version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeScriptVersion {
    TS5_0,
}

/// A transpiler for a Dyst package containing related Dyst sources.
#[derive(Debug)]
pub struct Transpiler<'a> {
    /// The language options.
    pub language: LanguageOptions,
    /// The options for compiling the Package.
    pub options: TranspilerOptions,

    /// The node tree of the compiled DIR.
    pub tree: &'a NodeTree,
    /// The string pool.
    pub strings: &'a StringPool,

    /// The diagnostic collector.
    pub diagnostics: &'a mut DiagnosticCollector,
    /// The artifacts of the transpiled files.
    pub artifacts: HashMap<FileId, TranspilerArtifact>,
}

impl<'a> Transpiler<'a> {
    /// Create a new Transpiler from a Compiler state.
    pub fn from_compiler(compiler: &'a Compiler, options: TranspilerOptions) -> Self {
        todo!("from_compiler({compiler:?})")
    }
}
