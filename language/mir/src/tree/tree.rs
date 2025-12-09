use std::fmt::{Debug, Formatter};

use destack_source::Arena;

use crate::{
    Block, Field, Function, Global, Instruction, Local, LocalNodeId, Node, NodeType, Type,
};

/// MIR node tree for a single module.
///
/// This is the main storage for all MIR nodes in a module. All nodes
/// (functions, blocks, instructions, locals, types) are stored in arenas
/// and referenced by `LocalNodeId<T>`.
#[derive(Clone)]
pub struct NodeTree {
    /// The next global node id to allocate.
    pub(crate) next_global_id: u32,
    /// Maps global node id → local arena index.
    pub(crate) local_id_by_node_id: Vec<u32>,
    /// Maps global node id → node type.
    pub(crate) node_type_by_node_id: Vec<NodeType>,

    // node arenas
    pub(crate) functions: Arena<Function>,
    pub(crate) blocks: Arena<Block>,
    pub(crate) instructions: Arena<Instruction>,
    pub(crate) locals: Arena<Local>,
    pub(crate) types: Arena<Type>,
    pub(crate) fields: Arena<Field>,
    pub(crate) globals: Arena<Global>,
}

impl Debug for NodeTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeTree")
            .field("functions", &self.functions.len())
            .field("blocks", &self.blocks.len())
            .field("instructions", &self.instructions.len())
            .field("locals", &self.locals.len())
            .field("types", &self.types.len())
            .field("fields", &self.fields.len())
            .field("globals", &self.globals.len())
            .finish()
    }
}

impl Default for NodeTree {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeTree {
    /// Create a new empty node tree.
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Create a new node tree with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            next_global_id: 0,
            local_id_by_node_id: Vec::with_capacity(capacity),
            node_type_by_node_id: Vec::with_capacity(capacity),

            functions: Arena::new(),
            blocks: Arena::new(),
            instructions: Arena::new(),
            locals: Arena::new(),
            types: Arena::new(),
            fields: Arena::new(),
            globals: Arena::new(),
        }
    }

    /// Insert a node into the tree and return its id.
    pub fn insert<T>(&mut self, node: T) -> LocalNodeId<T>
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        let global_id = self.next_global_id;
        self.next_global_id += 1;

        let local_id = <Self as NodeTreeImpl<T>>::allocate(self, node);
        self.local_id_by_node_id.push(local_id);
        self.node_type_by_node_id.push(T::TYPE);

        LocalNodeId::new(global_id)
    }

    /// Get a reference to a node by id.
    #[inline]
    pub fn get<T>(&self, id: LocalNodeId<T>) -> &T
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        let local_id = self.local_id_by_node_id[id.id as usize];
        <Self as NodeTreeImpl<T>>::get(self, local_id)
    }

    /// Get a mutable reference to a node by id.
    #[inline]
    pub fn get_mut<T>(&mut self, id: LocalNodeId<T>) -> &mut T
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        let local_id = self.local_id_by_node_id[id.id as usize];
        <Self as NodeTreeImpl<T>>::get_mut(self, local_id)
    }

    /// Get the node type of a node by its raw id.
    #[inline]
    pub fn get_node_type(&self, id: u32) -> NodeType {
        self.node_type_by_node_id[id as usize]
    }

    /// Iterate over all nodes of a given type.
    pub fn iter_nodes<'a, T>(&'a self) -> impl Iterator<Item = (LocalNodeId<T>, &'a T)> + 'a
    where
        T: Node + 'a,
        Self: NodeTreeImpl<T>,
    {
        self.local_id_by_node_id
            .iter()
            .enumerate()
            .filter_map(|(global_id, &local_id)| {
                if self.node_type_by_node_id[global_id] == T::TYPE {
                    let id = LocalNodeId::new(global_id as u32);
                    let node = <Self as NodeTreeImpl<T>>::get(self, local_id);
                    Some((id, node))
                } else {
                    None
                }
            })
    }
}

/// Trait for mapping node types to arenas.
pub trait NodeTreeImpl<T: Node> {
    /// Allocate a node in the arena and return its local index.
    fn allocate(tree: &mut NodeTree, node: T) -> u32;
    /// Get a node from the arena by local index.
    fn get(tree: &NodeTree, idx: u32) -> &T;
    /// Get a mutable node from the arena by local index.
    fn get_mut(tree: &mut NodeTree, idx: u32) -> &mut T;
}

macro_rules! impl_node_tree {
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

impl_node_tree!(Function, functions);
impl_node_tree!(Block, blocks);
impl_node_tree!(Instruction, instructions);
impl_node_tree!(Local, locals);
impl_node_tree!(Type, types);
impl_node_tree!(Field, fields);
impl_node_tree!(Global, globals);
