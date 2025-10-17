use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

use dyst_ast as ast;
use dyst_source::SourceId;

use crate::tree::arena::NodeArena;
use crate::{
    Annotation, Argument, Block, Definition, Expression, ImportItem, MatchCase, Node, NodeId,
    NodeType, Parameter, Pattern, PatternField, Type, Variant, VariantField, WhereClause,
    WithClause,
};

/// The DIR Node tree across a set of related source units.
#[derive(Clone)]
pub struct NodeTree {
    /// The next id to allocate.
    pub(crate) next_global_id: u32,
    /// The local ids of all nodes. Index is the global node id.
    pub(crate) local_id_by_node_id: Vec<u32>,
    /// The types of all nodes. Index is the global node id.
    pub(crate) type_by_node_id: Vec<NodeType>,
    /// The sources of all nodes. Index is the global node id.
    pub(crate) source_by_node_id: Vec<SourceId>,
    /// The annotations attached to nodes.
    pub(crate) annotations_per_node_id: HashMap<u32, Vec<NodeId<Annotation>>>,

    /// The source AST ids of all nodes. Index is the global node id.
    pub(crate) ast_id_by_node_id: Vec<Option<u32>>,
    /// The alias node id by AST source / node id.
    pub(crate) alias_node_id_by_ast_id: HashMap<(SourceId, u32), u32>,
    /// The alias node id by DIR source / node id.
    pub(crate) alias_node_id_by_dir_id: HashMap<u32, u32>,

    // per-node arenas
    // groupings
    pub(crate) expressions: NodeArena<Expression>,
    pub(crate) blocks: NodeArena<Block>,
    // definitions
    pub(crate) definitions: NodeArena<Definition>,
    // types
    pub(crate) types: NodeArena<Type>,
    pub(crate) variants: NodeArena<Variant>,
    pub(crate) variant_fields: NodeArena<VariantField>,
    pub(crate) where_clauses: NodeArena<WhereClause>,
    // context
    pub(crate) with_clauses: NodeArena<WithClause>,
    pub(crate) import_items: NodeArena<ImportItem>,
    // bindings
    pub(crate) parameters: NodeArena<Parameter>,
    pub(crate) arguments: NodeArena<Argument>,
    // matching
    pub(crate) match_cases: NodeArena<MatchCase>,
    pub(crate) patterns: NodeArena<Pattern>,
    pub(crate) pattern_fields: NodeArena<PatternField>,
    // annotations
    pub(crate) annotations: NodeArena<Annotation>,
}

impl Debug for NodeTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeTree")
            .field("next_global_id", &self.next_global_id)
            .field("node_count", &self.local_id_by_node_id.len())
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
            source_by_node_id: Vec::with_capacity(capacity),
            ast_id_by_node_id: Vec::with_capacity(capacity),
            alias_node_id_by_ast_id: HashMap::new(),
            alias_node_id_by_dir_id: HashMap::new(),
            annotations_per_node_id: HashMap::new(),
            // groupings
            expressions: NodeArena::new(),
            blocks: NodeArena::new(),
            // definitions
            definitions: NodeArena::new(),
            // types
            types: NodeArena::new(),
            variants: NodeArena::new(),
            variant_fields: NodeArena::new(),
            where_clauses: NodeArena::new(),
            // context
            with_clauses: NodeArena::new(),
            import_items: NodeArena::new(),
            // bindings
            parameters: NodeArena::new(),
            arguments: NodeArena::new(),
            // matching
            match_cases: NodeArena::new(),
            patterns: NodeArena::new(),
            pattern_fields: NodeArena::new(),
            // annotations
            annotations: NodeArena::new(),
        }
    }

    /// Allocate a new node in the tree from a source AST node.
    fn insert<T>(&mut self, node: T, source_id: SourceId) -> NodeId<T>
    where
        T: Node,
        Self: NodeTreeStore<T>,
    {
        let global_id = self.next_global_id;
        self.next_global_id = global_id + 1;
        self.type_by_node_id.push(T::KIND);
        let local_id = <Self as NodeTreeStore<T>>::push(self, node);
        self.local_id_by_node_id.push(local_id);
        self.source_by_node_id.push(source_id);
        NodeId::new(global_id)
    }

    /// Allocate a new node in the DIR tree lowered from a source AST node.
    pub fn insert_from_ast<T, U>(
        &mut self,
        node: T,
        source_id: SourceId,
        ast_node_id: ast::NodeId<U>,
    ) -> NodeId<T>
    where
        T: Node,
        Self: NodeTreeStore<T>,
        U: ast::Node,
        ast::NodeTree: ast::NodeTreeStore<U>,
    {
        let node_id = self.insert(node, source_id);
        self.ast_id_by_node_id.push(Some(ast_node_id.id));
        self.alias_node_id_by_ast_id
            .insert((source_id, ast_node_id.id), node_id.id);
        node_id
    }

    /// Allocate a new node in the DIR tree derived from another node.
    pub fn insert_from_dir<T, U>(&mut self, node: T, dir_node_id: NodeId<U>) -> NodeId<T>
    where
        T: Node,
        Self: NodeTreeStore<T>,
        U: Node,
        Self: NodeTreeStore<U>,
    {
        let source_id = self.source_by_node_id[dir_node_id.id as usize];
        let node_id = self.insert(node, source_id);
        self.alias_node_id_by_dir_id.insert(node_id.id, node_id.id);
        node_id
    }

    /// Add an alias node for a lowered AST id.
    pub fn alias_from_ast<T>(&mut self, source_id: SourceId, ast_id: u32, alias: NodeId<T>)
    where
        T: Node,
        Self: NodeTreeStore<T>,
    {
        self.alias_node_id_by_ast_id
            .insert((source_id, ast_id), alias.id);
    }

    /// Add an alias node for a derived DIR id.
    pub fn alias_from_dir<T>(&mut self, dir_id: u32, alias: NodeId<T>)
    where
        T: Node,
        Self: NodeTreeStore<T>,
    {
        self.alias_node_id_by_dir_id.insert(dir_id, alias.id);
    }

    /// Get the type of an untyped node id.
    #[inline]
    pub fn get_type(&self, id: u32) -> NodeType {
        self.type_by_node_id[id as usize]
    }

    /// Get an immutable reference to the node with the given NodeId.
    #[inline]
    pub fn get<T>(&self, id: NodeId<T>) -> &T
    where
        T: Node,
        Self: NodeTreeStore<T>,
    {
        let local_id = self.local_id_by_node_id[id.id as usize];
        <Self as NodeTreeStore<T>>::get(self, local_id)
    }

    /// Get a mutable reference to the node with the given NodeId.
    #[inline]
    pub fn get_mut<T>(&mut self, id: NodeId<T>) -> &mut T
    where
        T: Node,
        Self: NodeTreeStore<T>,
    {
        let local_id = self.local_id_by_node_id[id.id as usize];
        <Self as NodeTreeStore<T>>::get_mut(self, local_id)
    }

    /// Iterate over all nodes of a given type together with their NodeId.
    #[inline]
    pub fn iter_nodes<'a, T>(&'a self) -> impl Iterator<Item = (NodeId<T>, &'a T)> + 'a
    where
        T: Node + 'a,
        Self: NodeTreeStore<T>,
    {
        self.local_id_by_node_id
            .iter()
            .enumerate()
            .filter_map(|(global_index, &local_index)| {
                if self.type_by_node_id[global_index] == T::KIND {
                    let node_id = NodeId::new(global_index as u32);
                    let node = <Self as NodeTreeStore<T>>::get(self, local_index);
                    Some((node_id, node))
                } else {
                    None
                }
            })
    }

    /// Get the source and AST id of a node by its global id.
    /// Every DIR node has a source, but only some come directly from AST nodes.
    pub fn get_source(&self, node_id: u32) -> (SourceId, Option<u32>) {
        (
            self.source_by_node_id[node_id as usize],
            self.ast_id_by_node_id[node_id as usize],
        )
    }

    // Get the node id by its source / AST id.
    #[inline]
    pub fn get_node_id_by_ast_id(&self, source_id: SourceId, ast_id: u32) -> Option<u32> {
        self.alias_node_id_by_ast_id
            .get(&(source_id, ast_id))
            .copied()
    }

    /// Append a doc to a node by its global id.
    #[inline]
    pub fn append_annotation(&mut self, target_id: u32, annotation: NodeId<Annotation>) {
        debug_assert!(target_id < self.next_global_id);
        self.annotations_per_node_id
            .entry(target_id)
            .or_default()
            .push(annotation);
    }

    /// Whether there are any annotations attached to a node.
    #[inline]
    pub fn has_annotations_for(&self, node_id: u32) -> bool {
        self.annotations_per_node_id.contains_key(&node_id)
    }

    /// Get annotations attached to a node.
    #[inline]
    pub fn get_annotations_for(&self, node_id: u32) -> Vec<NodeId<Annotation>> {
        self.annotations_per_node_id
            .get(&node_id)
            .cloned()
            .unwrap_or_else(Vec::new)
    }
}

/// Map node types to arenas.
pub trait NodeTreeStore<T: Node> {
    /// Push a node into the relevant arena.
    fn push(tree: &mut NodeTree, node: T) -> u32;
    /// Get a node from the relevant arena.
    fn get(tree: &NodeTree, idx: u32) -> &T;
    /// Get a mutable node from the relevant arena.
    fn get_mut(tree: &mut NodeTree, idx: u32) -> &mut T;
}

macro_rules! impl_node_tree_store {
    ($ty:ty, $field:ident) => {
        impl NodeTreeStore<$ty> for NodeTree {
            #[inline]
            fn push(tree: &mut NodeTree, node: $ty) -> u32 {
                tree.$field.push(node)
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
    // groupings
    Expression => expressions,
    Block => blocks,
    // definitions
    Definition => definitions,
    // types
    Type => types,
    Variant => variants,
    VariantField => variant_fields,
    WhereClause => where_clauses,
    // context
    WithClause => with_clauses,
    ImportItem => import_items,
    // bindings
    Parameter => parameters,
    Argument => arguments,
    // matching
    MatchCase => match_cases,
    Pattern => patterns,
    PatternField => pattern_fields,
    // annotations
    Annotation => annotations,
}
