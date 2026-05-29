use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};

use destack_core::Arena;
use destack_source::{FileId, NodeSpanType, SourceIndex, Span};
use serde::{Deserialize, Serialize};

use crate::source::{Token, TokenType};
use crate::{
    Access, ArgumentSlice, Attribute, Block, CommentSpan, Field, FieldSpan, Function,
    FunctionHeaderSpans, Global, Instruction, InterfaceShape, InterfaceTable, Layout, LayoutId,
    Lifetime, Local, LocalNodeId, Metadata, Node, NodeType, PlaceProjection, PlaceTable,
    ReferenceKind, Space, Terminator, Type, TypeAlias, TypeDeclarationSpans, TypeLineage,
    TypeMetadata, TypeReference, TypedValueSpan, ValueReference, Vtable,
};

#[inline]
fn empty_source_span() -> Span {
    Span::empty(FileId::new(0))
}

/// Dense metadata for one MIR node id.
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub(crate) struct NodeIndexEntry {
    /// The packed local id and node type.
    packed: u32,
}

impl NodeIndexEntry {
    const NODE_TYPE_SHIFT: u32 = 24;
    const LOCAL_ID_MASK: u32 = (1 << Self::NODE_TYPE_SHIFT) - 1;

    /// Pack one local id and node type into a dense entry.
    #[inline]
    pub(crate) fn new(local_id: u32, node_type: NodeType) -> Self {
        debug_assert!(
            local_id < Self::LOCAL_ID_MASK,
            "MIR node local id exceeds packed index capacity: {local_id}"
        );

        Self {
            packed: local_id | (Self::node_type_tag(node_type) << Self::NODE_TYPE_SHIFT),
        }
    }

    /// Return the local arena id for this entry.
    #[inline]
    pub(crate) fn local_id(self) -> u32 {
        self.packed & Self::LOCAL_ID_MASK
    }

    /// Return the concrete node type for this entry.
    #[inline]
    pub(crate) fn node_type(self) -> NodeType {
        match (self.packed >> Self::NODE_TYPE_SHIFT) as u8 {
            0 => NodeType::Function,
            1 => NodeType::Block,
            2 => NodeType::Instruction,
            3 => NodeType::Terminator,
            4 => NodeType::Local,
            5 => NodeType::Type,
            6 => NodeType::TypeAlias,
            7 => NodeType::Field,
            8 => NodeType::Global,
            _ => unreachable!("invalid MIR node type tag in packed node index"),
        }
    }

    /// Return the stable packed tag for one node type.
    #[inline]
    fn node_type_tag(node_type: NodeType) -> u32 {
        match node_type {
            NodeType::Function => 0,
            NodeType::Block => 1,
            NodeType::Instruction => 2,
            NodeType::Terminator => 3,
            NodeType::Local => 4,
            NodeType::Type => 5,
            NodeType::TypeAlias => 6,
            NodeType::Field => 7,
            NodeType::Global => 8,
        }
    }
}

/// MIR tree for a single module.
///
/// This is the main storage for all MIR nodes in a module. All nodes
/// (functions, blocks, instructions, locals, types) are stored in arenas
/// and referenced by `LocalNodeId<T>`.
#[derive(Clone, Serialize, Deserialize)]
pub struct Tree {
    /// The first global node id stored in this tree.
    pub(crate) first_global_id: u32,
    /// The next global node id to allocate.
    pub(crate) next_global_id: u32,
    /// Dense local id and node type metadata by node id.
    pub(crate) node_index_by_node_id: Vec<NodeIndexEntry>,
    /// Maps global node id → attached attributes.
    pub(crate) attributes_by_node_id: HashMap<u32, Vec<Attribute>>,
    /// DIR source id keyed by MIR node id.
    pub(crate) source_id_by_node_id: Vec<Option<u32>>,

    /// Source ranges and anchors for parsed MIR node ownership.
    pub source_index: SourceIndex,
    /// The parsed MIR source text.
    pub(crate) source_text: Option<String>,

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

impl Debug for Tree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tree")
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

impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
}

impl Tree {
    /// Create a new empty tree.
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Create a new tree with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            first_global_id: 0,
            next_global_id: 0,
            node_index_by_node_id: Vec::with_capacity(capacity),
            attributes_by_node_id: HashMap::with_capacity(capacity),
            source_id_by_node_id: Vec::with_capacity(capacity),
            source_index: SourceIndex::with_capacity(capacity),
            source_text: None,
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

    /// Create a new tree with parsed source data.
    pub(crate) fn with_parsed_source(source_text: String, tokens: Vec<Token>) -> Self {
        let mut tree = Self::new();
        tree.source_text = Some(source_text);
        tree.tokens = tokens;
        tree
    }

    /// Rebuild place facts for one function body.
    pub fn rebuild_function_places(&mut self, function_id: LocalNodeId<Function>) {
        let blocks = self.get(function_id).blocks.clone();
        let mut places = PlaceTable::new();

        for block_id in blocks {
            let instructions = self.get(block_id).instructions.clone();

            for instruction_id in instructions {
                let instruction = self.get(instruction_id);
                Self::record_instruction_places(instruction, &mut places);
            }
        }

        self.get_mut(function_id).places = places;
    }

    /// Infer the return lifetime for one function signature.
    pub fn infer_function_return_lifetime(&self, function_id: LocalNodeId<Function>) -> Lifetime {
        let function = self.get(function_id);
        if let Some(lifetime) = self.type_reference_lifetime(function.return_type) {
            return lifetime;
        }

        if !self.type_reference_contains_borrowed_refs(function.return_type) {
            return Lifetime::empty();
        }

        let parameter_indices =
            function
                .parameters
                .iter()
                .enumerate()
                .filter_map(|(index, parameter)| {
                    self.type_reference_contains_borrowed_refs(parameter.ty)
                        .then_some(index as u32)
                });
        let lifetime = Lifetime::parameter_set(parameter_indices);

        if lifetime.is_empty() {
            Lifetime::static_storage()
        } else {
            lifetime
        }
    }

    /// Record the inferred return lifetime for one function.
    pub fn infer_and_set_function_return_lifetime(&mut self, function_id: LocalNodeId<Function>) {
        let lifetime = self.infer_function_return_lifetime(function_id);
        self.get_mut(function_id).return_lifetime = lifetime;
    }

    /// Return the explicit lifetime carried by a type reference.
    pub fn type_reference_lifetime(&self, ty: TypeReference) -> Option<Lifetime> {
        let ty = ty.ty()?;
        let mut visited = HashSet::new();

        self.type_lifetime(ty, &mut visited)
    }

    /// Return the explicit lifetime carried by a type.
    fn type_lifetime(
        &self,
        ty: LocalNodeId<Type>,
        visited: &mut HashSet<LocalNodeId<Type>>,
    ) -> Option<Lifetime> {
        if !visited.insert(ty) {
            return None;
        }

        match self.get(ty) {
            Type::Reference {
                kind: ReferenceKind::Borrowed,
                lifetime,
                ..
            }
            | Type::Slice {
                kind: ReferenceKind::Borrowed,
                lifetime,
                ..
            }
            | Type::TensorView {
                kind: ReferenceKind::Borrowed,
                lifetime,
                ..
            } if !lifetime.is_empty() => Some(lifetime.clone()),
            Type::Struct { fields, .. } => {
                let lifetimes = fields.iter().filter_map(|field| {
                    let field = self.get(*field);
                    self.type_reference_lifetime_inner(field.ty, visited)
                });

                Some(Lifetime::new(
                    lifetimes.flat_map(|lifetime| lifetime.origins.into_iter()),
                ))
                .filter(|lifetime| !lifetime.is_empty())
            }
            Type::Newtype { inner, .. } => self.type_reference_lifetime_inner(*inner, visited),
            Type::Any { interface } => self.type_reference_lifetime_inner(*interface, visited),
            Type::Variant {
                tag,
                storage,
                cases,
                ..
            } => {
                let tag = self.type_reference_lifetime_inner(*tag, visited);
                let storage = self.type_reference_lifetime_inner(*storage, visited);
                let lifetimes = cases
                    .iter()
                    .filter_map(|case| self.type_reference_lifetime_inner(case.ty, visited));

                Some(Lifetime::new(
                    tag.into_iter()
                        .chain(storage)
                        .chain(lifetimes)
                        .flat_map(|lifetime| lifetime.origins.into_iter()),
                ))
                .filter(|lifetime| !lifetime.is_empty())
            }
            Type::Tuple { elements, .. } => {
                let lifetimes = elements
                    .iter()
                    .filter_map(|element| self.type_reference_lifetime_inner(*element, visited));

                Some(Lifetime::new(
                    lifetimes.flat_map(|lifetime| lifetime.origins.into_iter()),
                ))
                .filter(|lifetime| !lifetime.is_empty())
            }
            Type::Array { element, .. }
            | Type::Slice { element, .. }
            | Type::Vector { element, .. }
            | Type::Tensor { element, .. }
            | Type::TensorView { element, .. }
            | Type::Atomic { value: element } => {
                self.type_reference_lifetime_inner(*element, visited)
            }
            _ => None,
        }
    }

    /// Return the explicit lifetime carried by a nested type reference.
    fn type_reference_lifetime_inner(
        &self,
        ty: TypeReference,
        visited: &mut HashSet<LocalNodeId<Type>>,
    ) -> Option<Lifetime> {
        let ty = ty.ty()?;

        self.type_lifetime(ty, visited)
    }

    /// Return whether a type reference may contain borrowed references.
    pub fn type_reference_contains_borrowed_refs(&self, ty: TypeReference) -> bool {
        let Some(ty) = ty.ty() else {
            return true;
        };

        self.type_contains_borrowed_refs(ty)
    }

    /// Return whether a type may contain borrowed references.
    pub fn type_contains_borrowed_refs(&self, ty: LocalNodeId<Type>) -> bool {
        let ty = self.get(ty);
        if ty.is_borrowed_reference() {
            return true;
        }

        match ty {
            Type::Struct { fields, .. } => fields.iter().any(|field| {
                let field = self.get(*field);
                self.type_reference_contains_borrowed_refs(field.ty)
            }),
            Type::Newtype { inner, .. } => self.type_reference_contains_borrowed_refs(*inner),
            Type::Any { interface } => self.type_reference_contains_borrowed_refs(*interface),
            Type::Variant {
                tag,
                storage,
                cases,
                ..
            } => {
                self.type_reference_contains_borrowed_refs(*tag)
                    || self.type_reference_contains_borrowed_refs(*storage)
                    || cases
                        .iter()
                        .any(|case| self.type_reference_contains_borrowed_refs(case.ty))
            }
            Type::Tuple { elements, .. } => elements
                .iter()
                .any(|element| self.type_reference_contains_borrowed_refs(*element)),
            Type::Array { element, .. }
            | Type::Slice { element, .. }
            | Type::Vector { element, .. }
            | Type::Tensor { element, .. }
            | Type::TensorView { element, .. }
            | Type::Atomic { value: element } => {
                self.type_reference_contains_borrowed_refs(*element)
            }
            _ => false,
        }
    }

    /// Record place facts produced by one instruction.
    fn record_instruction_places(instruction: &Instruction, places: &mut PlaceTable) {
        match instruction {
            Instruction::LocalAddr {
                destination, local, ..
            } => {
                if let Some(value) = destination.value() {
                    places.set_local(value, *local);
                }
            }
            Instruction::GlobalAddr {
                destination,
                global,
                ..
            } => {
                if let Some(value) = destination.value() {
                    places.set_global(value, *global);
                }
            }
            Instruction::NewZeroed { destination, .. }
            | Instruction::NewUninit { destination, .. }
            | Instruction::NewComplete { destination, .. }
            | Instruction::NewSliceZeroed { destination, .. }
            | Instruction::NewSliceUninit { destination, .. }
            | Instruction::RawAllocZeroed { destination, .. }
            | Instruction::RawAllocUninit { destination, .. }
            | Instruction::FrameAllocZeroed { destination, .. }
            | Instruction::FrameAllocUninit { destination, .. }
            | Instruction::CallableEnvironment { destination } => {
                if let Some(value) = destination.value() {
                    places.set_value(value, *destination);
                }
            }
            Instruction::FieldAddr {
                destination,
                aggregate,
                index,
                ..
            } => {
                let Some(value) = destination.value() else {
                    return;
                };
                let Some(base) = aggregate.value() else {
                    return;
                };

                places.set_projection(value, base, PlaceProjection::Field { index: *index });
            }
            Instruction::ElementAddr {
                destination,
                array,
                index,
                ..
            } => {
                let Some(value) = destination.value() else {
                    return;
                };
                let Some(base) = array.value() else {
                    return;
                };

                places.set_projection(value, base, PlaceProjection::Index { index: *index });
            }
            Instruction::Slice {
                destination,
                source,
                start,
                length,
                ..
            } => {
                let Some(value) = destination.value() else {
                    return;
                };
                let Some(base) = source.value() else {
                    return;
                };

                places.set_projection(
                    value,
                    base,
                    PlaceProjection::Range {
                        start: *start,
                        length: *length,
                    },
                );
            }
            Instruction::Cast {
                destination,
                argument,
                ..
            }
            | Instruction::TensorCast {
                destination,
                tensor: argument,
            }
            | Instruction::TensorView {
                destination,
                view: argument,
                ..
            }
            | Instruction::Pin {
                destination,
                value: argument,
                ..
            } => {
                let Some(value) = destination.value() else {
                    return;
                };
                let Some(source) = argument.value() else {
                    return;
                };

                places.set_from_value(value, source);
            }
            _ => {}
        }
    }

    /// Create a new tail tree after one immutable base tree.
    pub fn from_base(base: &Tree, capacity: usize) -> Self {
        let mut tree = Self::with_capacity(capacity);
        tree.first_global_id = base.next_global_id();
        tree.next_global_id = base.next_global_id();

        tree
    }

    /// Return the first global node id stored in this tree.
    #[inline]
    pub fn first_global_id(&self) -> u32 {
        self.first_global_id
    }

    /// Return the next global node id this tree will allocate.
    #[inline]
    pub fn next_global_id(&self) -> u32 {
        self.next_global_id
    }

    /// Insert a node into the tree and return its id.
    /// The node will have no source DIR node associated (synthesized).
    pub fn insert<T>(&mut self, node: T) -> LocalNodeId<T>
    where
        T: Node,
        Self: TreeImpl<T>,
    {
        let global_id = self.next_global_id;
        self.next_global_id += 1;

        let local_id = <Self as TreeImpl<T>>::allocate(self, node);
        self.node_index_by_node_id
            .push(NodeIndexEntry::new(local_id, T::TYPE));
        self.source_id_by_node_id.push(None);
        self.source_index.append(empty_source_span());

        LocalNodeId::new(global_id)
    }

    /// Insert a node into the tree with a source DIR node id for diagnostics.
    pub fn insert_from<T>(&mut self, node: T, source_dir_id: u32) -> LocalNodeId<T>
    where
        T: Node,
        Self: TreeImpl<T>,
    {
        let global_id = self.next_global_id;
        self.next_global_id += 1;

        let local_id = <Self as TreeImpl<T>>::allocate(self, node);
        self.node_index_by_node_id
            .push(NodeIndexEntry::new(local_id, T::TYPE));
        self.source_id_by_node_id.push(Some(source_dir_id));
        self.source_index.append(empty_source_span());

        LocalNodeId::new(global_id)
    }

    /// Insert a type node into the tree and update the primitive type cache.
    pub fn insert_type(&mut self, ty: Type) -> LocalNodeId<Type> {
        // determine the primitive shape before moving the type
        let primitive = TypeMetadata::primitive_type(&ty);
        let type_id = self.insert(ty);

        // record the type in the primitive type cache
        if let Some(primitive) = primitive {
            self.metadata
                .types
                .record_primitive_type(type_id, primitive);
        }

        type_id
    }

    /// Insert a type node into the tree with a source DIR id and update the primitive type cache.
    pub fn insert_type_from(&mut self, ty: Type, source_dir_id: u32) -> LocalNodeId<Type> {
        // determine the primitive shape before moving the type
        let primitive = TypeMetadata::primitive_type(&ty);
        let type_id = self.insert_from(ty, source_dir_id);

        // record the type in the primitive type cache
        if let Some(primitive) = primitive {
            self.metadata
                .types
                .record_primitive_type(type_id, primitive);
        }

        type_id
    }

    /// Return the boolean type id.
    pub fn boolean_type(&self) -> LocalNodeId<Type> {
        // use the primitive type cache when available
        if let Some(type_id) = self.metadata.types.boolean_type() {
            return type_id;
        }

        // fall back to a structural lookup
        if let Some(type_id) = self.find_type_by_predicate(|ty| matches!(ty, Type::Boolean)) {
            return type_id;
        }

        panic!("missing boolean type id in MIR primitive type cache");
    }

    /// Return the void type id.
    pub fn void_type(&self) -> LocalNodeId<Type> {
        // use the primitive type cache when available
        if let Some(type_id) = self.metadata.types.void_type() {
            return type_id;
        }

        // fall back to a structural lookup
        if let Some(type_id) = self.find_type_by_predicate(|ty| matches!(ty, Type::Void)) {
            return type_id;
        }

        panic!("missing void type id in MIR primitive type cache");
    }

    /// Return the type descriptor type id.
    pub fn type_descriptor_type(&self) -> LocalNodeId<Type> {
        // use the primitive type cache when available
        if let Some(type_id) = self.metadata.types.type_descriptor_type() {
            return type_id;
        }

        // fall back to a structural lookup
        if let Some(type_id) = self.find_type_by_predicate(|ty| matches!(ty, Type::TypeDescriptor))
        {
            return type_id;
        }

        panic!("missing type descriptor type id in MIR primitive type cache");
    }

    /// Return the type id type id.
    pub fn type_id_type(&self) -> LocalNodeId<Type> {
        // use the primitive type cache when available
        if let Some(type_id) = self.metadata.types.type_id_type() {
            return type_id;
        }

        // fall back to a structural lookup
        if let Some(type_id) = self.find_type_by_predicate(|ty| matches!(ty, Type::TypeId)) {
            return type_id;
        }

        panic!("missing type id type in MIR primitive type cache");
    }

    /// Return the isize type id.
    pub fn isize_type(&self) -> LocalNodeId<Type> {
        // use the primitive type cache when available
        if let Some(type_id) = self.metadata.types.isize_type() {
            return type_id;
        }

        // fall back to a structural lookup
        if let Some(type_id) = self.find_type_by_predicate(|ty| matches!(ty, Type::Isize)) {
            return type_id;
        }

        panic!("missing isize type id in MIR primitive type cache");
    }

    /// Return lineage metadata for a type when present.
    pub fn type_lineage(&self, ty: LocalNodeId<Type>) -> Option<&TypeLineage> {
        self.metadata.types.lineage(ty)
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

    /// Return the type descriptor global for a type when present.
    pub fn type_descriptor_global(&self, ty: LocalNodeId<Type>) -> Option<LocalNodeId<Global>> {
        self.metadata.types.descriptor_global(ty)
    }

    /// Return the vtable metadata for a type when present.
    pub fn vtable_for_type(&self, ty: LocalNodeId<Type>) -> Option<&Vtable> {
        self.metadata.dispatch.vtable(ty)
    }

    /// Return the interface table for a concrete type and interface when present.
    pub fn interface_table_for_type(
        &self,
        concrete: LocalNodeId<Type>,
        interface: LocalNodeId<Type>,
    ) -> Option<&InterfaceTable> {
        self.metadata.dispatch.interface_table(concrete, interface)
    }

    /// Return the display name for a type when present.
    pub fn type_display_name(&self, ty: LocalNodeId<Type>) -> Option<destack_core::StringId> {
        self.metadata.types.display_name(ty)
    }

    /// Return the interface shape when present.
    pub fn interface_shape(&self, interface: LocalNodeId<Type>) -> Option<&InterfaceShape> {
        self.metadata.dispatch.interface_shape(interface)
    }

    /// Return the usize type id.
    pub fn usize_type(&self) -> LocalNodeId<Type> {
        // use the primitive type cache when available
        if let Some(type_id) = self.metadata.types.usize_type() {
            return type_id;
        }

        // fall back to a structural lookup
        if let Some(type_id) = self.find_type_by_predicate(|ty| matches!(ty, Type::Usize)) {
            return type_id;
        }

        panic!("missing usize type id in MIR primitive type cache");
    }

    /// Return an integer type id for width and signedness.
    pub fn int_type(&self, width: u16, signed: bool) -> LocalNodeId<Type> {
        // use the primitive type cache when available
        if let Some(type_id) = self.metadata.types.int_type(width, signed) {
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
        // use the primitive type cache when available
        if let Some(type_id) = self.metadata.types.float_type(width) {
            return type_id;
        }

        // fall back to a structural lookup
        if let Some(type_id) = self.find_type_by_predicate(
            |ty| matches!(ty, Type::Float(float_type) if float_type.width() == width),
        ) {
            return type_id;
        }

        panic!("missing float type id for width {width}");
    }

    /// Return the canonical storage type for the hidden environment field in one callable.
    pub fn callable_environment_type(&self) -> LocalNodeId<Type> {
        let void_type = if let Some(type_id) = self.metadata.types.void_type() {
            type_id
        } else if let Some(type_id) = self.find_type_by_predicate(|ty| matches!(ty, Type::Void)) {
            type_id
        } else {
            panic!("missing void type for callable environment storage");
        };

        if let Some(type_id) = self.find_type_by_predicate(|ty| {
            matches!(
                ty,
                Type::Reference {
                    kind: ReferenceKind::Managed,
                    space: Space::Local,
                    access: Access::Mutable,
                    pointee,
                    nullability: crate::Nullability::Null,
                    ..
                } if *pointee == TypeReference::Type(void_type)
            )
        }) {
            return type_id;
        }

        panic!("missing canonical callable environment storage type");
    }

    /// Ensure the canonical storage type for the hidden environment field in one callable.
    pub fn ensure_callable_environment_type(&mut self) -> LocalNodeId<Type> {
        // reuse or create the canonical void type
        let void_type = if let Some(type_id) = self.metadata.types.void_type() {
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
                    space: Space::Local,
                    access: Access::Mutable,
                    pointee,
                    nullability: crate::Nullability::Null,
                    ..
                } if *pointee == TypeReference::Type(void_type)
            )
        }) {
            return type_id;
        }

        // otherwise create the canonical erased environment reference
        self.insert_type(Type::Reference {
            kind: ReferenceKind::Managed,
            lifetime: Lifetime::empty(),
            space: Space::Local,
            access: Access::Mutable,
            pointee: TypeReference::Type(void_type),
            nullability: crate::Nullability::Null,
        })
    }

    /// Return module pointer size in bytes.
    pub fn pointer_bytes(&self) -> u8 {
        self.metadata.data_layout.pointer_bytes
    }

    /// Return module pointer size in bits.
    pub fn pointer_bits(&self) -> u16 {
        self.metadata.data_layout.pointer_bits()
    }

    /// Update module pointer size in bytes.
    pub fn set_pointer_bytes(&mut self, pointer_bytes: u8) {
        match pointer_bytes {
            4 | 8 => {
                self.metadata.data_layout.pointer_bytes = pointer_bytes;
            }
            _ => {
                panic!("unsupported pointer size {pointer_bytes} bytes");
            }
        }
    }

    /// Rebuild the primitive type cache from canonical type nodes.
    pub fn rebuild_primitive_types(&mut self) {
        // reset the index state
        self.metadata.types.primitive_types.clear();

        // collect primitive entries before mutating the table
        let mut type_entries = Vec::new();
        for (type_id, ty) in self.iter_nodes::<Type>() {
            let Some(primitive) = TypeMetadata::primitive_type(ty) else {
                continue;
            };
            type_entries.push((type_id, primitive));
        }

        // repopulate the primitive type cache in node order
        for (type_id, primitive) in type_entries {
            self.metadata
                .types
                .record_primitive_type(type_id, primitive);
        }
    }

    /// Get a reference to a node by id.
    #[inline]
    pub fn get<T>(&self, id: LocalNodeId<T>) -> &T
    where
        T: Node,
        Self: TreeImpl<T>,
    {
        let local_id = self.local_id_for_node_id(id.id);
        <Self as TreeImpl<T>>::get(self, local_id)
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
        Self: TreeImpl<T>,
    {
        let local_id = self.local_id_for_node_id(id.id);
        <Self as TreeImpl<T>>::get_mut(self, local_id)
    }

    /// Get the node type of a node by its raw id.
    #[inline]
    pub fn get_node_type(&self, id: u32) -> NodeType {
        self.node_index_by_node_id[self.node_index(id)].node_type()
    }

    /// Return the number of nodes stored in this tree.
    #[inline]
    pub fn node_count(&self) -> usize {
        self.node_index_by_node_id.len()
    }

    /// Return the local metadata index for one global node id.
    #[inline]
    pub(crate) fn node_index(&self, node_id: u32) -> usize {
        let index = node_id
            .checked_sub(self.first_global_id)
            .unwrap_or_else(|| panic!("MIR node id {node_id} is before this tree"));
        let index = index as usize;
        assert!(
            index < self.node_index_by_node_id.len(),
            "MIR node id {node_id} is outside this tree"
        );

        index
    }

    /// Return the local arena id for one untyped node id.
    #[inline]
    pub(crate) fn local_id_for_node_id(&self, id: u32) -> u32 {
        self.node_index_by_node_id[self.node_index(id)].local_id()
    }

    /// Return true when the node id exists in this tree.
    #[inline]
    pub fn has_node_id(&self, node_id: u32) -> bool {
        node_id >= self.first_global_id
            && ((node_id - self.first_global_id) as usize) < self.node_index_by_node_id.len()
    }

    /// Get the source DIR node id for a MIR node, if available.
    /// Returns None for synthesized nodes that don't correspond to source.
    #[inline]
    pub fn get_source(&self, id: u32) -> Option<u32> {
        self.source_id_by_node_id[self.node_index(id)]
    }

    /// Set the direct DIR origin for a MIR node.
    #[inline]
    pub fn set_source(&mut self, id: u32, source_dir_id: u32) {
        let index = self.node_index(id);
        self.source_id_by_node_id[index] = Some(source_dir_id);
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
        self.get_span_by_id(id.id)
    }

    /// Get the span for a node by raw id.
    #[inline]
    pub fn get_span_by_id(&self, id: u32) -> Option<Span> {
        let span = self.source_index.get(self.node_index(id) as u32);

        (span.end > span.start).then_some(span)
    }

    /// Set the span for a node.
    #[inline]
    pub fn set_span<T>(&mut self, id: LocalNodeId<T>, span: Span)
    where
        T: Node,
    {
        self.set_span_by_id(id.id, span);
    }

    /// Set the span for a node by raw id.
    #[inline]
    pub fn set_span_by_id(&mut self, id: u32, span: Span) {
        self.source_index.set(self.node_index(id) as u32, span);
    }

    /// Set the span for one parsed MIR node and anchor it to the parsed text.
    #[inline]
    pub fn set_text_span<T>(&mut self, id: LocalNodeId<T>, span: Span)
    where
        T: Node,
    {
        self.source_index.set(self.node_index(id.id) as u32, span);
    }

    /// Get the main source span for a MIR node when present.
    #[inline]
    pub fn get_main_span<T>(&self, id: LocalNodeId<T>) -> Option<Span>
    where
        T: Node,
    {
        self.source_index.get_main(self.node_index(id.id) as u32)
    }

    /// Get the main source span for a MIR node by raw id.
    #[inline]
    pub fn get_main_span_by_id(&self, id: u32) -> Option<Span> {
        self.source_index.get_main(self.node_index(id) as u32)
    }

    /// Set the main source span for a MIR node.
    #[inline]
    pub fn set_main_span<T>(&mut self, id: LocalNodeId<T>, span: Span)
    where
        T: Node,
    {
        self.source_index
            .set_main(self.node_index(id.id) as u32, span);
    }

    /// Get one side span for a MIR node when present.
    #[inline]
    pub fn get_side_span<T>(&self, id: LocalNodeId<T>, span_type: NodeSpanType) -> Option<Span>
    where
        T: Node,
    {
        self.source_index
            .get_side(self.node_index(id.id) as u32, span_type)
    }

    /// Get one side span for a MIR node by raw id when present.
    #[inline]
    pub fn get_side_span_by_id(&self, id: u32, span_type: NodeSpanType) -> Option<Span> {
        self.source_index
            .get_side(self.node_index(id) as u32, span_type)
    }

    /// Set one side span for a MIR node.
    #[inline]
    pub fn set_side_span<T>(&mut self, id: LocalNodeId<T>, span_type: NodeSpanType, span: Span)
    where
        T: Node,
    {
        self.source_index
            .set_side(self.node_index(id.id) as u32, span_type, span);
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
            .get(self.node_index(id.id))
            .copied()
            .flatten()
    }

    /// Set the leading comment span for one raw node id.
    pub(crate) fn set_leading_comment_span_by_id(&mut self, id: u32, span: Span) {
        let index = self.node_index(id);

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

        let source_text = self
            .source_text
            .as_deref()
            .expect("MIR tree has no parsed source text");

        &source_text[start..end]
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
            if token.ty == TokenType::Newline {
                saw_newline = true;
            }

            if token.ty == TokenType::Comment && !saw_newline {
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

            if token.ty == TokenType::Comment {
                comments.push(CommentSpan::new(token.span, self.source_text(token.span)));
            }
        }

        comments
    }

    /// Iterate over all nodes of a given type.
    pub fn iter_nodes<'a, T>(&'a self) -> impl Iterator<Item = (LocalNodeId<T>, &'a T)> + 'a
    where
        T: Node + 'a,
        Self: TreeImpl<T>,
    {
        self.node_index_by_node_id
            .iter()
            .enumerate()
            .filter_map(|(global_id, &entry)| {
                if entry.node_type() == T::TYPE {
                    let id = LocalNodeId::new(self.first_global_id + global_id as u32);
                    let local_id = entry.local_id();
                    let node = <Self as TreeImpl<T>>::get(self, local_id);
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
        Self: TreeImpl<T>,
    {
        // get original node and source
        let original = self.get(id).clone();
        let source_id = self.get_source(id.id);

        // preserve original at new ID
        let preserved_id = self.insert(original);

        // preserve source on the preserved copy
        if let Some(source_id) = source_id {
            self.set_source(preserved_id.id, source_id);
        }

        // replace in-place
        *self.get_mut(id) = replacement;

        preserved_id
    }
}

/// Trait for mapping node types to arenas.
pub trait TreeImpl<T: Node> {
    /// Allocate a node in the arena and return its local index.
    fn allocate(tree: &mut Tree, node: T) -> u32;
    /// Get a node from the arena by local index.
    fn get(tree: &Tree, idx: u32) -> &T;
    /// Get a mutable node from the arena by local index.
    fn get_mut(tree: &mut Tree, idx: u32) -> &mut T;
}

macro_rules! impl_tree {
    ($ty:ty, $field:ident) => {
        impl TreeImpl<$ty> for Tree {
            #[inline]
            fn allocate(tree: &mut Tree, node: $ty) -> u32 {
                tree.$field.allocate(node)
            }

            #[inline]
            fn get(tree: &Tree, idx: u32) -> &$ty {
                tree.$field.get(idx)
            }

            #[inline]
            fn get_mut(tree: &mut Tree, idx: u32) -> &mut $ty {
                tree.$field.get_mut(idx)
            }
        }
    };
}

impl_tree!(Function, functions);
impl_tree!(Block, blocks);
impl_tree!(Instruction, instructions);
impl_tree!(Terminator, terminators);
impl_tree!(Local, locals);
impl_tree!(Type, types);
impl_tree!(TypeAlias, type_aliases);
impl_tree!(Field, fields);
impl_tree!(Global, globals);
