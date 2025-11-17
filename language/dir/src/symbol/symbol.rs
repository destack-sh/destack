use crate::{NodeId, NodeIdAny, ScopeId, StringId, Type};

/// Key for a symbol.
#[derive(Debug, Clone, Copy, PartialEq, Hash, PartialOrd, Eq)]
pub enum SymbolKey {
    /// Regular name key (like `x` or `"weird identifier"`).
    Name(StringId),
    /// Unique symbol expression (like `const x = Symbol("x");`).
    UniqueSymbol(NodeIdAny),
    /// Global symbol key (like `Symbol.iterator`).
    GlobalSymbol(StringId),
}

/// The space of a symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SymbolSpace {
    /// The type space.
    Type,
    /// The value space.
    Value,
}

/// Unique identifier for Symbols.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SymbolId(pub u32);

impl SymbolId {
    /// Wrap an id as a SymbolId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// A Symbol is a bindable item in a scope (which may also declare a scope).
/// Some symbols are virtual / anonymous (like block targets).
#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    /// The id of the symbol.
    pub id: SymbolId,
    /// The "space" of the symbol.
    pub space: SymbolSpace,
    /// The key of the symbol.
    pub key: Option<SymbolKey>,
    /// The scope that introduces the symbol.
    pub scope: ScopeId,
    /// The owned scope of the symbol.
    pub owned_scope: Option<ScopeId>,
    /// The main declaration node of the symbol.
    pub primary_declaration: Option<NodeIdAny>,
    /// Secondary declaration nodes of the symbol.
    pub secondary_declarations: Vec<NodeIdAny>,
    /// The declared type of the symbol.
    pub declared_ty: Option<NodeId<Type>>,
    /// The inferred type of the symbol.
    pub inferred_ty: Option<NodeId<Type>>,
    /// Forward to another remote symbol (like for imports, pattern bindings, etc.).
    pub target: Option<SymbolId>,
}

impl Symbol {
    /// Get the name of the symbol.
    #[inline]
    pub fn name(&self) -> Option<StringId> {
        match self.key {
            Some(SymbolKey::Name(name)) => Some(name),
            _ => None,
        }
    }
}
