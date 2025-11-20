use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

use dyst_dir::{self as dir, ModuleId};
use dyst_source::Arena;

use crate::{
    Annotation, Argument, Block, Definition, DependencyItem, EnumField, Expression, Node, NodeId,
    NodeType, Parameter, Pattern, PatternField, Property, Statement, SwitchCase, Type, TypeField,
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
    pub(crate) annotations_by_node_id: HashMap<u32, Vec<NodeId<Annotation>>>,

    /// The DIR ids of all nodes. Index is the global node id.
    pub(crate) dir_id_by_node_id: Vec<Option<u32>>,
    /// The alias node id by DIR node id.
    pub(crate) alias_node_id_by_dir_id: HashMap<u32, u32>,
    /// The alias node id by JS AST node id.
    pub(crate) alias_node_id_by_node_id: HashMap<u32, u32>,

    // node arenas
    pub(crate) blocks: Arena<Block>,
    pub(crate) statements: Arena<Statement>,
    pub(crate) expressions: Arena<Expression>,
    pub(crate) definitions: Arena<Definition>,
    pub(crate) fields: Arena<Property>,
    pub(crate) types: Arena<Type>,
    pub(crate) type_fields: Arena<TypeField>,
    pub(crate) enum_fields: Arena<EnumField>,
    pub(crate) dependency_items: Arena<DependencyItem>,
    pub(crate) switch_cases: Arena<SwitchCase>,
    pub(crate) parameters: Arena<Parameter>,
    pub(crate) arguments: Arena<Argument>,
    pub(crate) patterns: Arena<Pattern>,
    pub(crate) pattern_fields: Arena<PatternField>,
    pub(crate) annotations: Arena<Annotation>,
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
            annotations_by_node_id: HashMap::new(),
            dir_id_by_node_id: Vec::with_capacity(capacity),
            alias_node_id_by_dir_id: HashMap::new(),
            alias_node_id_by_node_id: HashMap::new(),
            // node arenas
            blocks: Arena::new(),
            statements: Arena::new(),
            expressions: Arena::new(),
            definitions: Arena::new(),
            fields: Arena::new(),
            types: Arena::new(),
            type_fields: Arena::new(),
            enum_fields: Arena::new(),
            dependency_items: Arena::new(),
            switch_cases: Arena::new(),
            parameters: Arena::new(),
            arguments: Arena::new(),
            patterns: Arena::new(),
            pattern_fields: Arena::new(),
            annotations: Arena::new(),
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
        let local_id = <Self as MutableNodeTreeImpl<T>>::allocate(self, node);
        self.local_id_by_node_id.push(local_id);
        self.module_by_node_id.push(module_id);
        NodeId::new(global_id)
    }

    /// Allocate a new node in the DIR tree derived from another node.
    pub fn insert_from_source<T, U>(
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
        self.dir_id_by_node_id.push(Some(dir_node_id.id));
        node_id
    }

    /// Allocate a new node in the JS AST tree derived from another DIR node.
    pub fn insert_from<T, U>(&mut self, node: T, dir_node_id: NodeId<U>) -> NodeId<T>
    where
        T: Node,
        Self: MutableNodeTreeImpl<T>,
        U: Node,
    {
        let module_id = self.module_by_node_id[dir_node_id.id as usize];
        let node_id = self.insert(node, module_id);
        self.dir_id_by_node_id.push(None);
        self.alias_node_id_by_dir_id
            .insert(dir_node_id.id, node_id.id);
        node_id
    }

    /// Alias a node in the JS AST tree from a DIR node.
    pub fn alias_from_source<T>(&mut self, dir_id: u32, alias: NodeId<T>)
    where
        T: Node,
        Self: MutableNodeTreeImpl<T>,
    {
        self.alias_node_id_by_dir_id.insert(dir_id, alias.id);
    }

    /// Alias a node in the JS AST tree from a JS AST node.
    pub fn alias_from<T>(&mut self, node_id: u32, alias: NodeId<T>)
    where
        T: Node,
        Self: MutableNodeTreeImpl<T>,
    {
        self.alias_node_id_by_node_id.insert(node_id, alias.id);
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

    /// Get the source and DIR id of a node by its global id.
    pub fn get_source(&self, node_id: u32) -> (ModuleId, Option<u32>) {
        (
            self.module_by_node_id[node_id as usize],
            self.dir_id_by_node_id[node_id as usize],
        )
    }

    /// Get the annotations for a node.
    pub fn get_annotations(&self, node_id: u32) -> Vec<NodeId<Annotation>> {
        self.annotations_by_node_id
            .get(&node_id)
            .cloned()
            .unwrap_or_else(Vec::new)
    }
}

/// Map node types to arenas.
pub trait MutableNodeTreeImpl<T: Node> {
    /// Allocate a node into the relevant arena.
    fn allocate(tree: &mut MutableNodeTree, node: T) -> u32;
    /// Get a node from the relevant arena.
    fn get(tree: &MutableNodeTree, idx: u32) -> &T;
    /// Get a mutable node from the relevant arena.
    fn get_mut(tree: &mut MutableNodeTree, idx: u32) -> &mut T;
}

macro_rules! impl_node_tree_store {
    ($ty:ty, $field:ident) => {
        impl MutableNodeTreeImpl<$ty> for MutableNodeTree {
            #[inline]
            fn allocate(tree: &mut MutableNodeTree, node: $ty) -> u32 {
                tree.$field.allocate(node)
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
    Property => fields,
    Type => types,
    TypeField => type_fields,
    EnumField => enum_fields,
    DependencyItem => dependency_items,
    SwitchCase => switch_cases,
    Parameter => parameters,
    Argument => arguments,
    Pattern => patterns,
    PatternField => pattern_fields,
    Annotation => annotations,
}
