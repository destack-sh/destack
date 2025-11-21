use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

use dyst_ast as ast;

use crate::{
    Annotation, Arena, Argument, Block, Definition, DependencyItem, EnumField, Expression,
    LocalNodeId, LocalNodeIdAny, LocalScopeId, LocalSymbolId, MatchCase, ModuleId, Node, NodeType,
    Parameter, Pattern, PatternField, Property, Scope, ScopeKind, Symbol, SymbolKey, SymbolSpace,
    Type, TypeField, WhereClause, WithClause,
};

/// Mutable DIR Node tree across a set of related source units. NOT THREAD-SAFE.
#[derive(Clone)]
pub struct NodeTree {
    // node index
    /// The next id to allocate.
    pub(crate) next_global_id: u32,
    /// The local ids of all nodes. Index is the global node id.
    pub(crate) local_id_by_node_id: Vec<u32>,
    /// The types of all nodes. Index is the global node id.
    pub(crate) type_by_node_id: Vec<NodeType>,

    // node arenas
    pub(crate) expressions: Arena<Expression>,
    pub(crate) blocks: Arena<Block>,
    pub(crate) definitions: Arena<Definition>,
    pub(crate) types: Arena<Type>,
    pub(crate) type_fields: Arena<TypeField>,
    pub(crate) properties: Arena<Property>,
    pub(crate) enum_fields: Arena<EnumField>,
    pub(crate) where_clauses: Arena<WhereClause>,
    pub(crate) with_clauses: Arena<WithClause>,
    pub(crate) dependency_items: Arena<DependencyItem>,
    pub(crate) parameters: Arena<Parameter>,
    pub(crate) arguments: Arena<Argument>,
    pub(crate) match_cases: Arena<MatchCase>,
    pub(crate) patterns: Arena<Pattern>,
    pub(crate) pattern_fields: Arena<PatternField>,
    pub(crate) annotations: Arena<Annotation>,

    // node side data
    /// The AST node ids of all nodes. Index is the global node id.
    pub(crate) source_id_by_node_id: Vec<Option<u32>>,
    /// The alias node id by AST node id.
    pub(crate) alias_node_id_by_source_id: HashMap<u32, u32>,
    /// The alias node id by DIR node id.
    pub(crate) alias_node_id_by_node_id: HashMap<u32, u32>,
    /// The annotations attached to nodes.
    pub(crate) annotations_by_node_id: HashMap<u32, Vec<LocalNodeId<Annotation>>>,

    // meta index
    /// The next symbol id to allocate.
    pub(crate) next_symbol_id: u32,
    /// The next scope id to allocate.
    pub(crate) next_scope_id: u32,

    // meta arenas
    pub(crate) symbols: Arena<Symbol>,
    pub(crate) scopes: Arena<Scope>,

    // meta side data
    /// The symbols by node id. Index is the global node id.
    pub(crate) symbol_by_node_id: Vec<Option<LocalSymbolId>>,
    /// The scopes by node id. Index is the global node id.
    pub(crate) scope_by_node_id: Vec<LocalScopeId>,
}

impl Debug for NodeTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeTree")
            .field("len", &self.local_id_by_node_id.len())
            .finish()
    }
}

impl Default for NodeTree {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeTree {
    /// Create a new NodeTree.
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Create a new NodeTree with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            next_global_id: 0,
            local_id_by_node_id: Vec::with_capacity(capacity),
            type_by_node_id: Vec::with_capacity(capacity),

            expressions: Arena::new(),
            blocks: Arena::new(),
            definitions: Arena::new(),
            types: Arena::new(),
            type_fields: Arena::new(),
            properties: Arena::new(),
            enum_fields: Arena::new(),
            where_clauses: Arena::new(),
            with_clauses: Arena::new(),
            dependency_items: Arena::new(),
            parameters: Arena::new(),
            arguments: Arena::new(),
            match_cases: Arena::new(),
            patterns: Arena::new(),
            pattern_fields: Arena::new(),
            annotations: Arena::new(),

            source_id_by_node_id: Vec::with_capacity(capacity),
            alias_node_id_by_source_id: HashMap::new(),
            alias_node_id_by_node_id: HashMap::new(),
            annotations_by_node_id: HashMap::new(),

            next_symbol_id: 0,
            next_scope_id: 0,

            symbols: Arena::new(),
            scopes: Arena::new(),

            symbol_by_node_id: Vec::with_capacity(capacity),
            scope_by_node_id: Vec::with_capacity(capacity),
        }
    }

    /// Allocate a new node in the tree.
    fn insert<T>(&mut self, node: T) -> LocalNodeId<T>
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        let global_id = self.next_global_id;
        self.next_global_id = global_id + 1;
        self.type_by_node_id.push(T::TYPE);
        let local_id = <Self as NodeTreeImpl<T>>::allocate(self, node);
        self.local_id_by_node_id.push(local_id);
        LocalNodeId::new(global_id)
    }

    /// Allocate a new node in the DIR tree lowered from a source AST node.
    pub fn insert_from_source<T, U>(
        &mut self,
        node: T,
        ast_node_id: ast::LocalNodeId<U>,
    ) -> LocalNodeId<T>
    where
        T: Node,
        Self: NodeTreeImpl<T>,
        U: ast::Node,
    {
        let node_id = self.insert(node);
        self.source_id_by_node_id.push(Some(ast_node_id.id));
        self.alias_node_id_by_source_id
            .insert(ast_node_id.id, node_id.id);
        node_id
    }

    /// Allocate a new node in the DIR tree derived from another DIR node.
    pub fn insert_from<T, U>(&mut self, node: T, dir_node_id: LocalNodeId<U>) -> LocalNodeId<T>
    where
        T: Node,
        Self: NodeTreeImpl<T>,
        U: Node,
    {
        let node_id = self.insert(node);
        self.source_id_by_node_id.push(None);
        self.alias_node_id_by_node_id
            .insert(dir_node_id.id, node_id.id);
        node_id
    }

    /// Allocate a new node in the DIR tree derived from a source AST node as the primary declaration.
    pub fn insert_from_source_as_symbol<T, U>(
        &mut self,
        node: T,
        ast_node_id: ast::LocalNodeId<U>,
        symbol_id: LocalSymbolId,
    ) -> LocalNodeId<T>
    where
        T: Node + Clone,
        Self: NodeTreeImpl<T>,
        U: ast::Node,
    {
        let node_id = self.insert_from_source(node, ast_node_id);
        self.symbols.get_mut(symbol_id.0).primary_declaration = Some(node_id.into_any());
        node_id
    }

    /// Add an alias node for a lowered AST id.
    pub fn alias_from_source<T>(&mut self, ast_id: u32, alias: LocalNodeId<T>)
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        self.alias_node_id_by_source_id.insert(ast_id, alias.id);
    }

    /// Add an alias node for a derived DIR id.
    pub fn alias_from<T>(&mut self, dir_id: u32, alias: LocalNodeId<T>)
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        self.alias_node_id_by_node_id.insert(dir_id, alias.id);
    }

    /// Get the type of an untyped node id.
    #[inline]
    pub fn get_type(&self, id: u32) -> NodeType {
        self.type_by_node_id[id as usize]
    }

    /// Get an immutable reference to the node with the given NodeId.
    #[inline]
    pub fn get<T>(&self, id: LocalNodeId<T>) -> &T
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        let local_id = self.local_id_by_node_id[id.id as usize];
        <Self as NodeTreeImpl<T>>::get(self, local_id)
    }

    /// Get a mutable reference to the node with the given NodeId.
    #[inline]
    pub fn get_mut<T>(&mut self, id: LocalNodeId<T>) -> &mut T
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        let local_id = self.local_id_by_node_id[id.id as usize];
        <Self as NodeTreeImpl<T>>::get_mut(self, local_id)
    }

    /// Iterate over all nodes of a given type together with their NodeId.
    pub fn iter_nodes_of_type<'a, T>(&'a self) -> impl Iterator<Item = (LocalNodeId<T>, &'a T)> + 'a
    where
        T: Node + 'a,
        Self: NodeTreeImpl<T>,
    {
        self.local_id_by_node_id
            .iter()
            .enumerate()
            .filter_map(|(global_index, &local_index)| {
                if self.type_by_node_id[global_index] == T::TYPE {
                    let node_id = LocalNodeId::new(global_index as u32);
                    let node = <Self as NodeTreeImpl<T>>::get(self, local_index);
                    Some((node_id, node))
                } else {
                    None
                }
            })
    }

    /// Iterate over all nodes ids.
    pub fn iter_node_ids(&self) -> impl Iterator<Item = LocalNodeIdAny> + '_ {
        self.type_by_node_id
            .iter()
            .enumerate()
            .map(|(global_index, &type_id)| LocalNodeIdAny::new(global_index as u32, type_id))
    }

    /// Iterate over all nodes ids of a given type.
    pub fn iter_node_ids_of_type<T>(&self) -> Vec<LocalNodeId<T>>
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        self.local_id_by_node_id
            .iter()
            .enumerate()
            .filter_map(|(global_index, _)| {
                if self.type_by_node_id[global_index] == T::TYPE {
                    let node_id = LocalNodeId::new(global_index as u32);
                    Some(node_id)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get the source and AST id of a node by its global id.
    /// Every DIR node has a source, but only some come directly from AST nodes.
    pub fn get_source(&self, node_id: u32) -> Option<u32> {
        self.source_id_by_node_id[node_id as usize]
    }

    // Get the node id by its source / AST id.
    #[inline]
    pub fn get_node_id_by_source_id(&self, ast_id: u32) -> Option<u32> {
        self.alias_node_id_by_source_id.get(&ast_id).copied()
    }

    /// Append a doc to a node by its global id.
    #[inline]
    pub fn append_annotation(&mut self, target_id: u32, annotation: LocalNodeId<Annotation>) {
        debug_assert!(target_id < self.next_global_id);
        self.annotations_by_node_id
            .entry(target_id)
            .or_default()
            .push(annotation);
    }

    /// Whether there are any annotations attached to a node.
    #[inline]
    pub fn has_annotations(&self, node_id: u32) -> bool {
        self.annotations_by_node_id.contains_key(&node_id)
    }

    /// Get annotations attached to a node.
    #[inline]
    pub fn get_annotations(&self, node_id: u32) -> Vec<LocalNodeId<Annotation>> {
        self.annotations_by_node_id
            .get(&node_id)
            .cloned()
            .unwrap_or_else(Vec::new)
    }

    /// Create a new symbol in the tree.
    pub fn create_symbol(
        &mut self,
        space: SymbolSpace,
        key: Option<SymbolKey>,
        scope: LocalScopeId,
        module_id: Option<ModuleId>,
    ) -> LocalSymbolId {
        let symbol_id = LocalSymbolId::new(self.next_symbol_id);
        self.next_symbol_id += 1;
        let symbol = Symbol {
            id: symbol_id,
            space,
            key,
            scope,
            module_id,
            owned_scope: None,
            primary_declaration: None,
            secondary_declarations: Vec::new(),
            declared_ty: None,
            inferred_ty: None,
            remote_symbol: None,
        };
        self.symbols.allocate(symbol);
        if let Some(key) = key {
            self.scopes.get_mut(scope.0).insert_symbol(key, symbol_id);
        }
        symbol_id
    }

    /// Create a new scope in the tree.
    pub fn create_scope(
        &mut self,
        kind: ScopeKind,
        parent: Option<LocalScopeId>,
        owner: Option<LocalSymbolId>,
        module_id: Option<ModuleId>,
    ) -> LocalScopeId {
        let scope_id = LocalScopeId::new(self.next_scope_id);
        self.next_scope_id += 1;
        let scope = Scope {
            id: scope_id,
            kind,
            owner,
            parent_id: parent,
            module_id,
            symbols: HashMap::new(),
            children: Vec::new(),
        };
        self.scopes.allocate(scope);
        if let Some(parent) = parent {
            self.scopes.get_mut(parent.0).insert_child_scope(scope_id);
        }
        scope_id
    }

    /// Create a new symbol with a scope.
    pub fn create_symbol_with_scope(
        &mut self,
        space: SymbolSpace,
        key: Option<SymbolKey>,
        kind: ScopeKind,
        parent: LocalScopeId,
        module_id: Option<ModuleId>,
    ) -> (LocalSymbolId, LocalScopeId) {
        let symbol_id = self.create_symbol(space, key, parent, module_id);
        let scope_id = self.create_scope(kind, Some(parent), Some(symbol_id), module_id);
        (symbol_id, scope_id)
    }

    /// Set the symbol for a node.
    #[inline]
    pub fn set_symbol(&mut self, node_id: u32, symbol_id: LocalSymbolId) {
        self.symbol_by_node_id[node_id as usize] = Some(symbol_id);
    }

    /// Get a symbol by its id.
    #[inline]
    pub fn get_symbol(&self, symbol_id: LocalSymbolId) -> &Symbol {
        self.symbols.get(symbol_id.0)
    }

    /// Get the symbol mutable by its id.
    #[inline]
    pub fn get_symbol_mut(&mut self, symbol_id: LocalSymbolId) -> &mut Symbol {
        self.symbols.get_mut(symbol_id.0)
    }

    /// Get the scope for a node id.
    #[inline]
    pub fn get_scope<T>(&self, node_id: LocalNodeId<T>) -> &Scope
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        let scope_id = self.scope_by_node_id[node_id.id as usize];
        self.get_scope_by_id(scope_id)
    }

    /// Get a scope by its id.
    #[inline]
    pub fn get_scope_by_id(&self, scope_id: LocalScopeId) -> &Scope {
        self.scopes.get(scope_id.0)
    }

    /// Get the scope mutable by its id.
    #[inline]
    pub fn get_scope_by_id_mut(&mut self, scope_id: LocalScopeId) -> &mut Scope {
        self.scopes.get_mut(scope_id.0)
    }

    /// Get the scope for a node by its id.
    #[inline]
    pub fn get_scope_by_node_id(&self, node_id: u32) -> LocalScopeId {
        self.scope_by_node_id[node_id as usize]
    }
}

/// Map node types to arenas.
pub trait NodeTreeImpl<T: Node> {
    /// Allocate a node into the relevant arena.
    fn allocate(tree: &mut NodeTree, node: T) -> u32;
    /// Get a node from the relevant arena.
    fn get(tree: &NodeTree, idx: u32) -> &T;
    /// Get a mutable node from the relevant arena.
    fn get_mut(tree: &mut NodeTree, idx: u32) -> &mut T;
}

macro_rules! impl_node_tree_store {
    ($ty:ty, $field:ident) => {
        impl NodeTreeImpl<$ty> for NodeTree {
            #[inline]
            fn allocate(tree: &mut NodeTree, node: $ty) -> u32 {
                tree.$field.allocate(node)
            }

            #[inline]
            fn get(tree: &NodeTree, idx: u32) -> &$ty {
                tree.$field.get(idx)
            }

            #[inline]
            fn get_mut(tree: &mut NodeTree, idx: u32) -> &mut $ty {
                tree.$field.get_mut(idx)
            }
        }
    };
}

macro_rules! impl_node_tree_stores {
    ( $( $ty:ty => $field:ident ),+ $(,)? ) => {
        $( impl_node_tree_store!($ty, $field); )*
    };
}

// usage
impl_node_tree_stores! {
    Expression => expressions,
    Block => blocks,
    Definition => definitions,
    Type => types,
    TypeField => type_fields,
    Property => properties,
    EnumField => enum_fields,
    WhereClause => where_clauses,
    WithClause => with_clauses,
    DependencyItem => dependency_items,
    Parameter => parameters,
    Argument => arguments,
    MatchCase => match_cases,
    Pattern => patterns,
    PatternField => pattern_fields,
    Annotation => annotations,
}
