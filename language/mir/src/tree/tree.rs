use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

use destack_core::Arena;
use destack_source::Span;
use serde::{Deserialize, Serialize};

use crate::{
    ArgumentSlice, Attribute, Block, DebugInfoTable, Field, Function, Global, Instruction, Local,
    LocalNodeId, MemoryTable, Node, NodeType, Type, TypeAlias, TypeTable, Value,
};

/// MIR node tree for a single module.
///
/// This is the main storage for all MIR nodes in a module. All nodes
/// (functions, blocks, instructions, locals, types) are stored in arenas
/// and referenced by `LocalNodeId<T>`.
#[derive(Clone, Serialize, Deserialize)]
pub struct NodeTree {
    /// The next global node id to allocate.
    pub(crate) next_global_id: u32,
    /// Maps global node id → local arena index.
    pub(crate) local_id_by_node_id: Vec<u32>,
    /// Maps global node id → node type.
    pub(crate) node_type_by_node_id: Vec<NodeType>,
    /// Maps global node id → attached attributes.
    pub(crate) attributes_by_node_id: HashMap<u32, Vec<Attribute>>,

    // node arenas
    pub(crate) functions: Arena<Function>,
    pub(crate) blocks: Arena<Block>,
    pub(crate) instructions: Arena<Instruction>,
    pub(crate) locals: Arena<Local>,
    pub(crate) types: Arena<Type>,
    pub(crate) type_aliases: Arena<TypeAlias>,
    pub(crate) fields: Arena<Field>,
    pub(crate) globals: Arena<Global>,

    // source tracking (MIR node id → DIR node id)
    /// Maps MIR node id → source DIR node id (for diagnostics).
    /// None for synthesized nodes that don't correspond to source.
    pub(crate) source_id_by_node_id: Vec<Option<u32>>,
    /// Source spans by MIR node id.
    pub(crate) span_by_node_id: Vec<Option<Span>>,

    // externalized instruction arguments
    /// Flat buffer of instruction arguments (for Call, CallIndirect, Intrinsic).
    /// Instructions reference slices of this buffer via ArgumentSlice.
    pub(crate) instruction_arguments: Vec<Value>,

    // metadata tables
    /// Type metadata table.
    pub type_table: TypeTable,
    /// Memory metadata table.
    pub memory_table: MemoryTable,
    /// Debug metadata table.
    pub debug_info: DebugInfoTable,
}

impl Debug for NodeTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeTree")
            .field("functions", &self.functions.len())
            .field("blocks", &self.blocks.len())
            .field("instructions", &self.instructions.len())
            .field("locals", &self.locals.len())
            .field("types", &self.types.len())
            .field("type_aliases", &self.type_aliases.len())
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
            attributes_by_node_id: HashMap::with_capacity(capacity),

            functions: Arena::new(),
            blocks: Arena::new(),
            instructions: Arena::new(),
            locals: Arena::new(),
            types: Arena::new(),
            type_aliases: Arena::new(),
            fields: Arena::new(),
            globals: Arena::new(),

            source_id_by_node_id: Vec::with_capacity(capacity),
            span_by_node_id: Vec::with_capacity(capacity),
            instruction_arguments: Vec::new(),
            type_table: TypeTable::new(),
            memory_table: MemoryTable::new(),
            debug_info: DebugInfoTable::new(),
        }
    }

    /// Insert a node into the tree and return its id.
    /// The node will have no source DIR node associated (synthesized).
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
        self.source_id_by_node_id.push(None);
        self.span_by_node_id.push(None);

        LocalNodeId::new(global_id)
    }

    /// Insert a node into the tree with a source DIR node id for diagnostics.
    pub fn insert_from<T>(&mut self, node: T, source_dir_id: u32) -> LocalNodeId<T>
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        let global_id = self.next_global_id;
        self.next_global_id += 1;

        let local_id = <Self as NodeTreeImpl<T>>::allocate(self, node);
        self.local_id_by_node_id.push(local_id);
        self.node_type_by_node_id.push(T::TYPE);
        self.source_id_by_node_id.push(Some(source_dir_id));
        self.span_by_node_id.push(None);

        LocalNodeId::new(global_id)
    }

    /// Insert a type node into the tree and update the type cache.
    pub fn insert_type(&mut self, ty: Type) -> LocalNodeId<Type> {
        // determine the cache entry before moving the type
        let cache_entry = TypeTable::cache_entry_for_type(&ty);
        let type_id = self.insert(ty);

        // register the type in the cache
        if let Some(entry) = cache_entry {
            self.type_table.register_type_entry(type_id, entry);
        }

        type_id
    }

    /// Insert a type node into the tree with a source DIR id and update the type cache.
    pub fn insert_type_from(&mut self, ty: Type, source_dir_id: u32) -> LocalNodeId<Type> {
        // determine the cache entry before moving the type
        let cache_entry = TypeTable::cache_entry_for_type(&ty);
        let type_id = self.insert_from(ty, source_dir_id);

        // register the type in the cache
        if let Some(entry) = cache_entry {
            self.type_table.register_type_entry(type_id, entry);
        }

        type_id
    }

    /// Return the boolean type id.
    pub fn boolean_type(&self) -> LocalNodeId<Type> {
        // require a cached boolean type
        match self.type_table.boolean_type() {
            Some(type_id) => type_id,
            None => panic!("missing boolean type id in MIR type cache"),
        }
    }

    /// Return the void type id.
    pub fn void_type(&self) -> LocalNodeId<Type> {
        // require a cached void type
        match self.type_table.void_type() {
            Some(type_id) => type_id,
            None => panic!("missing void type id in MIR type cache"),
        }
    }

    /// Return the type tag type id.
    pub fn type_tag_type(&self) -> LocalNodeId<Type> {
        // require a cached type tag type
        match self.type_table.type_tag_type() {
            Some(type_id) => type_id,
            None => panic!("missing type tag type id in MIR type cache"),
        }
    }

    /// Return the isize type id.
    pub fn isize_type(&self) -> LocalNodeId<Type> {
        // require a cached isize type
        match self.type_table.isize_type() {
            Some(type_id) => type_id,
            None => panic!("missing isize type id in MIR type cache"),
        }
    }

    /// Return the usize type id.
    pub fn usize_type(&self) -> LocalNodeId<Type> {
        // require a cached usize type
        match self.type_table.usize_type() {
            Some(type_id) => type_id,
            None => panic!("missing usize type id in MIR type cache"),
        }
    }

    /// Return an integer type id for width and signedness.
    pub fn int_type(&self, width: u16, signed: bool) -> LocalNodeId<Type> {
        // require a cached integer type
        match self.type_table.int_type(width, signed) {
            Some(type_id) => type_id,
            None => panic!("missing int type id for width {width} signed {signed}"),
        }
    }

    /// Return a float type id for width.
    pub fn float_type(&self, width: u16) -> LocalNodeId<Type> {
        // require a cached float type
        match self.type_table.float_type(width) {
            Some(type_id) => type_id,
            None => panic!("missing float type id for width {width}"),
        }
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

    /// Get the source DIR node id for a MIR node, if available.
    /// Returns None for synthesized nodes that don't correspond to source.
    #[inline]
    pub fn get_source(&self, id: u32) -> Option<u32> {
        self.source_id_by_node_id[id as usize]
    }

    /// Set the source DIR node id for a MIR node.
    #[inline]
    pub fn set_source(&mut self, id: u32, source_dir_id: u32) {
        self.source_id_by_node_id[id as usize] = Some(source_dir_id);
    }

    /// Get the attributes for a node.
    #[inline]
    pub fn attributes<T>(&self, id: LocalNodeId<T>) -> &[Attribute]
    where
        T: Node,
    {
        self.attributes_by_node_id
            .get(&id.id)
            .map(|attrs| attrs.as_slice())
            .unwrap_or_default()
    }

    /// Set the attributes for a node.
    #[inline]
    pub fn set_attributes<T>(&mut self, id: LocalNodeId<T>, attributes: Vec<Attribute>)
    where
        T: Node,
    {
        if attributes.is_empty() {
            self.attributes_by_node_id.remove(&id.id);
        } else {
            self.attributes_by_node_id.insert(id.id, attributes);
        }
    }

    /// Push a new attribute onto a node.
    #[inline]
    pub fn push_attribute<T>(&mut self, id: LocalNodeId<T>, attribute: Attribute)
    where
        T: Node,
    {
        self.attributes_by_node_id
            .entry(id.id)
            .or_default()
            .push(attribute);
    }

    /// Get the span for a node.
    #[inline]
    pub fn get_span<T>(&self, id: LocalNodeId<T>) -> Option<Span>
    where
        T: Node,
    {
        self.span_by_node_id.get(id.id as usize).copied().flatten()
    }

    /// Get the span for a node by raw id.
    #[inline]
    pub fn get_span_by_id(&self, id: u32) -> Option<Span> {
        self.span_by_node_id.get(id as usize).copied().flatten()
    }

    /// Set the span for a node.
    #[inline]
    pub fn set_span<T>(&mut self, id: LocalNodeId<T>, span: Span)
    where
        T: Node,
    {
        if let Some(entry) = self.span_by_node_id.get_mut(id.id as usize) {
            *entry = Some(span);
        }
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

    /// Add arguments to the arguments buffer and return an ArgumentSlice.
    ///
    /// This is used when creating Call, CallIndirect, or Intrinsic instructions.
    #[inline]
    pub fn add_arguments(&mut self, args: &[Value]) -> ArgumentSlice {
        let start = self.instruction_arguments.len() as u32;
        let count = args.len() as u16;
        self.instruction_arguments.extend_from_slice(args);
        ArgumentSlice::new(start, count)
    }

    /// Get arguments from the arguments buffer by slice.
    #[inline]
    pub fn get_arguments(&self, slice: ArgumentSlice) -> &[Value] {
        let start = slice.start as usize;
        let end = start + slice.count as usize;
        &self.instruction_arguments[start..end]
    }

    /// Replace a node in-place, preserving the original at a new ID.
    ///
    /// - The original node is preserved at a new ID (for diagnostics/mapping)
    /// - The node at `id` is replaced with `replacement`
    /// - The source ID is preserved on both the original location and the preserved copy
    ///
    /// Returns the ID of the preserved original node.
    pub fn replace<T>(&mut self, id: LocalNodeId<T>, replacement: T) -> LocalNodeId<T>
    where
        T: Node + Clone,
        Self: NodeTreeImpl<T>,
    {
        // get original node and its source
        let original = self.get(id).clone();
        let source = self.source_id_by_node_id[id.id as usize];

        // preserve original at new ID
        let preserved_id = if let Some(source_dir_id) = source {
            self.insert_from(original, source_dir_id)
        } else {
            self.insert(original)
        };

        // replace in-place
        *self.get_mut(id) = replacement;

        preserved_id
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
impl_node_tree!(TypeAlias, type_aliases);
impl_node_tree!(Field, fields);
impl_node_tree!(Global, globals);
