use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

use destack_ast as ast;

use crate::{
    Annotation, Arena, Argument, Block, Declaration, DependencyItem, EnumField, Expression,
    LocalNodeId, LocalNodeIdAny, LocalScopeId, LocalScopeMark, MatchCase, ModuleId, Node, NodeType,
    Parameter, Pattern, PatternField, Property, WhereClause,
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
    pub(crate) properties: Arena<Property>,
    pub(crate) enum_fields: Arena<EnumField>,
    pub(crate) where_clauses: Arena<WhereClause>,
    pub(crate) dependency_items: Arena<DependencyItem>,
    pub(crate) parameters: Arena<Parameter>,
    pub(crate) arguments: Arena<Argument>,
    pub(crate) match_cases: Arena<MatchCase>,
    pub(crate) patterns: Arena<Pattern>,
    pub(crate) pattern_fields: Arena<PatternField>,
    pub(crate) annotations: Arena<Annotation>,

    // node side data
    /// The parent node id by node id. Index is the global node id.
    /// (Unlike in AST, we can index parents here directly since we have the shape up front.)
    parent_id_by_node_id: Vec<Option<u32>>,
    /// The scopes by node id. Index is the global node id.
    /// (Main data is in SymbolTable, but indexed here for efficiency since *every* node needs a scope.)
    scopes_by_node_id: Vec<(LocalScopeId, LocalScopeMark)>,
    /// The AST node ids of all nodes. Index is the global node id.
    source_id_by_node_id: Vec<u32>,
    /// The alias node id by AST node id.
    alias_node_id_by_source_id: HashMap<u32, u32>,
    /// The alias node id by DIR node id.
    alias_node_id_by_node_id: HashMap<u32, u32>,
    /// The annotations attached to nodes.
    annotations_by_node_id: HashMap<u32, Vec<LocalNodeId<Annotation>>>,
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
            properties: Arena::new(),
            enum_fields: Arena::new(),
            where_clauses: Arena::new(),
            dependency_items: Arena::new(),
            parameters: Arena::new(),
            arguments: Arena::new(),
            match_cases: Arena::new(),
            patterns: Arena::new(),
            pattern_fields: Arena::new(),
            annotations: Arena::new(),

            parent_id_by_node_id: Vec::with_capacity(capacity),
            scopes_by_node_id: Vec::with_capacity(capacity),
            source_id_by_node_id: Vec::with_capacity(capacity),
            alias_node_id_by_source_id: HashMap::new(),
            alias_node_id_by_node_id: HashMap::new(),
            annotations_by_node_id: HashMap::new(),
        }
    }

    /// Reserve a new node slot in the tree for a node lowered from an AST node.
    pub fn reserve_from_source<U: ast::Node>(
        &mut self,
        node_type: NodeType,
        ast_node_id: ast::LocalNodeId<U>,
        scope: (LocalScopeId, LocalScopeMark),
        parent_id: Option<LocalNodeIdAny>,
    ) -> LocalNodeIdAny {
        let global_id = self.next_global_id;
        self.next_global_id = global_id + 1;

        self.node_type_by_node_id.push(node_type);
        self.local_id_by_node_id.push(u32::MAX); // placeholder, filled by insert
        self.scopes_by_node_id.push(scope);
        self.parent_id_by_node_id
            .push(parent_id.map(|parent_id| parent_id.id));
        self.source_id_by_node_id.push(ast_node_id.id);
        self.alias_node_id_by_source_id
            .insert(ast_node_id.id, global_id);

        LocalNodeIdAny::new(global_id, node_type)
    }

    /// Reserve a new node slot in the tree for a node derived from another DIR node.
    pub fn reserve_from(
        &mut self,
        node_type: NodeType,
        dir_node_id: LocalNodeIdAny,
        scope: (LocalScopeId, LocalScopeMark),
        parent_id: Option<LocalNodeIdAny>,
    ) -> LocalNodeIdAny {
        let global_id = self.next_global_id;
        self.next_global_id = global_id + 1;

        self.node_type_by_node_id.push(node_type);
        self.local_id_by_node_id.push(u32::MAX); // placeholder, filled by insert
        self.scopes_by_node_id.push(scope);
        self.parent_id_by_node_id
            .push(parent_id.map(|parent_id| parent_id.id));
        let source_id = self.source_id_by_node_id[dir_node_id.id as usize];
        self.source_id_by_node_id.push(source_id);
        self.alias_node_id_by_node_id
            .insert(dir_node_id.id, global_id);

        LocalNodeIdAny::new(global_id, node_type)
    }

    /// Fill in the node data for a previously reserved slot.
    pub fn insert<T>(&mut self, node_id: LocalNodeIdAny, node: T) -> LocalNodeId<T>
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        let local_id = <Self as NodeTreeImpl<T>>::allocate(self, node);
        self.local_id_by_node_id[node_id.id as usize] = local_id;
        LocalNodeId::new(node_id.id)
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

    /// Get the parent node id for a node id.
    #[inline]
    pub fn get_parent_id(&self, node_id: u32) -> Option<u32> {
        self.parent_id_by_node_id[node_id as usize]
    }

    /// Get the parent node id for a node id.
    #[inline]
    pub fn get_parent(&self, node_id: u32) -> Option<LocalNodeIdAny> {
        let parent_id = self.parent_id_by_node_id[node_id as usize];
        parent_id.map(|parent_id| {
            LocalNodeIdAny::new(parent_id, self.node_type_by_node_id[parent_id as usize])
        })
    }

    /// Get the AST id of a node by its DIR node id.
    #[inline]
    pub fn get_source(&self, node_id: u32) -> u32 {
        self.source_id_by_node_id[node_id as usize]
    }

    /// Get the DIR node id by its AST id.
    #[inline]
    pub fn get_node_id_by_source_id(&self, ast_id: u32) -> Option<LocalNodeIdAny> {
        self.alias_node_id_by_source_id
            .get(&ast_id)
            .copied()
            .map(|node_id| {
                LocalNodeIdAny::new(node_id, self.node_type_by_node_id[node_id as usize])
            })
    }

    /// Get the scope for a node id.
    #[inline]
    pub fn get_scope<T: Node>(&self, node_id: LocalNodeId<T>) -> (LocalScopeId, LocalScopeMark) {
        self.scopes_by_node_id[node_id.id as usize]
    }

    /// Append a doc to a node by its global id.
    #[inline]
    pub fn append_annotation(
        &mut self,
        target_id: LocalNodeIdAny,
        annotation: LocalNodeId<Annotation>,
    ) {
        self.annotations_by_node_id
            .entry(target_id.id)
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
    Property => properties,
    EnumField => enum_fields,
    WhereClause => where_clauses,
    DependencyItem => dependency_items,
    Parameter => parameters,
    Argument => arguments,
    MatchCase => match_cases,
    Pattern => patterns,
    PatternField => pattern_fields,
    Annotation => annotations,
}
