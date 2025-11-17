use crate::{NodeId, NodeIdAny, NodeType, ScopeId, StringId, Type};

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

/// The kind of a symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SymbolKind {
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

/// A Symbol is a named, bindable item in a scope.
#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    /// The id of the symbol.
    pub id: SymbolId,
    /// The kind of the symbol.
    pub kind: SymbolKind,
    /// The name of the symbol.
    pub name: Option<StringId>,
    /// The key of the symbol.
    pub key: SymbolKey,
    /// The "space" of the symbol.
    pub space: SymbolSpace,
    /// The scope that introduces the symbol.
    pub scope: ScopeId,
    /// The main declaration node of the symbol.
    pub primary_declaration: NodeIdAny,
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
    /// Get the node type of the primary declaration.
    #[inline]
    pub fn primary_declaration_type(&self) -> NodeType {
        self.primary_declaration.ty
    }
}
