use std::path::PathBuf;

use destack_dir::{GlobalSymbolId, LocalExtensionId, LocalSymbolId, SymbolSpace, SymbolType};
use destack_source::{FileId, ModuleId, Span};

/// One importable exported symbol entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImportIndexEntry {
    /// The exported name.
    pub name: String,
    /// The symbol type.
    pub kind: SymbolType,
    /// The symbol space.
    pub space: SymbolSpace,
    /// The exporting module id.
    pub module_id: ModuleId,
    /// The local symbol id within the exporting module.
    pub local_id: LocalSymbolId,
    /// The canonical import path when available.
    pub module_path: Option<String>,
}

/// The workspace index kind for named symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolIndexKind {
    /// A namespace symbol.
    Namespace,
    /// A class symbol.
    Class,
    /// A method symbol.
    Method,
    /// A field symbol.
    Field,
    /// An enum symbol.
    Enum,
    /// An interface symbol.
    Interface,
    /// A function symbol.
    Function,
    /// A variable symbol.
    Variable,
    /// A constant symbol.
    Constant,
    /// An enum member symbol.
    EnumMember,
    /// A struct symbol.
    Struct,
    /// A type parameter symbol.
    TypeParameter,
}

/// One searchable workspace symbol entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SymbolIndexEntry {
    /// The symbol name.
    pub name: String,
    /// The symbol kind.
    pub kind: SymbolIndexKind,
    /// The owning module id.
    pub module_id: ModuleId,
    /// The source file id.
    pub file_id: FileId,
    /// The symbol range.
    pub range: Span,
    /// The optional container name.
    pub container_name: Option<String>,
}

/// The workspace index relation kind for nominal hierarchies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NominalRelationKind {
    /// A direct `extends` relationship.
    Extends,
    /// A direct `implements` relationship.
    Implements,
    /// A direct `embeds` relationship.
    Embeds,
}

/// One nominal hierarchy relation entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NominalIndexEntry {
    /// The source symbol.
    pub source_symbol: GlobalSymbolId,
    /// The target symbol.
    pub target_symbol: GlobalSymbolId,
    /// The direct relation kind.
    pub relation: NominalRelationKind,
}

/// One extension lookup entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExtensionIndexEntry {
    /// The owning module id.
    pub module_id: ModuleId,
    /// The local extension id.
    pub extension_id: LocalExtensionId,
    /// The extension target symbol.
    pub target_symbol: GlobalSymbolId,
}

/// One indexed call edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CallIndexEntry {
    /// The module that contains the call site.
    pub module_id: ModuleId,
    /// The caller function symbol when present.
    pub caller_symbol: Option<GlobalSymbolId>,
    /// The callee function symbol.
    pub callee_symbol: GlobalSymbolId,
    /// The call expression span.
    pub span: Span,
}

/// One indexed module specifier entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpecifierIndexEntry {
    /// The module that contains the specifier.
    pub module_id: ModuleId,
    /// The file that contains the specifier.
    pub file_id: FileId,
    /// The ast expression node id.
    pub ast_node_id: u32,
    /// The raw specifier text.
    pub specifier: String,
    /// The resolved target module when available.
    pub target_module_id: Option<ModuleId>,
    /// The resolved target path when available.
    pub target_path: Option<PathBuf>,
}
