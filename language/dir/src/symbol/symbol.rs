use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{ExportKind, GlobalNodeIdAny, LocalScope, Mutability, NodeType, StaticKey, StringId};

/// A bindable item or local in a scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Symbol {
    /// The scope lookup role of the symbol.
    pub role: SymbolRole,
    /// The declaration form of the symbol.
    pub form: SymbolForm,
    /// The mutability for value bindings when known.
    pub binding_mutability: Option<Mutability>,

    /// Where this symbol was introduced.
    pub origin: SymbolOrigin,
    /// The key of the symbol.
    pub key: Option<StaticKey>,
    /// The scope that introduces the symbol.
    pub scope: LocalScope,

    /// The export kind of the symbol.
    pub export_kind: Option<ExportKind>,
    /// The declaration node that introduced this symbol.
    pub declaration: Option<GlobalNodeIdAny>,
}

impl Symbol {
    /// Get the name of the symbol.
    #[inline]
    pub fn name(&self) -> Option<StringId> {
        match self.key {
            Some(StaticKey::Name(name)) => Some(name),
            _ => None,
        }
    }

    /// Check whether this symbol is a generic parameter.
    pub fn is_generic_parameter(&self) -> bool {
        self.declaration
            .is_some_and(|declaration| declaration.local_id.ty == NodeType::GenericParameter)
    }
}

/// The space of a symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SymbolSpace {
    /// The type space.
    Type,
    /// The value space.
    Value,
    /// The label space.
    Label,
}

impl SymbolSpace {
    /// Check if this space conflicts with another space.
    #[inline]
    pub fn conflicts_with(self, other: Self) -> bool {
        match (self, other) {
            // labels only conflict with labels
            (Self::Label, Self::Label) => true,
            (Self::Label, _) | (_, Self::Label) => false,
            // destack keeps type and value names in one declaration namespace
            (Self::Type, _) | (Self::Value, _) => true,
        }
    }
}

/// The scope lookup role of a symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SymbolRole {
    /// Namespace symbol with an owned scope.
    Namespace,
    /// Item symbol that must be unique within its scope.
    Item,
    /// Local symbol that may be shadowed within its scope.
    Local,
}

/// Where a symbol originated in the source.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum SymbolOrigin {
    /// Declaration in module scope.
    #[default]
    Module,
    /// Declaration inside a `global` block.
    Global,
}

impl SymbolOrigin {
    /// Check if this symbol originates from a global block.
    #[inline]
    pub fn is_global(self) -> bool {
        matches!(self, SymbolOrigin::Global)
    }
}

/// The declaration form of a symbol.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum SymbolForm {
    /// Plain variable-like value symbol without a more specific form.
    #[default]
    Variable,
    /// Imported dependency binding before target resolution.
    Import,
    /// Class symbol.
    Class,
    /// Struct symbol.
    Struct,
    /// Interface symbol.
    Interface,
    /// Nominal interface symbol.
    NewtypeInterface,
    /// Enum symbol.
    Enum,
    /// Function symbol.
    Function,
    /// Label symbol.
    Label,
    /// Extension symbol.
    Extension,
    /// Transparent type alias symbol.
    TypeAlias,
    /// Nominal newtype symbol.
    Newtype,
}

impl SymbolForm {
    /// Check if this is an interface.
    #[inline]
    pub fn is_interface(self) -> bool {
        matches!(self, Self::Interface | Self::NewtypeInterface)
    }

    /// Check whether this form satisfies one requested form.
    pub fn matches_form(self, form: SymbolForm) -> bool {
        if form == Self::Interface {
            self.is_interface()
        } else {
            self == form
        }
    }

    /// Return the symbol space normally introduced by this symbol form.
    pub fn symbol_space(self) -> SymbolSpace {
        match self {
            Self::Interface | Self::NewtypeInterface | Self::TypeAlias => SymbolSpace::Type,
            Self::Label => SymbolSpace::Label,
            Self::Class
            | Self::Enum
            | Self::Extension
            | Self::Function
            | Self::Import
            | Self::Newtype
            | Self::Struct
            | Self::Variable => SymbolSpace::Value,
        }
    }

    /// Check whether this symbol form is visible in one lookup space.
    pub fn is_visible_in(self, space: SymbolSpace) -> bool {
        match space {
            SymbolSpace::Type => matches!(
                self,
                Self::Class
                    | Self::Enum
                    | Self::Extension
                    | Self::Import
                    | Self::Interface
                    | Self::Newtype
                    | Self::NewtypeInterface
                    | Self::Struct
                    | Self::TypeAlias
            ),
            SymbolSpace::Value => matches!(
                self,
                Self::Class
                    | Self::Enum
                    | Self::Function
                    | Self::Import
                    | Self::Newtype
                    | Self::Struct
                    | Self::Variable
            ),
            SymbolSpace::Label => self == Self::Label,
        }
    }
}

/// Unique identifier for Symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalSymbolId {
    /// The numeric id.
    pub id: u32,
}

impl LocalSymbolId {
    /// Create a new symbol id.
    pub fn new(id: u32) -> Self {
        Self { id }
    }

    /// Turn into a GlobalSymbolId.
    pub fn into_global(self, module_id: ModuleId) -> GlobalSymbolId {
        GlobalSymbolId {
            module_id,
            local_id: self,
        }
    }
}

/// Global symbol id across modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GlobalSymbolId {
    /// The module id of the global symbol.
    pub module_id: ModuleId,
    /// The local id of the global symbol.
    pub local_id: LocalSymbolId,
}

impl GlobalSymbolId {
    /// Create a new global symbol id.
    pub fn new(module_id: ModuleId, local_id: LocalSymbolId) -> Self {
        Self {
            module_id,
            local_id,
        }
    }

    /// Turn into a LocalSymbolId.
    #[inline]
    pub fn into_local(self) -> LocalSymbolId {
        self.local_id
    }
}

impl From<GlobalSymbolId> for LocalSymbolId {
    fn from(id: GlobalSymbolId) -> Self {
        id.local_id
    }
}
