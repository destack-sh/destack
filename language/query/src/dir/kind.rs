use destack_dir::{Declaration, DependencyKind, Member};
use serde::{Deserialize, Serialize};

/// Kind of a symbol in navigation queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolKind {
    /// A file symbol.
    File,
    /// A module symbol.
    Module,
    /// A namespace symbol.
    Namespace,
    /// A package symbol.
    Package,
    /// A class symbol.
    Class,
    /// A method symbol.
    Method,
    /// A property symbol.
    Property,
    /// A field symbol.
    Field,
    /// A constructor symbol.
    Constructor,
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
    /// A string symbol.
    String,
    /// A number symbol.
    Number,
    /// A boolean symbol.
    Boolean,
    /// An array symbol.
    Array,
    /// An object symbol.
    Object,
    /// A key symbol.
    Key,
    /// A null symbol.
    Null,
    /// An enum member symbol.
    EnumMember,
    /// A struct symbol.
    Struct,
    /// An event symbol.
    Event,
    /// An operator symbol.
    Operator,
    /// A type parameter symbol.
    TypeParameter,
}

/// Map a declaration to its symbol kind.
pub(crate) fn declaration_symbol_kind(declaration: &Declaration) -> SymbolKind {
    match declaration {
        Declaration::Global(_) => SymbolKind::Namespace,
        Declaration::Function(_) => SymbolKind::Function,
        Declaration::Struct(_) => SymbolKind::Struct,
        Declaration::Class(_) => SymbolKind::Class,
        Declaration::Interface(_) => SymbolKind::Interface,
        Declaration::Enum(_) => SymbolKind::Enum,
        Declaration::Namespace(_) => SymbolKind::Namespace,
        Declaration::Type(_) => SymbolKind::TypeParameter,
        Declaration::ImportAlias(declaration) => match declaration.kind {
            DependencyKind::Type => SymbolKind::TypeParameter,
            DependencyKind::Value => SymbolKind::Variable,
        },
        Declaration::Extension(_) => SymbolKind::Class,
    }
}

/// Map a member to its symbol kind.
pub(crate) fn member_symbol_kind(member: &Member) -> Option<SymbolKind> {
    match member {
        Member::Type { .. } => Some(SymbolKind::TypeParameter),
        Member::ComptimeConst { .. } => Some(SymbolKind::Constant),
        Member::Field { .. } => Some(SymbolKind::Field),
        Member::Method { .. } => Some(SymbolKind::Method),
        Member::Embed { .. } => None,
        Member::StaticBlock { .. } => None,
        Member::ComptimeBlock { .. } => None,
        Member::Error { .. } => None,
    }
}
