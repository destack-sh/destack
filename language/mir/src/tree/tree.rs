use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

use destack_core::Arena;
use destack_source::Span;
use serde::{Deserialize, Serialize};

use crate::{
    ArgumentSlice, Attribute, Block, CallSite, DataLayout, DebugTable, DevirtualizationMetadata,
    DispatchTable, Field, Function, Global, Instruction, InterfaceDispatchShape, Itab, ItabId,
    Layout, LayoutId, Local, LocalNodeId, MemoryTable, Node, NodeType, ProvenanceId,
    ProvenanceKind, ProvenanceReason, ProvenanceTable, Type, TypeAlias, TypeCache, TypeLineage,
    TypeTable, Value, Vtable, VtableId,
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

    // source tracking
    /// Maps MIR node id → provenance record.
    /// None for nodes without recorded semantic origin.
    pub(crate) provenance_by_node_id: Vec<Option<ProvenanceId>>,
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
    /// Dispatch metadata table.
    pub dispatch_table: DispatchTable,
    /// Debug metadata table.
    pub debug_table: DebugTable,
    /// Provenance metadata table.
    pub provenance_table: ProvenanceTable,
    /// Canonical module data layout metadata.
    #[serde(default)]
    pub data_layout: DataLayout,
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

            provenance_by_node_id: Vec::with_capacity(capacity),
            span_by_node_id: Vec::with_capacity(capacity),
            instruction_arguments: Vec::new(),
            type_table: TypeTable::new(),
            memory_table: MemoryTable::new(),
            dispatch_table: DispatchTable::new(),
            debug_table: DebugTable::new(),
            provenance_table: ProvenanceTable::new(),
            data_layout: DataLayout::default(),
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
        self.provenance_by_node_id.push(None);
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
        let provenance_id = self.create_direct_provenance(source_dir_id);

        self.provenance_by_node_id.push(Some(provenance_id));
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
        // use the cache when available
        if let Some(type_id) = self.type_table.boolean_type() {
            return type_id;
        }

        // fall back to a structural lookup
        if let Some(type_id) = self.find_type_by_predicate(|ty| matches!(ty, Type::Boolean)) {
            return type_id;
        }

        panic!("missing boolean type id in MIR type cache");
    }

    /// Return the void type id.
    pub fn void_type(&self) -> LocalNodeId<Type> {
        // use the cache when available
        if let Some(type_id) = self.type_table.void_type() {
            return type_id;
        }

        // fall back to a structural lookup
        if let Some(type_id) = self.find_type_by_predicate(|ty| matches!(ty, Type::Void)) {
            return type_id;
        }

        panic!("missing void type id in MIR type cache");
    }

    /// Return the type descriptor type id.
    pub fn type_descriptor_type(&self) -> LocalNodeId<Type> {
        // use the cache when available
        if let Some(type_id) = self.type_table.type_descriptor_type() {
            return type_id;
        }

        // fall back to a structural lookup
        if let Some(type_id) = self.find_type_by_predicate(|ty| matches!(ty, Type::TypeDescriptor))
        {
            return type_id;
        }

        panic!("missing type descriptor type id in MIR type cache");
    }

    /// Return the type id type id.
    pub fn type_id_type(&self) -> LocalNodeId<Type> {
        // use the cache when available
        if let Some(type_id) = self.type_table.type_id_type() {
            return type_id;
        }

        // fall back to a structural lookup
        if let Some(type_id) = self.find_type_by_predicate(|ty| matches!(ty, Type::TypeId)) {
            return type_id;
        }

        panic!("missing type id type in MIR type cache");
    }

    /// Return the isize type id.
    pub fn isize_type(&self) -> LocalNodeId<Type> {
        // use the cache when available
        if let Some(type_id) = self.type_table.isize_type() {
            return type_id;
        }

        // fall back to a structural lookup
        if let Some(type_id) = self.find_type_by_predicate(|ty| matches!(ty, Type::Isize)) {
            return type_id;
        }

        panic!("missing isize type id in MIR type cache");
    }

    /// Return lineage metadata for a type when present.
    pub fn type_lineage(&self, ty: LocalNodeId<Type>) -> Option<&TypeLineage> {
        self.type_table.lineage(ty)
    }

    /// Return the layout id for a type when present.
    pub fn type_layout_id(&self, ty: LocalNodeId<Type>) -> Option<LayoutId> {
        self.type_table.layout_id(ty)
    }

    /// Return the concrete layout for a type when present.
    pub fn type_layout(&self, ty: LocalNodeId<Type>) -> Option<&Layout> {
        let layout_id = self.type_layout_id(ty)?;
        Some(self.type_table.layout_table.layout(layout_id))
    }

    /// Return the type descriptor global for a type when present.
    pub fn type_descriptor_global(&self, ty: LocalNodeId<Type>) -> Option<LocalNodeId<Global>> {
        self.type_table.descriptor_global(ty)
    }

    /// Return the vtable metadata for a type when present.
    pub fn vtable_for_type(&self, ty: LocalNodeId<Type>) -> Option<(VtableId, &Vtable)> {
        let vtable_id = self.type_table.vtable_id(ty)?;
        Some((vtable_id, self.dispatch_table.vtable(vtable_id)))
    }

    /// Return the itab metadata for a concrete type and interface when present.
    pub fn itab_for_type(
        &self,
        concrete: LocalNodeId<Type>,
        interface: LocalNodeId<Type>,
    ) -> Option<(ItabId, &Itab)> {
        let itab_id = self.type_table.itab_id(concrete, interface)?;
        Some((itab_id, self.dispatch_table.itab(itab_id)))
    }

    /// Return the display name for a type when present.
    pub fn type_display_name(&self, ty: LocalNodeId<Type>) -> Option<destack_core::StringId> {
        self.type_table.display_name(ty)
    }

    /// Return the field lookup map for a type when present.
    pub fn type_field_map(
        &self,
        ty: LocalNodeId<Type>,
    ) -> Option<&std::collections::HashMap<destack_core::StringId, LocalNodeId<Field>>> {
        self.type_table.field_map(ty)
    }

    /// Return the canonical interface dispatch shape when present.
    pub fn interface_dispatch_shape(
        &self,
        interface: LocalNodeId<Type>,
    ) -> Option<&InterfaceDispatchShape> {
        self.dispatch_table.interface_dispatch_shape(interface)
    }

    /// Return sparse devirtualization metadata for a callsite when present.
    pub fn dispatch_metadata(&self, callsite: CallSite) -> Option<&DevirtualizationMetadata> {
        self.dispatch_table.callsite_metadata(callsite)
    }

    /// Return the usize type id.
    pub fn usize_type(&self) -> LocalNodeId<Type> {
        // use the cache when available
        if let Some(type_id) = self.type_table.usize_type() {
            return type_id;
        }

        // fall back to a structural lookup
        if let Some(type_id) = self.find_type_by_predicate(|ty| matches!(ty, Type::Usize)) {
            return type_id;
        }

        panic!("missing usize type id in MIR type cache");
    }

    /// Return an integer type id for width and signedness.
    pub fn int_type(&self, width: u16, signed: bool) -> LocalNodeId<Type> {
        // use the cache when available
        if let Some(type_id) = self.type_table.int_type(width, signed) {
            return type_id;
        }

        // fall back to a structural lookup
        if let Some(type_id) = self.find_type_by_predicate(|ty| {
            matches!(
                ty,
                Type::Int {
                    width: w,
                    is_signed: s,
                } if *w == width && *s == signed
            )
        }) {
            return type_id;
        }

        panic!("missing int type id for width {width} signed {signed}");
    }

    /// Return a float type id for width.
    pub fn float_type(&self, width: u16) -> LocalNodeId<Type> {
        // use the cache when available
        if let Some(type_id) = self.type_table.float_type(width) {
            return type_id;
        }

        // fall back to a structural lookup
        if let Some(type_id) =
            self.find_type_by_predicate(|ty| matches!(ty, Type::Float { width: w } if *w == width))
        {
            return type_id;
        }

        panic!("missing float type id for width {width}");
    }

    /// Return module pointer size in bytes.
    pub fn pointer_bytes(&self) -> u8 {
        self.data_layout.native_pointer_bytes
    }

    /// Return module pointer size in bits.
    pub fn pointer_bits(&self) -> u16 {
        self.data_layout.pointer_bits()
    }

    /// Update module pointer size in bytes.
    pub fn set_pointer_bytes(&mut self, pointer_bytes: u8) {
        match pointer_bytes {
            4 | 8 => {
                self.data_layout.native_pointer_bytes = pointer_bytes;
            }
            _ => {
                panic!("unsupported pointer size {pointer_bytes} bytes");
            }
        }
    }

    /// Rebuild the primitive type cache from canonical type nodes.
    pub fn rebuild_type_cache(&mut self) {
        // reset the cache state
        self.type_table.type_cache = TypeCache::default();

        // collect primitive entries before mutating the table
        let mut cache_entries = Vec::new();
        for (type_id, ty) in self.iter_nodes::<Type>() {
            let Some(cache_entry) = TypeTable::cache_entry_for_type(ty) else {
                continue;
            };
            cache_entries.push((type_id, cache_entry));
        }

        // repopulate the cache in node order
        for (type_id, cache_entry) in cache_entries {
            self.type_table.register_type_entry(type_id, cache_entry);
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

    /// Find the first type id matching a predicate.
    fn find_type_by_predicate(
        &self,
        predicate: impl Fn(&Type) -> bool,
    ) -> Option<LocalNodeId<Type>> {
        self.iter_nodes::<Type>()
            .find_map(|(type_id, ty)| predicate(ty).then_some(type_id))
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
        let provenance_id = self.provenance_by_node_id[id as usize]?;

        self.provenance_table.record(provenance_id).primary_origin()
    }

    /// Get the provenance record id for a MIR node, if available.
    #[inline]
    pub fn get_provenance(&self, id: u32) -> Option<ProvenanceId> {
        self.provenance_by_node_id[id as usize]
    }

    /// Set the provenance record id for a MIR node.
    #[inline]
    pub fn set_provenance(&mut self, id: u32, provenance_id: ProvenanceId) {
        self.provenance_by_node_id[id as usize] = Some(provenance_id);
    }

    /// Set the direct DIR origin for a MIR node.
    #[inline]
    pub fn set_source(&mut self, id: u32, source_dir_id: u32) {
        let provenance_id = self.create_direct_provenance(source_dir_id);

        self.set_provenance(id, provenance_id);
    }

    /// Create a provenance record for one MIR node.
    #[inline]
    pub fn create_provenance(
        &mut self,
        kind: ProvenanceKind,
        reason: Option<ProvenanceReason>,
        origins: Vec<u32>,
        parents: Vec<ProvenanceId>,
    ) -> ProvenanceId {
        self.provenance_table.create(kind, reason, origins, parents)
    }

    /// Create one direct provenance record.
    #[inline]
    pub fn create_direct_provenance(&mut self, origin: u32) -> ProvenanceId {
        self.provenance_table.direct(origin)
    }

    /// Create one synthetic provenance record.
    #[inline]
    pub fn create_synthetic_provenance(
        &mut self,
        reason: Option<ProvenanceReason>,
        parents: Vec<ProvenanceId>,
    ) -> ProvenanceId {
        self.provenance_table.synthetic(reason, parents)
    }

    /// Create one derived provenance record.
    #[inline]
    pub fn create_derived_provenance(
        &mut self,
        reason: Option<ProvenanceReason>,
        origins: Vec<u32>,
        parents: Vec<ProvenanceId>,
    ) -> ProvenanceId {
        self.provenance_table.derived(reason, origins, parents)
    }

    /// Create one merged provenance record.
    #[inline]
    pub fn create_merged_provenance(
        &mut self,
        origins: Vec<u32>,
        parents: Vec<ProvenanceId>,
    ) -> ProvenanceId {
        self.provenance_table.merged(origins, parents)
    }

    /// Create one inlined provenance record.
    #[inline]
    pub fn create_inlined_provenance(
        &mut self,
        origins: Vec<u32>,
        parents: Vec<ProvenanceId>,
    ) -> ProvenanceId {
        self.provenance_table.inlined(origins, parents)
    }

    /// Create one optimized provenance record.
    #[inline]
    pub fn create_optimized_provenance(
        &mut self,
        origins: Vec<u32>,
        parents: Vec<ProvenanceId>,
    ) -> ProvenanceId {
        self.provenance_table.optimized(origins, parents)
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
    /// - The provenance record is preserved on both the original location and the preserved copy
    ///
    /// Returns the ID of the preserved original node.
    pub fn replace<T>(&mut self, id: LocalNodeId<T>, replacement: T) -> LocalNodeId<T>
    where
        T: Node + Clone,
        Self: NodeTreeImpl<T>,
    {
        // get original node and its provenance
        let original = self.get(id).clone();
        let provenance = self.provenance_by_node_id[id.id as usize];

        // preserve original at new ID
        let preserved_id = self.insert(original);

        // preserve provenance on the preserved copy
        if let Some(provenance_id) = provenance {
            self.set_provenance(preserved_id.id, provenance_id);
        }

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
