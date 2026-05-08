use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{
    ExportKind, GlobalNodeIdAny, LocalNodeId, LocalScopeId, LocalScopeMark, Mutability, Node,
    NodeType, StaticKey, StringId,
};

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

/// How a symbol was introduced/bound.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum SymbolBinding {
    /// A runtime definition (let, const, var, class, function, etc.)
    #[default]
    Runtime,
    /// An ambient declaration (declare var, declare function, .d.ts)
    Ambient,
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

/// The lexical scope form used by a binding declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum BindingScope {
    /// Function-scoped declaration (`var`-style).
    Function,
    /// Block-scoped declaration (`let` and `const`-style).
    Block,
    /// Formal parameter declaration, including catch parameters.
    Parameter,
}

/// The semantic form of a symbol.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum SymbolForm {
    /// Plain value binding without a more specific form.
    #[default]
    Value,
    /// Class symbol.
    Class,
    /// Struct symbol.
    Struct,
    /// Interface symbol.
    Interface,
    /// Enum symbol.
    Enum,
    /// Function symbol.
    Function,
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
        self == SymbolForm::Interface
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

/// A bindable item or local in a scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Symbol {
    /// The scope lookup role of the symbol.
    pub role: SymbolRole,
    /// The semantic form of the symbol.
    pub form: SymbolForm,
    /// The lookup space of the symbol.
    pub space: SymbolSpace,
    /// How this symbol was introduced/bound.
    pub binding: SymbolBinding,
    /// The mutability for value bindings when known.
    pub binding_mutability: Option<Mutability>,
    /// The lexical scope form for declarations that bind into source scopes.
    pub binding_scope: Option<BindingScope>,

    /// Where this symbol was introduced.
    pub origin: SymbolOrigin,
    /// The key of the symbol.
    pub key: Option<StaticKey>,
    /// The scope that introduces the symbol.
    pub scope: (LocalScopeId, LocalScopeMark),

    /// The module id of the scope.
    pub module_id: ModuleId,
    /// The export kind of the symbol.
    pub export: Option<ExportKind>,
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
        self.space == SymbolSpace::Type
            && self
                .declaration
                .is_some_and(|declaration| declaration.local_id.ty == NodeType::Parameter)
    }

    /// Declare this symbol from a declaration node.
    pub fn declare<T: Node>(&mut self, node_id: LocalNodeId<T>) {
        self.declaration = Some(node_id.into_global_any(self.module_id));
    }
}
