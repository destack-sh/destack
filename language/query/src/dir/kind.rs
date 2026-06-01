use destack_dir as dir;
use destack_qir::{MemberKind as MemberEntryKind, SymbolKind as SymbolEntryKind};
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

impl From<SymbolEntryKind> for SymbolKind {
    /// Convert one indexed declaration symbol kind to a public symbol kind.
    fn from(kind: SymbolEntryKind) -> Self {
        match kind {
            SymbolEntryKind::Namespace => SymbolKind::Namespace,
            SymbolEntryKind::Class => SymbolKind::Class,
            SymbolEntryKind::Enum => SymbolKind::Enum,
            SymbolEntryKind::Interface => SymbolKind::Interface,
            SymbolEntryKind::Function => SymbolKind::Function,
            SymbolEntryKind::Variable => SymbolKind::Variable,
            SymbolEntryKind::Constant => SymbolKind::Constant,
            SymbolEntryKind::Struct => SymbolKind::Struct,
            SymbolEntryKind::TypeParameter => SymbolKind::TypeParameter,
        }
    }
}

impl From<MemberEntryKind> for SymbolKind {
    /// Convert one indexed member kind to a public symbol kind.
    fn from(kind: MemberEntryKind) -> Self {
        match kind {
            MemberEntryKind::Field => SymbolKind::Field,
            MemberEntryKind::Method => SymbolKind::Method,
            MemberEntryKind::AssociatedType => SymbolKind::TypeParameter,
            MemberEntryKind::AssociatedConst => SymbolKind::Constant,
            MemberEntryKind::Variant => SymbolKind::EnumMember,
        }
    }
}

impl SymbolKind {
    /// Map one declaration to its public symbol kind.
    pub(crate) fn from_declaration(declaration: &dir::Declaration) -> Self {
        match declaration {
            dir::Declaration::Global(_) => Self::Namespace,
            dir::Declaration::Module(_) => Self::Namespace,
            dir::Declaration::Function(_) => Self::Function,
            dir::Declaration::Struct(_) => Self::Struct,
            dir::Declaration::Class(_) => Self::Class,
            dir::Declaration::Interface(_) => Self::Interface,
            dir::Declaration::Enum(_) => Self::Enum,
            dir::Declaration::Type(_) => Self::TypeParameter,
            dir::Declaration::Extension(_) => Self::Class,
        }
    }

    /// Map one declaration member to its public symbol kind.
    pub(crate) fn from_member(member: &dir::Member) -> Option<Self> {
        match member {
            dir::Member::AssociatedType { .. } => Some(Self::TypeParameter),
            dir::Member::AssociatedConst { .. } => Some(Self::Constant),
            dir::Member::Field { .. } => Some(Self::Field),
            dir::Member::Method { .. } => Some(Self::Method),
            dir::Member::StaticBlock { .. } => None,
            dir::Member::ComptimeBlock { .. } => None,
            dir::Member::Error => None,
        }
    }

    /// Map one type member to its public symbol kind.
    pub(crate) fn from_type_member(member: &dir::TypeMember) -> Option<Self> {
        match member {
            dir::TypeMember::AssociatedType { .. } => Some(Self::TypeParameter),
            dir::TypeMember::AssociatedConst { .. } => Some(Self::Constant),
            dir::TypeMember::Field { .. } => Some(Self::Field),
            dir::TypeMember::Method { .. } => Some(Self::Method),
            dir::TypeMember::CallSignature { .. } => Some(Self::Method),
            dir::TypeMember::ConstructSignature { .. } => Some(Self::Method),
            dir::TypeMember::IndexSignature { .. } => None,
            dir::TypeMember::Error => None,
        }
    }
}
