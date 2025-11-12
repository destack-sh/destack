use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

use dyst_dir::{self as dir, ModuleId};
use dyst_source::NodeArena;

use crate::{
    Annotation, Argument, Block, Definition, DependencyItem, EnumField, Expression, Field, Node,
    NodeId, NodeType, Parameter, Pattern, PatternField, Statement, SwitchCase, Type,
};

/// Mutable AST Node tree for a single source unit. NOT THREAD-SAFE.
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
    pub(crate) ast_id_by_node_id: Vec<Option<u32>>,
    /// The source DIR ids of all nodes. Index is the global node id.
    pub(crate) dir_id_by_node_id: Vec<Option<u32>>,

    // per-node arenas
    pub(crate) blocks: NodeArena<Block>,
    pub(crate) statements: NodeArena<Statement>,
    pub(crate) expressions: NodeArena<Expression>,
    pub(crate) definitions: NodeArena<Definition>,
    pub(crate) fields: NodeArena<Field>,
    pub(crate) types: NodeArena<Type>,
    pub(crate) enum_fields: NodeArena<EnumField>,
    pub(crate) dependency_items: NodeArena<DependencyItem>,
    pub(crate) switch_cases: NodeArena<SwitchCase>,
    pub(crate) parameters: NodeArena<Parameter>,
    pub(crate) arguments: NodeArena<Argument>,
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
            annotations_per_node_id: HashMap::new(),
            ast_id_by_node_id: Vec::with_capacity(capacity),
            dir_id_by_node_id: Vec::with_capacity(capacity),
            // per-node arenas
            blocks: NodeArena::new(),
            statements: NodeArena::new(),
            expressions: NodeArena::new(),
            definitions: NodeArena::new(),
            fields: NodeArena::new(),
            types: NodeArena::new(),
            enum_fields: NodeArena::new(),
            dependency_items: NodeArena::new(),
            switch_cases: NodeArena::new(),
            parameters: NodeArena::new(),
            arguments: NodeArena::new(),
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

    /// Allocate a new node in the DIR tree derived from another node.
    pub fn insert_from_dir<T, U>(
        &mut self,
        node: T,
        module_id: ModuleId,
        dir_node_id: dir::NodeId<U>,
    ) -> NodeId<T>
    where
        T: Node,
        Self: MutableNodeTreeImpl<T>,
        U: dir::Node,
        dir::MutableNodeTree: dir::MutableNodeTreeImpl<U>,
    {
        let node_id = self.insert(node, module_id);
        self.ast_id_by_node_id.push(None);
        self.dir_id_by_node_id.push(Some(dir_node_id.id));
        node_id
    }

    /// Get the next id.
    #[inline]
    pub fn next_id(&self) -> u32 {
        self.next_global_id
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

    /// Get the nodes for all nodes of a given type.
    #[inline]
    pub fn get_nodes<T>(&self) -> Vec<NodeId<T>>
    where
        T: Node,
    {
        let mut nodes = Vec::new();
        for (idx, ty) in self.type_by_node_id.iter().enumerate() {
            if *ty == T::TYPE {
                nodes.push(NodeId::new(idx as u32));
            }
        }
        nodes
    }

    /// Get the source and AST id of a node by its global id.
    /// Every DIR node has a source, but only some come directly from AST nodes.
    pub fn get_source_ast(&self, node_id: u32) -> (ModuleId, Option<u32>) {
        (
            self.module_by_node_id[node_id as usize],
            self.ast_id_by_node_id[node_id as usize],
        )
    }

    /// Get the source and DIR id of a node by its global id.
    pub fn get_source_dir(&self, node_id: u32) -> (ModuleId, Option<u32>) {
        (
            self.module_by_node_id[node_id as usize],
            self.dir_id_by_node_id[node_id as usize],
        )
    }

    /// Get the annotations for a node.
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

impl_node_tree_stores! {
    Block => blocks,
    Statement => statements,
    Expression => expressions,
    Definition => definitions,
    Field => fields,
    Type => types,
    EnumField => enum_fields,
    DependencyItem => dependency_items,
    SwitchCase => switch_cases,
    Parameter => parameters,
    Argument => arguments,
    Pattern => patterns,
    PatternField => pattern_fields,
    Annotation => annotations,
}
