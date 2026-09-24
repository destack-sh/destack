use destack_serde::Reflect;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::{ExportKind, GlobalNodeIdAny, LocalScope, Mutability, StaticKey, StringId};

/// A bindable item or local in a scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Symbol {
    /// The scope lookup role of the symbol.
    pub role: SymbolRole,
    /// The declaration kind of the symbol.
    pub kind: SymbolKind,
    /// The lookup visibility of this symbol.
    pub visibility: SymbolVisibility,
    /// The mutability for value bindings when known.
    pub binding_mutability: Option<Mutability>,
    /// Whether the value binding lives in shared storage.
    pub is_shared: bool,

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
        matches!(
            self.kind,
            SymbolKind::GenericTypeParameter
                | SymbolKind::GenericConstParameter
                | SymbolKind::GenericLifetimeParameter
        )
    }
}

/// Lexical owner path for one symbol.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SymbolPath {
    /// The owner symbols followed by the leaf symbol.
    symbols: Vec<LocalSymbolId>,
}

impl SymbolPath {
    /// Create a non-empty symbol path.
    pub fn new(symbols: Vec<LocalSymbolId>) -> Self {
        assert!(!symbols.is_empty(), "symbol path cannot be empty");

        Self { symbols }
    }

    /// Return path symbols from outermost owner to leaf.
    pub fn symbols(&self) -> &[LocalSymbolId] {
        &self.symbols
    }

    /// Return the symbol named by this path.
    pub fn symbol(&self) -> LocalSymbolId {
        *self
            .symbols
            .last()
            .unwrap_or_else(|| panic!("symbol path cannot be empty"))
    }

    /// Return the nearest declaration owner of the selected symbol.
    pub fn owner(&self) -> Option<LocalSymbolId> {
        self.symbols
            .len()
            .checked_sub(2)
            .map(|index| self.symbols[index])
    }
}

/// Result of looking up one binding symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolLookup {
    /// No visible symbol matched.
    Missing,
    /// Exactly one visible symbol matched.
    Found(LocalSymbolId),
    /// More than one visible symbol matched.
    Ambiguous(SmallVec<[LocalSymbolId; 4]>),
}

impl SymbolLookup {
    /// Add one matching symbol.
    pub fn push(&mut self, symbol: LocalSymbolId) {
        match self {
            Self::Missing => {
                *self = Self::Found(symbol);
            }
            Self::Found(first) => {
                let mut symbols = SmallVec::new();
                symbols.push(*first);
                symbols.push(symbol);

                *self = Self::Ambiguous(symbols);
            }
            Self::Ambiguous(symbols) => {
                symbols.push(symbol);
            }
        }
    }
}

/// The lookup visibility of a symbol.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub enum SymbolVisibility {
    /// Visible from the binding point forward in its scope.
    Forward,
    /// Visible throughout its whole scope.
    Scope,
    /// Visible only through member lookup.
    Member,
    /// Excluded from scope lookup.
    Hidden,
}

/// The scope lookup role of a symbol.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
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
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    Reflect,
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

/// The declaration kind of a symbol.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    Reflect,
)]
pub enum SymbolKind {
    /// Plain variable-like value symbol without a more specific kind.
    #[default]
    Variable,
    /// Callable value parameter symbol.
    Parameter,
    /// Control label symbol.
    Label,
    /// Imported dependency binding before target resolution.
    Import,
    /// Explicit public export alias.
    ExportAlias,
    /// Class symbol.
    Class,
    /// Struct symbol.
    Struct,
    /// Structural interface constraint symbol.
    Interface,
    /// Nominal interface constraint symbol.
    NewtypeInterface,
    /// Enum symbol.
    Enum,
    /// Enum or derived newtype variant symbol.
    Variant,
    /// Function symbol.
    Function,
    /// Extension symbol.
    Extension,
    /// Type alias declaration symbol.
    TypeAlias,
    /// Generic type parameter symbol.
    GenericTypeParameter,
    /// Generic const parameter symbol.
    GenericConstParameter,
    /// Generic lifetime parameter symbol.
    GenericLifetimeParameter,
    /// Associated type declaration symbol.
    AssociatedType,
    /// Associated constant declaration symbol.
    AssociatedConst,
    /// Concrete nominal newtype symbol.
    Newtype,
}

impl SymbolKind {
    /// Check if this is an interface.
    #[inline]
    pub fn is_interface(self) -> bool {
        matches!(self, Self::Interface | Self::NewtypeInterface)
    }

    /// Check whether this kind declares a nominal type.
    #[inline]
    pub fn is_nominal(self) -> bool {
        matches!(
            self,
            Self::Class | Self::Enum | Self::Newtype | Self::NewtypeInterface | Self::Struct
        )
    }

    /// Check whether this kind has a checked declaration definition.
    #[inline]
    pub fn is_definition(self) -> bool {
        matches!(
            self,
            Self::Class
                | Self::Enum
                | Self::Extension
                | Self::Interface
                | Self::Newtype
                | Self::NewtypeInterface
                | Self::Struct
                | Self::TypeAlias
        )
    }

    /// Check whether this kind declares a type definition.
    #[inline]
    pub fn is_type_definition(self) -> bool {
        matches!(
            self,
            Self::Class
                | Self::Enum
                | Self::Interface
                | Self::Newtype
                | Self::NewtypeInterface
                | Self::Struct
                | Self::TypeAlias
        )
    }

    /// Return whether this kind declares a value.
    pub fn is_value(self) -> bool {
        matches!(
            self,
            Self::Variable
                | Self::Parameter
                | Self::Function
                | Self::Class
                | Self::Variant
                | Self::AssociatedConst
                | Self::GenericConstParameter
        )
    }

    /// Check whether this kind declares a transparent type alias.
    #[inline]
    pub fn is_type_alias(self) -> bool {
        matches!(self, Self::TypeAlias)
    }

    /// Check whether this kind declares a body-owned value binding.
    #[inline]
    pub fn is_binding(self) -> bool {
        matches!(self, Self::Variable | Self::Parameter)
    }
}

/// Unique identifier for Symbols.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
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
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
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
