use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

use dyst_ast as ast;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::{
    Annotation, Argument, Block, Definition, DependencyItem, EnumField, Expression, MatchCase,
    ModuleId, Node, NodeArena, NodeId, NodeType, Parameter, Pattern, PatternField, Property, Type,
    WhereClause, WithClause,
};

/// Mutable DIR Node tree across a set of related source units. NOT THREAD-SAFE.
#[derive(Clone)]
pub struct MutableNodeTree {
    /// The next id to allocate.
    pub(crate) next_global_id: u32,
    /// The local ids of all nodes. Index is the global node id.
    pub(crate) local_id_by_node_id: Vec<u32>,
    /// The types of all nodes. Index is the global node id.
    pub(crate) type_by_node_id: Vec<NodeType>,
    /// The sources of all nodes. Index is the global node id.
    pub(crate) module_by_node_id: Vec<ModuleId>,
    /// The annotations attached to nodes.
    pub(crate) annotations_per_node_id: HashMap<u32, Vec<NodeId<Annotation>>>,

    /// The source AST ids of all nodes. Index is the global node id.
    pub(crate) source_id_by_node_id: Vec<Option<u32>>,
    /// The alias node id by AST source / node id.
    pub(crate) alias_node_id_by_source_id: HashMap<(ModuleId, u32), u32>,
    /// The alias node id by DIR source / node id.
    pub(crate) alias_node_id_by_node_id: HashMap<u32, u32>,

    // per-node arenas
    pub(crate) expressions: NodeArena<Expression>,
    pub(crate) blocks: NodeArena<Block>,
    pub(crate) definitions: NodeArena<Definition>,
    pub(crate) types: NodeArena<Type>,
    pub(crate) properties: NodeArena<Property>,
    pub(crate) enum_fields: NodeArena<EnumField>,
    pub(crate) where_clauses: NodeArena<WhereClause>,
    pub(crate) with_clauses: NodeArena<WithClause>,
    pub(crate) dependency_items: NodeArena<DependencyItem>,
    pub(crate) parameters: NodeArena<Parameter>,
    pub(crate) arguments: NodeArena<Argument>,
    pub(crate) match_cases: NodeArena<MatchCase>,
    pub(crate) patterns: NodeArena<Pattern>,
    pub(crate) pattern_fields: NodeArena<PatternField>,
    pub(crate) annotations: NodeArena<Annotation>,
}

impl Debug for MutableNodeTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeTree")
            .field("next_global_id", &self.next_global_id)
            .field("node_count", &self.local_id_by_node_id.len())
            .finish()
    }
}

impl Default for MutableNodeTree {
    fn default() -> Self {
        Self::new()
    }
}

impl MutableNodeTree {
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
            module_by_node_id: Vec::with_capacity(capacity),
            source_id_by_node_id: Vec::with_capacity(capacity),
            alias_node_id_by_source_id: HashMap::new(),
            alias_node_id_by_node_id: HashMap::new(),
            annotations_per_node_id: HashMap::new(),
            expressions: NodeArena::new(),
            blocks: NodeArena::new(),
            definitions: NodeArena::new(),
            types: NodeArena::new(),
            properties: NodeArena::new(),
            enum_fields: NodeArena::new(),
            where_clauses: NodeArena::new(),
            with_clauses: NodeArena::new(),
            dependency_items: NodeArena::new(),
            parameters: NodeArena::new(),
            arguments: NodeArena::new(),
            match_cases: NodeArena::new(),
            patterns: NodeArena::new(),
            pattern_fields: NodeArena::new(),
            annotations: NodeArena::new(),
        }
    }

    /// Allocate a new node in the tree.
    fn insert<T>(&mut self, node: T, module_id: ModuleId) -> NodeId<T>
    where
        T: Node,
        Self: MutableNodeTreeImpl<T>,
    {
        let global_id = self.next_global_id;
        self.next_global_id = global_id + 1;
        self.type_by_node_id.push(T::TYPE);
        let local_id = <Self as MutableNodeTreeImpl<T>>::push(self, node);
        self.local_id_by_node_id.push(local_id);
        self.module_by_node_id.push(module_id);
        NodeId::new(global_id)
    }

    /// Allocate a new node in the DIR tree lowered from a source AST node.
    pub fn insert_from_source<T, U>(
        &mut self,
        node: T,
        module_id: ModuleId,
        ast_node_id: ast::NodeId<U>,
    ) -> NodeId<T>
    where
        T: Node,
        Self: MutableNodeTreeImpl<T>,
        U: ast::Node,
    {
        let node_id = self.insert(node, module_id);
        self.source_id_by_node_id.push(Some(ast_node_id.id));
        self.alias_node_id_by_source_id
            .insert((module_id, ast_node_id.id), node_id.id);
        node_id
    }

    /// Allocate a new node in the DIR tree derived from another DIR node.
    pub fn insert_from<T, U>(&mut self, node: T, dir_node_id: NodeId<U>) -> NodeId<T>
    where
        T: Node,
        Self: MutableNodeTreeImpl<T>,
        U: Node,
    {
        let module_id = self.module_by_node_id[dir_node_id.id as usize];
        let node_id = self.insert(node, module_id);
        self.source_id_by_node_id.push(None);
        self.alias_node_id_by_node_id.insert(node_id.id, node_id.id);
        node_id
    }

    /// Add an alias node for a lowered AST id.
    pub fn alias_from_source<T>(&mut self, module_id: ModuleId, ast_id: u32, alias: NodeId<T>)
    where
        T: Node,
        Self: MutableNodeTreeImpl<T>,
    {
        self.alias_node_id_by_source_id
            .insert((module_id, ast_id), alias.id);
    }

    /// Add an alias node for a derived DIR id.
    pub fn alias_from<T>(&mut self, dir_id: u32, alias: NodeId<T>)
    where
        T: Node,
        Self: MutableNodeTreeImpl<T>,
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
    pub fn get<T>(&self, id: NodeId<T>) -> &T
    where
        T: Node,
        Self: MutableNodeTreeImpl<T>,
    {
        let local_id = self.local_id_by_node_id[id.id as usize];
        <Self as MutableNodeTreeImpl<T>>::get(self, local_id)
    }

    /// Get a mutable reference to the node with the given NodeId.
    #[inline]
    pub fn get_mut<T>(&mut self, id: NodeId<T>) -> &mut T
    where
        T: Node,
        Self: MutableNodeTreeImpl<T>,
    {
        let local_id = self.local_id_by_node_id[id.id as usize];
        <Self as MutableNodeTreeImpl<T>>::get_mut(self, local_id)
    }

    /// Iterate over all nodes of a given type together with their NodeId.
    #[inline]
    pub fn iter_nodes<'a, T>(&'a self) -> impl Iterator<Item = (NodeId<T>, &'a T)> + 'a
    where
        T: Node + 'a,
        Self: MutableNodeTreeImpl<T>,
    {
        self.local_id_by_node_id
            .iter()
            .enumerate()
            .filter_map(|(global_index, &local_index)| {
                if self.type_by_node_id[global_index] == T::TYPE {
                    let node_id = NodeId::new(global_index as u32);
                    let node = <Self as MutableNodeTreeImpl<T>>::get(self, local_index);
                    Some((node_id, node))
                } else {
                    None
                }
            })
    }

    /// Get the source and AST id of a node by its global id.
    /// Every DIR node has a source, but only some come directly from AST nodes.
    pub fn get_source(&self, node_id: u32) -> (ModuleId, Option<u32>) {
        (
            self.module_by_node_id[node_id as usize],
            self.source_id_by_node_id[node_id as usize],
        )
    }

    // Get the node id by its source / AST id.
    #[inline]
    pub fn get_node_id_by_source_id(&self, module_id: ModuleId, ast_id: u32) -> Option<u32> {
        self.alias_node_id_by_source_id
            .get(&(module_id, ast_id))
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
    pub fn has_annotations(&self, node_id: u32) -> bool {
        self.annotations_per_node_id.contains_key(&node_id)
    }

    /// Get annotations attached to a node.
    #[inline]
    pub fn get_annotations(&self, node_id: u32) -> Vec<NodeId<Annotation>> {
        self.annotations_per_node_id
            .get(&node_id)
            .cloned()
            .unwrap_or_else(Vec::new)
    }
}

/// Map node types to arenas.
pub trait MutableNodeTreeImpl<T: Node> {
    /// Push a node into the relevant arena.
    fn push(tree: &mut MutableNodeTree, node: T) -> u32;
    /// Get a node from the relevant arena.
    fn get(tree: &MutableNodeTree, idx: u32) -> &T;
    /// Get a mutable node from the relevant arena.
    fn get_mut(tree: &mut MutableNodeTree, idx: u32) -> &mut T;
}

macro_rules! impl_node_tree_store {
    ($ty:ty, $field:ident) => {
        impl MutableNodeTreeImpl<$ty> for MutableNodeTree {
            #[inline]
            fn push(tree: &mut MutableNodeTree, node: $ty) -> u32 {
                tree.$field.push(node)
            }

            #[inline]
            fn get(tree: &MutableNodeTree, idx: u32) -> &$ty {
                tree.$field.get(idx)
            }

            #[inline]
            fn get_mut(tree: &mut MutableNodeTree, idx: u32) -> &mut $ty {
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

/// Shared DIR Node tree across a set of related source units. THREAD-SAFE.
/// NOTE #Performance: optimize SharedNodeTree locking (per module maybe?)
pub struct SharedNodeTree {
    inner: RwLock<MutableNodeTree>,
}

impl Clone for SharedNodeTree {
    fn clone(&self) -> Self {
        let state = self.inner.read().clone();
        Self {
            inner: RwLock::new(state),
        }
    }
}

impl Default for SharedNodeTree {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for SharedNodeTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedNodeTree").finish()
    }
}

impl SharedNodeTree {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(MutableNodeTree::new()),
        }
    }

    /// Get a read guard to the inner mutable node tree.
    #[inline]
    pub fn read(&self) -> RwLockReadGuard<'_, MutableNodeTree> {
        self.inner.read()
    }

    /// Get a write guard to the inner mutable node tree.
    #[inline]
    pub fn write(&self) -> RwLockWriteGuard<'_, MutableNodeTree> {
        self.inner.write()
    }

    /// Allocate a new node in the DIR tree lowered from a source AST node.
    pub fn insert_from_ast<T, U>(
        &self,
        node: T,
        module_id: ModuleId,
        ast_node_id: ast::NodeId<U>,
    ) -> NodeId<T>
    where
        T: Node,
        MutableNodeTree: MutableNodeTreeImpl<T>,
        U: ast::Node,
    {
        let mut tree = self.write();
        tree.insert_from_source(node, module_id, ast_node_id)
    }

    /// Allocate a new node in the DIR tree derived from another DIR node.
    pub fn insert_from_dir<T, U>(&self, node: T, dir_node_id: NodeId<U>) -> NodeId<T>
    where
        T: Node,
        MutableNodeTree: MutableNodeTreeImpl<T>,
        U: Node,
    {
        let mut tree = self.write();
        tree.insert_from(node, dir_node_id)
    }

    /// Add an alias node for a lowered AST id.
    pub fn alias_from_source<T>(&self, module_id: ModuleId, ast_id: u32, alias: NodeId<T>)
    where
        T: Node,
        MutableNodeTree: MutableNodeTreeImpl<T>,
    {
        let mut tree = self.write();
        tree.alias_from_source(module_id, ast_id, alias)
    }

    /// Get the type of an untyped node id.
    #[inline]
    pub fn get_type(&self, id: u32) -> NodeType {
        let tree = self.read();
        tree.get_type(id)
    }

    /// Get an immutable reference to the node with the given NodeId.
    #[inline]
    pub fn get<T>(&self, id: NodeId<T>) -> ReadNodeRef<'_, T>
    where
        T: Node,
        MutableNodeTree: MutableNodeTreeImpl<T>,
    {
        let tree = self.read();
        ReadNodeRef { tree, node_id: id }
    }

    /// Get a mutable reference to the node with the given NodeId.
    #[inline]
    pub fn get_mut<T>(&self, id: NodeId<T>) -> WriteNodeRef<'_, T>
    where
        T: Node,
        MutableNodeTree: MutableNodeTreeImpl<T>,
    {
        let tree = self.write();
        WriteNodeRef { tree, node_id: id }
    }

    /// Get the source and AST id of a node by its global id.
    /// Every DIR node has a source, but only some come directly from AST nodes.
    pub fn get_source(&self, node_id: u32) -> (ModuleId, Option<u32>) {
        let tree = self.read();
        tree.get_source(node_id)
    }

    /// Get the node id by its source / AST id.
    pub fn get_node_id_by_source_id(&self, module_id: ModuleId, ast_id: u32) -> Option<u32> {
        let tree = self.read();
        tree.get_node_id_by_source_id(module_id, ast_id)
    }

    /// Append a doc to a node by its global id.
    pub fn append_annotation(&self, target_id: u32, annotation: NodeId<Annotation>) {
        let mut tree = self.write();
        tree.append_annotation(target_id, annotation);
    }

    /// Whether there are any annotations attached to a node.
    pub fn has_annotations(&self, node_id: u32) -> bool {
        let tree = self.read();
        tree.has_annotations(node_id)
    }

    /// Get annotations attached to a node.
    pub fn get_annotations(&self, node_id: u32) -> Vec<NodeId<Annotation>> {
        let tree = self.read();
        tree.get_annotations(node_id)
    }
}

/// Immutable reference to a node.
#[derive(Debug)]
pub struct ReadNodeRef<'a, T>
where
    T: Node,
    MutableNodeTree: MutableNodeTreeImpl<T>,
{
    tree: RwLockReadGuard<'a, MutableNodeTree>,
    node_id: NodeId<T>,
}

impl<'a, T: Clone + Node> std::ops::Deref for ReadNodeRef<'a, T>
where
    MutableNodeTree: MutableNodeTreeImpl<T>,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.tree.get(self.node_id)
    }
}

impl<'a, T: Clone + Node> AsRef<T> for ReadNodeRef<'a, T>
where
    MutableNodeTree: MutableNodeTreeImpl<T>,
{
    fn as_ref(&self) -> &T {
        self.tree.get(self.node_id)
    }
}

/// Mutable reference to a node.
#[derive(Debug)]
pub struct WriteNodeRef<'a, T>
where
    T: Node,
    MutableNodeTree: MutableNodeTreeImpl<T>,
{
    tree: RwLockWriteGuard<'a, MutableNodeTree>,
    node_id: NodeId<T>,
}

impl<'a, T: Clone + Node> std::ops::Deref for WriteNodeRef<'a, T>
where
    MutableNodeTree: MutableNodeTreeImpl<T>,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.tree.get(self.node_id)
    }
}

impl<'a, T: Clone + Node> std::ops::DerefMut for WriteNodeRef<'a, T>
where
    MutableNodeTree: MutableNodeTreeImpl<T>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.tree.get_mut(self.node_id)
    }
}

impl<'a, T: Clone + Node> AsRef<T> for WriteNodeRef<'a, T>
where
    MutableNodeTree: MutableNodeTreeImpl<T>,
{
    fn as_ref(&self) -> &T {
        self.tree.get(self.node_id)
    }
}
