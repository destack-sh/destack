use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::mem::size_of;

use destack_core::Arena;
use destack_source::{FileId, NodeSourceMap, NodeSpanType, Span};
use serde::{Deserialize, Serialize};

use crate::parse::Token;
use crate::{
    AddressSpace, ArgumentSlice, Attribute, Block, CommentSpan, Field, FieldSpan, Function,
    FunctionHeaderSpans, Global, Instruction, InterfaceDispatchShape, Itab, ItabId, Layout,
    LayoutId, LayoutMetadata, Local, LocalNodeId, ManagedReferenceRepresentation, Metadata,
    Mutability, Node, NodeType, ProvenanceId, ProvenanceReason, ReferenceKind, Terminator, Type,
    TypeAlias, TypeDeclarationSpans, TypeLineage, TypeReference, TypedValueSpan, ValueReference,
    Vtable, VtableId,
};

/// Approximate per-entry overhead for one hash-map entry.
const HASH_MAP_ENTRY_OVERHEAD_BYTES: usize = size_of::<usize>() * 3;

#[inline]
fn empty_source_span() -> Span {
    Span::empty(FileId::new(0))
}

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
    /// Source spans for parsed MIR node ownership.
    pub source_map: NodeSourceMap,
    /// The parsed MIR source text.
    pub(crate) source_text: String,
    /// The full parsed token stream.
    pub(crate) tokens: Vec<Token>,
    /// Leading comment spans keyed by global node id.
    pub(crate) leading_comment_spans_by_node_id: Vec<Option<Span>>,
    /// Parsed attribute spans keyed by global node id.
    pub(crate) attribute_spans_by_node_id: HashMap<u32, Vec<Span>>,
    /// Parsed declaration keyword spans keyed by global node id.
    pub(crate) keyword_spans_by_node_id: HashMap<u32, Span>,
    /// Parsed function parameter spans keyed by global node id.
    pub(crate) function_parameter_spans_by_node_id: HashMap<u32, Vec<TypedValueSpan>>,
    /// Parsed function header spans keyed by global node id.
    pub(crate) function_header_spans_by_node_id: HashMap<u32, FunctionHeaderSpans>,
    /// Parsed type field spans keyed by global node id.
    pub(crate) type_field_spans_by_node_id: HashMap<u32, Vec<FieldSpan>>,
    /// Parsed type declaration spans keyed by global node id.
    pub(crate) type_declaration_spans_by_node_id: HashMap<u32, TypeDeclarationSpans>,

    // node arenas
    pub(crate) functions: Arena<Function>,
    pub(crate) blocks: Arena<Block>,
    pub(crate) instructions: Arena<Instruction>,
    pub(crate) terminators: Arena<Terminator>,
    pub(crate) locals: Arena<Local>,
    pub(crate) types: Arena<Type>,
    pub(crate) type_aliases: Arena<TypeAlias>,
    pub(crate) fields: Arena<Field>,
    pub(crate) globals: Arena<Global>,

    // externalized instruction arguments
    /// Flat buffer of instruction arguments (for Call, CallIndirect, Intrinsic).
    /// Instructions reference slices of this buffer via ArgumentSlice.
    pub(crate) instruction_arguments: Vec<ValueReference>,

    // metadata
    /// Structured MIR metadata domains.
    #[serde(default)]
    pub metadata: Metadata,
}

impl Debug for NodeTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeTree")
            .field("functions", &self.functions.len())
            .field("blocks", &self.blocks.len())
            .field("instructions", &self.instructions.len())
            .field("terminators", &self.terminators.len())
            .field("locals", &self.locals.len())
            .field("types", &self.types.len())
            .field("type_aliases", &self.type_aliases.len())
            .field("fields", &self.fields.len())
            .field("globals", &self.globals.len())
            .field("tokens", &self.tokens.len())
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
            source_map: NodeSourceMap::with_capacity(capacity),
            source_text: String::new(),
            tokens: Vec::new(),
            leading_comment_spans_by_node_id: Vec::new(),
            attribute_spans_by_node_id: HashMap::new(),
            keyword_spans_by_node_id: HashMap::new(),
            function_parameter_spans_by_node_id: HashMap::new(),
            function_header_spans_by_node_id: HashMap::new(),
            type_field_spans_by_node_id: HashMap::new(),
            type_declaration_spans_by_node_id: HashMap::new(),

            functions: Arena::new(),
            blocks: Arena::new(),
            instructions: Arena::new(),
            terminators: Arena::new(),
            locals: Arena::new(),
            types: Arena::new(),
            type_aliases: Arena::new(),
            fields: Arena::new(),
            globals: Arena::new(),

            instruction_arguments: Vec::new(),
            metadata: Metadata::default(),
        }
    }

    /// Create a new node tree with parsed source data.
    pub(crate) fn with_parsed_source(source_text: String, tokens: Vec<Token>) -> Self {
        let mut tree = Self::new();
        tree.source_text = source_text;
        tree.tokens = tokens;
        tree
    }

    /// Return the owned bytes for this MIR node tree.
    pub fn owned_bytes(&self) -> usize {
        let mut owned_bytes = size_of::<Self>();
        owned_bytes += self.local_id_by_node_id.capacity() * size_of::<u32>();
        owned_bytes += self.node_type_by_node_id.capacity() * size_of::<NodeType>();
        owned_bytes += hash_map_bytes(&self.attributes_by_node_id);
        owned_bytes += size_of::<NodeSourceMap>();
        owned_bytes += self.source_text.capacity();
        owned_bytes += self.tokens.capacity() * size_of::<Token>();
        owned_bytes += self.leading_comment_spans_by_node_id.capacity() * size_of::<Option<Span>>();
        owned_bytes += hash_map_bytes(&self.attribute_spans_by_node_id);
        owned_bytes += hash_map_bytes(&self.keyword_spans_by_node_id);
        owned_bytes += hash_map_bytes(&self.function_parameter_spans_by_node_id);
        owned_bytes += hash_map_bytes(&self.function_header_spans_by_node_id);
        owned_bytes += hash_map_bytes(&self.type_field_spans_by_node_id);
        owned_bytes += hash_map_bytes(&self.type_declaration_spans_by_node_id);
        owned_bytes += self.functions.retained_bytes();
        owned_bytes += self.blocks.retained_bytes();
        owned_bytes += self.instructions.retained_bytes();
        owned_bytes += self.terminators.retained_bytes();
        owned_bytes += self.locals.retained_bytes();
        owned_bytes += self.types.retained_bytes();
        owned_bytes += self.type_aliases.retained_bytes();
        owned_bytes += self.fields.retained_bytes();
        owned_bytes += self.globals.retained_bytes();
        owned_bytes += self.instruction_arguments.capacity() * size_of::<ValueReference>();
        owned_bytes += self.metadata.layout.owned_bytes();
        owned_bytes += self.metadata.dispatch.owned_bytes();
        owned_bytes += self.metadata.debug.owned_bytes();
        owned_bytes += self.metadata.memory.owned_bytes();
        owned_bytes += self.metadata.provenance.owned_bytes();

        for attributes in self.attributes_by_node_id.values() {
            owned_bytes += attributes.capacity() * size_of::<Attribute>();
        }

        owned_bytes
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
        self.source_map.append(empty_source_span());
        self.metadata.provenance.provenance_by_node_id.push(None);

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
        self.source_map.append(empty_source_span());
        let origin_id = self.create_direct_provenance(source_dir_id);

        self.metadata
            .provenance
            .provenance_by_node_id
            .push(Some(origin_id));

        LocalNodeId::new(global_id)
    }

    /// Insert a type node into the tree and update the type cache.
    pub fn insert_type(&mut self, ty: Type) -> LocalNodeId<Type> {
        // determine the cache entry before moving the type
        let cache_entry = LayoutMetadata::cache_entry_for_type(&ty);
        let type_id = self.insert(ty);

        // register the type in the cache
        if let Some(entry) = cache_entry {
            self.metadata.layout.register_type_entry(type_id, entry);
        }

        type_id
    }

    /// Insert a type node into the tree with a source DIR id and update the type cache.
    pub fn insert_type_from(&mut self, ty: Type, source_dir_id: u32) -> LocalNodeId<Type> {
        // determine the cache entry before moving the type
        let cache_entry = LayoutMetadata::cache_entry_for_type(&ty);
        let type_id = self.insert_from(ty, source_dir_id);

        // register the type in the cache
        if let Some(entry) = cache_entry {
            self.metadata.layout.register_type_entry(type_id, entry);
        }

        type_id
    }

    /// Return the boolean type id.
    pub fn boolean_type(&self) -> LocalNodeId<Type> {
        // use the cache when available
        if let Some(type_id) = self.metadata.layout.boolean_type() {
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
        if let Some(type_id) = self.metadata.layout.void_type() {
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
        if let Some(type_id) = self.metadata.layout.type_descriptor_type() {
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
        if let Some(type_id) = self.metadata.layout.type_id_type() {
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
        if let Some(type_id) = self.metadata.layout.isize_type() {
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
        self.metadata.layout.lineage(ty)
    }

    /// Return the layout id for a type when present.
    pub fn type_layout_id(&self, ty: LocalNodeId<Type>) -> Option<LayoutId> {
        self.metadata.layout.layout_id(ty)
    }

    /// Return the concrete layout for a type when present.
    pub fn type_layout(&self, ty: LocalNodeId<Type>) -> Option<&Layout> {
        let layout_id = self.type_layout_id(ty)?;
        Some(self.metadata.layout.layout_table.layout(layout_id))
    }

    /// Return the canonical well known string reference type.
    pub fn string_type(&self) -> Option<LocalNodeId<Type>> {
        self.metadata.layout.string_type()
    }

    /// Return the canonical well known string layout id.
    pub fn string_layout_id(&self) -> Option<LayoutId> {
        let string_type = self.string_type()?;

        match self.get(string_type) {
            Type::Reference {
                pointee: TypeReference::Type(pointee),
                ..
            } => self.type_layout_id(*pointee),
            _ => self.type_layout_id(string_type),
        }
    }

    /// Return the type descriptor global for a type when present.
    pub fn type_descriptor_global(&self, ty: LocalNodeId<Type>) -> Option<LocalNodeId<Global>> {
        self.metadata.layout.descriptor_global(ty)
    }

    /// Return the vtable metadata for a type when present.
    pub fn vtable_for_type(&self, ty: LocalNodeId<Type>) -> Option<(VtableId, &Vtable)> {
        let vtable_id = self.metadata.dispatch.vtable_id(ty)?;
        Some((vtable_id, self.metadata.dispatch.vtable(vtable_id)))
    }

    /// Return the itab metadata for a concrete type and interface when present.
    pub fn itab_for_type(
        &self,
        concrete: LocalNodeId<Type>,
        interface: LocalNodeId<Type>,
    ) -> Option<(ItabId, &Itab)> {
        let itab_id = self.metadata.dispatch.itab_id(concrete, interface)?;
        Some((itab_id, self.metadata.dispatch.itab(itab_id)))
    }

    /// Return the display name for a type when present.
    pub fn type_display_name(&self, ty: LocalNodeId<Type>) -> Option<destack_core::StringId> {
        self.metadata.layout.display_name(ty)
    }

    /// Return the canonical interface dispatch shape when present.
    pub fn interface_dispatch_shape(
        &self,
        interface: LocalNodeId<Type>,
    ) -> Option<&InterfaceDispatchShape> {
        self.metadata.dispatch.interface_dispatch_shape(interface)
    }

    /// Return the usize type id.
    pub fn usize_type(&self) -> LocalNodeId<Type> {
        // use the cache when available
        if let Some(type_id) = self.metadata.layout.usize_type() {
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
        if let Some(type_id) = self.metadata.layout.int_type(width, signed) {
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
        if let Some(type_id) = self.metadata.layout.float_type(width) {
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

    /// Return the canonical storage type for the hidden environment field in `closure`.
    pub fn function_value_environment_type(&self) -> LocalNodeId<Type> {
        let void_type = if let Some(type_id) = self.metadata.layout.void_type() {
            type_id
        } else if let Some(type_id) = self.find_type_by_predicate(|ty| matches!(ty, Type::Void)) {
            type_id
        } else {
            panic!("missing void type for closure environment storage");
        };

        if let Some(type_id) = self.find_type_by_predicate(|ty| {
            matches!(
                ty,
                Type::Reference {
                    kind: ReferenceKind::Managed,
                    address_space: AddressSpace::Generic,
                    mutability: Mutability::Mutable,
                    pointee,
                    is_nullable: true,
                } if *pointee == TypeReference::Type(void_type)
            )
        }) {
            return type_id;
        }

        panic!("missing canonical closure environment storage type");
    }

    /// Ensure the canonical storage type for the hidden environment field in `closure`.
    pub fn ensure_function_value_environment_type(&mut self) -> LocalNodeId<Type> {
        // reuse or create the canonical void type
        let void_type = if let Some(type_id) = self.metadata.layout.void_type() {
            type_id
        } else if let Some(type_id) = self.find_type_by_predicate(|ty| matches!(ty, Type::Void)) {
            type_id
        } else {
            self.insert_type(Type::Void)
        };

        // reuse the canonical erased environment reference when present
        if let Some(type_id) = self.find_type_by_predicate(|ty| {
            matches!(
                ty,
                Type::Reference {
                    kind: ReferenceKind::Managed,
                    address_space: AddressSpace::Generic,
                    mutability: Mutability::Mutable,
                    pointee,
                    is_nullable: true,
                } if *pointee == TypeReference::Type(void_type)
            )
        }) {
            return type_id;
        }

        // otherwise create the canonical erased environment reference
        self.insert_type(Type::Reference {
            kind: ReferenceKind::Managed,
            address_space: AddressSpace::Generic,
            mutability: Mutability::Mutable,
            pointee: TypeReference::Type(void_type),
            is_nullable: true,
        })
    }

    /// Return module pointer size in bytes.
    pub fn pointer_bytes(&self) -> u8 {
        self.metadata.layout.storage.native_pointer_bytes
    }

    /// Return module pointer size in bits.
    pub fn pointer_bits(&self) -> u16 {
        self.metadata.layout.storage.pointer_bits()
    }

    /// Update module pointer size in bytes.
    pub fn set_pointer_bytes(&mut self, pointer_bytes: u8) {
        match pointer_bytes {
            4 | 8 => {
                self.metadata.layout.storage.native_pointer_bytes = pointer_bytes;

                // keep native-pointer managed references in lockstep
                if self
                    .metadata
                    .layout
                    .storage
                    .managed_reference_layout
                    .representation
                    == ManagedReferenceRepresentation::NativePointer
                {
                    self.metadata.layout.storage.managed_reference_layout.bytes = pointer_bytes;
                    self.metadata
                        .layout
                        .storage
                        .managed_reference_layout
                        .alignment = pointer_bytes;
                }
            }
            _ => {
                panic!("unsupported pointer size {pointer_bytes} bytes");
            }
        }
    }

    /// Rebuild the primitive type cache from canonical type nodes.
    pub fn rebuild_type_cache(&mut self) {
        // reset the cache state
        self.metadata.layout.clear_type_cache();

        // collect primitive entries before mutating the table
        let mut cache_entries = Vec::new();
        for (type_id, ty) in self.iter_nodes::<Type>() {
            let Some(cache_entry) = LayoutMetadata::cache_entry_for_type(ty) else {
                continue;
            };
            cache_entries.push((type_id, cache_entry));
        }

        // repopulate the cache in node order
        for (type_id, cache_entry) in cache_entries {
            self.metadata
                .layout
                .register_type_entry(type_id, cache_entry);
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
        let origin_id = self.metadata.provenance.provenance_by_node_id[id as usize]?;
        let record = self.metadata.provenance.record(origin_id);

        record.primary_dir_source_id()
    }

    /// Get the origin record id for a MIR node, if available.
    #[inline]
    pub fn get_provenance(&self, id: u32) -> Option<ProvenanceId> {
        self.metadata.provenance.provenance_by_node_id[id as usize]
    }

    /// Set the origin record id for a MIR node.
    #[inline]
    pub fn set_provenance(&mut self, id: u32, origin_id: ProvenanceId) {
        self.metadata.provenance.provenance_by_node_id[id as usize] = Some(origin_id);
    }

    /// Set the direct DIR origin for a MIR node.
    #[inline]
    pub fn set_source(&mut self, id: u32, source_dir_id: u32) {
        let origin_id = self.create_direct_provenance(source_dir_id);

        self.set_provenance(id, origin_id);
    }

    /// Create one direct DIR-local origin record.
    #[inline]
    pub fn create_direct_provenance(&mut self, source_dir_id: u32) -> ProvenanceId {
        self.metadata.provenance.direct_dir_local(source_dir_id)
    }

    /// Create one synthetic origin record.
    #[inline]
    pub fn create_synthetic_provenance(
        &mut self,
        reason: Option<ProvenanceReason>,
        parents: Vec<ProvenanceId>,
    ) -> ProvenanceId {
        self.metadata.provenance.synthetic(reason, parents)
    }

    /// Create one derived origin record.
    #[inline]
    pub fn create_derived_provenance(
        &mut self,
        reason: Option<ProvenanceReason>,
        origins: Vec<u32>,
        parents: Vec<ProvenanceId>,
    ) -> ProvenanceId {
        self.metadata.provenance.derived(reason, origins, parents)
    }

    /// Create one merged origin record.
    #[inline]
    pub fn create_merged_provenance(
        &mut self,
        origins: Vec<u32>,
        parents: Vec<ProvenanceId>,
    ) -> ProvenanceId {
        self.metadata.provenance.merged(origins, parents)
    }

    /// Create one inlined origin record.
    #[inline]
    pub fn create_inlined_provenance(
        &mut self,
        origins: Vec<u32>,
        parents: Vec<ProvenanceId>,
    ) -> ProvenanceId {
        self.metadata.provenance.inlined(origins, parents)
    }

    /// Create one optimized origin record.
    #[inline]
    pub fn create_optimized_provenance(
        &mut self,
        origins: Vec<u32>,
        parents: Vec<ProvenanceId>,
    ) -> ProvenanceId {
        self.metadata.provenance.optimized(origins, parents)
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
        let provenance_id = self.get_provenance(id.id)?;

        self.metadata.provenance.span(provenance_id)
    }

    /// Get the span for a node by raw id.
    #[inline]
    pub fn get_span_by_id(&self, id: u32) -> Option<Span> {
        let provenance_id = self.get_provenance(id)?;

        self.metadata.provenance.span(provenance_id)
    }

    /// Set the span for a node.
    #[inline]
    pub fn set_span<T>(&mut self, id: LocalNodeId<T>, span: Span)
    where
        T: Node,
    {
        let provenance_id = if let Some(provenance_id) = self.get_provenance(id.id) {
            provenance_id
        } else {
            let provenance_id = self.create_synthetic_provenance(None, Vec::new());
            self.set_provenance(id.id, provenance_id);
            provenance_id
        };

        self.metadata.provenance.set_span(provenance_id, span);
        self.source_map.set(id.id, span);
    }

    /// Set the span for one parsed MIR node and anchor it to the parsed text.
    #[inline]
    pub fn set_text_span<T>(&mut self, id: LocalNodeId<T>, span: Span)
    where
        T: Node,
    {
        let provenance_id = if let Some(provenance_id) = self.get_provenance(id.id) {
            provenance_id
        } else {
            let provenance_id = self.metadata.provenance.text(span);
            self.set_provenance(id.id, provenance_id);
            provenance_id
        };

        self.metadata.provenance.set_span(provenance_id, span);
        self.source_map.set(id.id, span);
    }

    /// Get the main source span for a MIR node when present.
    #[inline]
    pub fn get_main_span<T>(&self, id: LocalNodeId<T>) -> Option<Span>
    where
        T: Node,
    {
        self.source_map.get_main(id.id)
    }

    /// Get the main source span for a MIR node by raw id.
    #[inline]
    pub fn get_main_span_by_id(&self, id: u32) -> Option<Span> {
        self.source_map.get_main(id)
    }

    /// Set the main source span for a MIR node.
    #[inline]
    pub fn set_main_span<T>(&mut self, id: LocalNodeId<T>, span: Span)
    where
        T: Node,
    {
        self.source_map.set_main(id.id, span);
    }

    /// Get one side span for a MIR node when present.
    #[inline]
    pub fn get_side_span<T>(&self, id: LocalNodeId<T>, span_type: NodeSpanType) -> Option<Span>
    where
        T: Node,
    {
        self.source_map.get_side(id.id, span_type)
    }

    /// Get one side span for a MIR node by raw id when present.
    #[inline]
    pub fn get_side_span_by_id(&self, id: u32, span_type: NodeSpanType) -> Option<Span> {
        self.source_map.get_side(id, span_type)
    }

    /// Set one side span for a MIR node.
    #[inline]
    pub fn set_side_span<T>(&mut self, id: LocalNodeId<T>, span_type: NodeSpanType, span: Span)
    where
        T: Node,
    {
        self.source_map.set_side(id.id, span_type, span);
    }

    /// Return the leading comments for one node.
    pub fn leading_comments<T>(&self, id: LocalNodeId<T>) -> Vec<CommentSpan>
    where
        T: Node,
    {
        let span = self.leading_comment_span(id);
        self.comments_in_span(span)
    }

    /// Return the leading comment span for one node.
    pub(crate) fn leading_comment_span<T>(&self, id: LocalNodeId<T>) -> Option<Span>
    where
        T: Node,
    {
        self.leading_comment_spans_by_node_id
            .get(id.id as usize)
            .copied()
            .flatten()
    }

    /// Set the leading comment span for one raw node id.
    pub(crate) fn set_leading_comment_span_by_id(&mut self, id: u32, span: Span) {
        let index = id as usize;

        if index >= self.leading_comment_spans_by_node_id.len() {
            self.leading_comment_spans_by_node_id
                .resize(index + 1, None);
        }

        self.leading_comment_spans_by_node_id[index] = Some(span);
    }

    /// Return all parsed tokens.
    pub(crate) fn tokens(&self) -> &[Token] {
        &self.tokens
    }

    /// Return one source slice for the provided span.
    pub(crate) fn source_text(&self, span: Span) -> &str {
        let start = span.start as usize;
        let end = span.end as usize;

        &self.source_text[start..end]
    }

    /// Return the comments between two byte offsets.
    pub(crate) fn comments_between(&self, start: u32, end: u32) -> Vec<CommentSpan> {
        self.comments_in_bounds(start, end)
    }

    /// Return the first inline comment between two byte offsets.
    pub(crate) fn inline_comment_between(&self, start: u32, end: u32) -> Option<CommentSpan> {
        let mut saw_newline = false;

        // find the first comment before the first newline
        for token in self.tokens() {
            if token.span.end <= start {
                continue;
            }

            if token.span.start >= end {
                break;
            }

            // inline comments must stay before the first newline
            if token.ty == crate::parse::TokenType::Newline {
                saw_newline = true;
            }

            if token.ty == crate::parse::TokenType::Comment && !saw_newline {
                return Some(CommentSpan::new(token.span, self.source_text(token.span)));
            }
        }

        None
    }

    /// Return the parsed attribute spans for one node.
    pub fn attribute_spans<T>(&self, id: LocalNodeId<T>) -> &[Span]
    where
        T: Node,
    {
        self.attribute_spans_by_node_id
            .get(&id.id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Set the parsed attribute spans for one node.
    pub fn set_attribute_spans<T>(&mut self, id: LocalNodeId<T>, spans: Vec<Span>)
    where
        T: Node,
    {
        if spans.is_empty() {
            self.attribute_spans_by_node_id.remove(&id.id);
        } else {
            self.attribute_spans_by_node_id.insert(id.id, spans);
        }
    }

    /// Return the parsed declaration keyword span for one node.
    pub fn keyword_span<T>(&self, id: LocalNodeId<T>) -> Option<Span>
    where
        T: Node,
    {
        self.keyword_spans_by_node_id.get(&id.id).copied()
    }

    /// Set the parsed declaration keyword span for one node.
    pub fn set_keyword_span<T>(&mut self, id: LocalNodeId<T>, span: Span)
    where
        T: Node,
    {
        self.keyword_spans_by_node_id.insert(id.id, span);
    }

    /// Return the parsed function parameter spans for one function.
    pub fn function_parameter_spans(&self, id: LocalNodeId<Function>) -> &[TypedValueSpan] {
        self.function_parameter_spans_by_node_id
            .get(&id.id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Set the parsed function parameter spans for one function.
    pub fn set_function_parameter_spans(
        &mut self,
        id: LocalNodeId<Function>,
        spans: Vec<TypedValueSpan>,
    ) {
        if spans.is_empty() {
            self.function_parameter_spans_by_node_id.remove(&id.id);
        } else {
            self.function_parameter_spans_by_node_id
                .insert(id.id, spans);
        }
    }

    /// Return the parsed function header spans for one function.
    pub fn function_header_spans(&self, id: LocalNodeId<Function>) -> Option<&FunctionHeaderSpans> {
        self.function_header_spans_by_node_id.get(&id.id)
    }

    /// Set the parsed function header spans for one function.
    pub fn set_function_header_spans(
        &mut self,
        id: LocalNodeId<Function>,
        spans: FunctionHeaderSpans,
    ) {
        self.function_header_spans_by_node_id.insert(id.id, spans);
    }

    /// Return the parsed field spans for one type alias.
    pub fn type_field_spans(&self, id: LocalNodeId<TypeAlias>) -> &[FieldSpan] {
        self.type_field_spans_by_node_id
            .get(&id.id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Set the parsed field spans for one type alias.
    pub fn set_type_field_spans(&mut self, id: LocalNodeId<TypeAlias>, spans: Vec<FieldSpan>) {
        if spans.is_empty() {
            self.type_field_spans_by_node_id.remove(&id.id);
        } else {
            self.type_field_spans_by_node_id.insert(id.id, spans);
        }
    }

    /// Return the parsed type declaration spans for one type alias.
    pub fn type_declaration_spans(
        &self,
        id: LocalNodeId<TypeAlias>,
    ) -> Option<&TypeDeclarationSpans> {
        self.type_declaration_spans_by_node_id.get(&id.id)
    }

    /// Set the parsed type declaration spans for one type alias.
    pub fn set_type_declaration_spans(
        &mut self,
        id: LocalNodeId<TypeAlias>,
        spans: TypeDeclarationSpans,
    ) {
        self.type_declaration_spans_by_node_id.insert(id.id, spans);
    }

    /// Return the comments covered by one span.
    fn comments_in_span(&self, span: Option<Span>) -> Vec<CommentSpan> {
        let Some(span) = span else {
            return Vec::new();
        };

        self.comments_in_bounds(span.start, span.end)
    }

    /// Return the comments covered by one byte range.
    fn comments_in_bounds(&self, start: u32, end: u32) -> Vec<CommentSpan> {
        let mut comments = Vec::new();

        // collect comments in source order
        for token in &self.tokens {
            if token.span.end <= start {
                continue;
            }

            if token.span.start >= end {
                break;
            }

            if token.ty == crate::parse::TokenType::Comment {
                comments.push(CommentSpan::new(token.span, self.source_text(token.span)));
            }
        }

        comments
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
    pub fn add_arguments(&mut self, args: &[ValueReference]) -> ArgumentSlice {
        let start = self.instruction_arguments.len() as u32;
        let count = args.len() as u16;
        self.instruction_arguments.extend_from_slice(args);
        ArgumentSlice::new(start, count)
    }

    /// Get arguments from the arguments buffer by slice.
    #[inline]
    pub fn get_arguments(&self, slice: ArgumentSlice) -> &[ValueReference] {
        let start = slice.start as usize;
        let end = start + slice.count as usize;
        &self.instruction_arguments[start..end]
    }

    /// Replace a node in-place, preserving the original at a new ID.
    ///
    /// - The original node is preserved at a new ID (for diagnostics/mapping)
    /// - The node at `id` is replaced with `replacement`
    /// - The origin record is preserved on both the original location and the preserved copy
    ///
    /// Returns the ID of the preserved original node.
    pub fn replace<T>(&mut self, id: LocalNodeId<T>, replacement: T) -> LocalNodeId<T>
    where
        T: Node + Clone,
        Self: NodeTreeImpl<T>,
    {
        // get original node and its origin
        let original = self.get(id).clone();
        let origin = self.metadata.provenance.provenance_by_node_id[id.id as usize];

        // preserve original at new ID
        let preserved_id = self.insert(original);

        // preserve origin on the preserved copy
        if let Some(origin_id) = origin {
            self.set_provenance(preserved_id.id, origin_id);
        }

        // replace in-place
        *self.get_mut(id) = replacement;

        preserved_id
    }
}

/// Return the approximate owned bytes for one hash map table.
fn hash_map_bytes<K, V>(map: &HashMap<K, V>) -> usize {
    size_of::<HashMap<K, V>>()
        + map.capacity() * (size_of::<K>() + size_of::<V>() + HASH_MAP_ENTRY_OVERHEAD_BYTES)
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
impl_node_tree!(Terminator, terminators);
impl_node_tree!(Local, locals);
impl_node_tree!(Type, types);
impl_node_tree!(TypeAlias, type_aliases);
impl_node_tree!(Field, fields);
impl_node_tree!(Global, globals);
