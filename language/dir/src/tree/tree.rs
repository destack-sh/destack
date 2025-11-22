use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

use dyst_ast as ast;

use crate::{
    Annotation, Arena, Argument, Block, Declaration, DependencyItem, EnumField, Expression,
    LocalNodeId, LocalNodeIdAny, LocalScopeId, MatchCase, ModuleId, Node, NodeType, Parameter,
    Pattern, PatternField, Property, Type, TypeField, WhereClause, WithClause,
};

/// Mutable DIR Node tree across a set of related source units. NOT THREAD-SAFE.
#[derive(Clone)]
pub struct NodeTree {
    /// The module id of the node tree.
    pub module_id: ModuleId,

    // node index
    /// The next id to allocate.
    pub(crate) next_global_id: u32,
    /// The local ids of all nodes. Index is the global node id.
    pub(crate) local_id_by_node_id: Vec<u32>,
    /// The types of all nodes. Index is the global node id.
    pub(crate) node_type_by_node_id: Vec<NodeType>,

    // node arenas
    pub(crate) expressions: Arena<Expression>,
    pub(crate) blocks: Arena<Block>,
    pub(crate) declarations: Arena<Declaration>,
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
    /// The scopes by node id. Index is the global node id.
    /// (Main data is in SymbolTable, but indexed here for efficiency since *every* node needs a scope.)
    pub(crate) scopes_by_node_id: Vec<LocalScopeId>,
    /// The AST node ids of all nodes. Index is the global node id.
    pub(crate) source_id_by_node_id: Vec<Option<u32>>,
    /// The alias node id by AST node id.
    pub(crate) alias_node_id_by_source_id: HashMap<u32, u32>,
    /// The alias node id by DIR node id.
    pub(crate) alias_node_id_by_node_id: HashMap<u32, u32>,
    /// The annotations attached to nodes.
    pub(crate) annotations_by_node_id: HashMap<u32, Vec<LocalNodeId<Annotation>>>,
}

impl Debug for NodeTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeTree")
            .field("len", &self.local_id_by_node_id.len())
            .finish()
    }
}

impl NodeTree {
    /// Create a new NodeTree.
    pub fn new(module_id: ModuleId) -> Self {
        Self::with_capacity(module_id, 0)
    }

    /// Create a new NodeTree with the given capacity.
    pub fn with_capacity(module_id: ModuleId, capacity: usize) -> Self {
        Self {
            module_id,

            next_global_id: 0,
            local_id_by_node_id: Vec::with_capacity(capacity),
            node_type_by_node_id: Vec::with_capacity(capacity),

            expressions: Arena::new(),
            blocks: Arena::new(),
            declarations: Arena::new(),
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

            scopes_by_node_id: Vec::with_capacity(capacity),
            source_id_by_node_id: Vec::with_capacity(capacity),
            alias_node_id_by_source_id: HashMap::new(),
            alias_node_id_by_node_id: HashMap::new(),
            annotations_by_node_id: HashMap::new(),
        }
    }

    /// Allocate a new node in the tree.
    fn insert<T>(&mut self, node: T, scope_id: LocalScopeId) -> LocalNodeId<T>
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        let global_id = self.next_global_id;
        self.next_global_id = global_id + 1;
        self.node_type_by_node_id.push(T::TYPE);
        let local_id = <Self as NodeTreeImpl<T>>::allocate(self, node);
        self.local_id_by_node_id.push(local_id);
        self.scopes_by_node_id.push(scope_id);
        LocalNodeId::new(global_id)
    }

    /// Allocate a new node in the DIR tree lowered from a source AST node.
    pub fn insert_from_source<T, U>(
        &mut self,
        node: T,
        ast_node_id: ast::LocalNodeId<U>,
        scope_id: LocalScopeId,
    ) -> LocalNodeId<T>
    where
        T: Node,
        Self: NodeTreeImpl<T>,
        U: ast::Node,
    {
        let node_id = self.insert(node, scope_id);
        self.source_id_by_node_id.push(Some(ast_node_id.id));
        self.alias_node_id_by_source_id
            .insert(ast_node_id.id, node_id.id);
        node_id
    }

    /// Allocate a new node in the DIR tree derived from another DIR node.
    pub fn insert_from<T, U>(
        &mut self,
        node: T,
        dir_node_id: LocalNodeId<U>,
        scope_id: Option<LocalScopeId>,
    ) -> LocalNodeId<T>
    where
        T: Node,
        Self: NodeTreeImpl<T>,
        U: Node,
    {
        let scope_id = scope_id.unwrap_or_else(|| self.scopes_by_node_id[dir_node_id.id as usize]);
        let node_id = self.insert(node, scope_id);
        self.source_id_by_node_id.push(None);
        self.alias_node_id_by_node_id
            .insert(dir_node_id.id, node_id.id);
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
    pub fn get_node_type(&self, id: u32) -> NodeType {
        self.node_type_by_node_id[id as usize]
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
                if self.node_type_by_node_id[global_index] == T::TYPE {
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
        self.node_type_by_node_id
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
                if self.node_type_by_node_id[global_index] == T::TYPE {
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

    /// Get the scope for a node id.
    #[inline]
    pub fn get_scope<T: Node>(&self, node_id: LocalNodeId<T>) -> LocalScopeId {
        self.scopes_by_node_id[node_id.id as usize]
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
    Declaration => declarations,
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
