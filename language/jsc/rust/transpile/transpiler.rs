use dyst_js::NodeTree;
use dyst_module::{Package, Workspace};
use dyst_session::Session;
use dyst_source::{LanguageOptions, StringPool};

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
#[derive(Debug, Clone)]
pub struct Transpiler<'s> {
    /// The workspace we're in.
    pub workspace: &'s Workspace,
    /// The session we're in.
    pub session: &'s Session,
    /// The package we're transpiling.
    pub package: &'s Package,
    /// The language options.
    pub language: LanguageOptions,

    /// The node tree of the compiled DIR.
    pub tree: NodeTree,
    /// The string pool.
    pub strings: StringPool,
    /// The options for compiling the Package.
    pub options: TranspilerOptions,
}
