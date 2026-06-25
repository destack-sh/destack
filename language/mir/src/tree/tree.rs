use destack_serde::Reflect;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};

use destack_core::{Arena, StringId};
use destack_source::{FileId, NodeSpanType, SourceIndex, Span};
use serde::{Deserialize, Serialize};

use crate::source::{Token, TokenType};
use crate::{
    Access, Attribute, Block, BorrowedPath, CommentSpan, DynamicLayout, DynamicTable, ExtentSlice,
    Field, FieldSpan, FlagSlice, FloatType, Function, FunctionHeaderSpans, Global, IndexSlice,
    Instruction, Layout, LayoutId, Lifetime, LifetimeParameter, LifetimeTerm, Local, LocalNodeId,
    MemoryAccessKind, Metadata, Node, NodeType, Nullability, Origin, OriginTable, Path, Projection,
    ReferenceKind, Space, SwitchCase, SwitchCaseSlice, TensorConvolutionDimensionNumbers,
    TensorConvolutionWindow, TensorDotDimensionNumbers, TensorGatherDimensionNumbers,
    TensorImmediate, TensorImmediateId, TensorScatterDimensionNumbers, Terminator, Type, TypeAlias,
    TypeDeclarationSpans, TypeId, TypeLineage, TypeMetadata, TypedValueSpan, Value, ValueSlice,
    VirtualTable,
};

#[inline]
fn empty_source_span() -> Span {
    Span::empty(FileId::new(0))
}

/// Dense metadata for one MIR node id.
#[derive(Debug, Copy, Clone, Serialize, Deserialize, Reflect)]
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

/// MIR tree for a single unit.
#[derive(Clone, Serialize, Deserialize, Reflect)]
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
    /// How each pass-created node came to be.
    pub(crate) origin_by_node_id: OriginTable,

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

    /// Lifetime parameters keyed by type node.
    pub(crate) lifetimes_by_type: HashMap<LocalNodeId<Type>, Vec<LifetimeParameter>>,

    // externalized instruction payloads
    /// Flat buffer of MIR values.
    pub(crate) values: Vec<Value>,
    /// Flat buffer of instruction indices.
    pub(crate) indices: Vec<u32>,
    /// Flat buffer of instruction extents.
    pub(crate) extents: Vec<u64>,
    /// Flat buffer of instruction flags.
    pub(crate) flags: Vec<u8>,
    /// Flat buffer of switch cases.
    pub(crate) switch_cases: Vec<SwitchCase>,
    /// Structured tensor immediates.
    pub(crate) tensor_immediates: Vec<TensorImmediate>,

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
            origin_by_node_id: OriginTable::default(),
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
            lifetimes_by_type: HashMap::new(),

            values: Vec::new(),
            indices: Vec::new(),
            extents: Vec::new(),
            flags: Vec::new(),
            switch_cases: Vec::new(),
            tensor_immediates: Vec::new(),
            metadata: Metadata::default(),
        }
    }

    /// Return whether an instruction has ordered memory access metadata.
    pub fn instruction_has_atomic_ordering(&self, instruction: LocalNodeId<Instruction>) -> bool {
        // atomic instructions carry ordering on the instruction
        if matches!(
            self.get(instruction),
            Instruction::AtomicLoad { .. }
                | Instruction::AtomicStore { .. }
                | Instruction::AtomicCompareExchange { .. }
                | Instruction::AtomicRmw { .. }
                | Instruction::AtomicFence { .. }
        ) {
            return true;
        }

        // read access metadata for this instruction
        let Some(accesses) = self.metadata.memory.memory_accesses(instruction) else {
            return false;
        };

        // check for ordered or fenced accesses
        accesses.iter().any(|access| {
            access.ordering.is_some()
                || access.flags.is_some()
                || matches!(access.kind, MemoryAccessKind::Fence)
        })
    }

    /// Return whether an instruction requires exact memory access behavior.
    pub fn instruction_requires_exact_access(&self, instruction: LocalNodeId<Instruction>) -> bool {
        // ordered instructions must preserve exact access behavior
        if matches!(
            self.get(instruction),
            Instruction::AtomicLoad { .. }
                | Instruction::AtomicStore { .. }
                | Instruction::AtomicCompareExchange { .. }
                | Instruction::AtomicRmw { .. }
                | Instruction::AtomicFence { .. }
                | Instruction::BarrierWrite { .. }
        ) {
            return true;
        }

        // read memory access metadata for the instruction
        let Some(accesses) = self.metadata.memory.memory_accesses(instruction) else {
            return false;
        };

        // require exact behavior for volatile, ordered, or fenced operations
        accesses.iter().any(|access| {
            access.is_volatile
                || access.ordering.is_some()
                || access.flags.is_some()
                || matches!(access.kind, MemoryAccessKind::Fence)
        })
    }

    /// Create a new tree with parsed source data.
    pub(crate) fn with_parsed_source(source_text: String, tokens: Vec<Token>) -> Self {
        let mut tree = Self::new();
        tree.source_text = Some(source_text);
        tree.tokens = tokens;
        tree
    }

    /// Substitute type-local lifetime slots with applied lifetimes.
    pub fn substitute_lifetime(&self, lifetime: &Lifetime, lifetime_args: &[Lifetime]) -> Lifetime {
        let mut terms = Vec::new();
        for term in &lifetime.terms {
            match term {
                LifetimeTerm::Static => terms.push(LifetimeTerm::Static),
                LifetimeTerm::Slot(slot) => {
                    if let Some(lifetime) = lifetime_args.get(slot.0 as usize) {
                        terms.extend(lifetime.terms.iter().copied());
                    } else {
                        terms.push(LifetimeTerm::Slot(*slot));
                    }
                }
            }
        }

        Lifetime::new(terms)
    }

    /// Return the transparent representation type.
    pub fn repr_type(&self, mut ty: TypeId) -> TypeId {
        loop {
            match self.get(ty) {
                Type::Newtype { inner, .. } | Type::WithLifetimes { base: inner, .. } => {
                    ty = *inner;
                }
                _ => return ty,
            }
        }
    }

    /// Return the explicit lifetime carried by a type.
    pub fn type_lifetime(&self, ty: TypeId) -> Option<Lifetime> {
        let mut visited = HashSet::new();

        self.type_lifetime_inner(ty, &[], &mut visited)
    }

    /// Return the explicit lifetime carried by a type under applied lifetimes.
    pub fn type_lifetime_with_lifetimes(
        &self,
        ty: TypeId,
        lifetimes: &[Lifetime],
    ) -> Option<Lifetime> {
        let mut visited = HashSet::new();

        self.type_lifetime_inner(ty, lifetimes, &mut visited)
    }

    /// Return the explicit lifetime carried by a type.
    fn type_lifetime_inner(
        &self,
        ty: TypeId,
        lifetime_args: &[Lifetime],
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
            } if !lifetime.is_empty() => Some(self.substitute_lifetime(lifetime, lifetime_args)),
            Type::Struct { fields, .. } => {
                let nested_lifetimes = fields.iter().filter_map(|field| {
                    let field = self.get(*field);
                    self.type_lifetime_inner(field.ty, lifetime_args, visited)
                });

                Some(Lifetime::new(
                    nested_lifetimes.flat_map(|lifetime| lifetime.terms.into_iter()),
                ))
                .filter(|lifetime| !lifetime.is_empty())
            }
            Type::Newtype { inner, .. } => self.type_lifetime_inner(*inner, lifetime_args, visited),
            Type::Dynamic { constraint } => {
                self.type_lifetime_inner(*constraint, lifetime_args, visited)
            }
            Type::WithLifetimes { base, lifetimes } => {
                self.type_lifetime_inner(*base, lifetimes, visited)
            }
            Type::Uninit { value } => self.type_lifetime_inner(*value, lifetime_args, visited),
            Type::Variant {
                tag,
                storage,
                cases,
                ..
            } => {
                let tag = self.type_lifetime_inner(*tag, lifetime_args, visited);
                let storage = self.type_lifetime_inner(*storage, lifetime_args, visited);
                let nested_lifetimes = cases
                    .iter()
                    .filter_map(|case| self.type_lifetime_inner(case.ty, lifetime_args, visited));

                Some(Lifetime::new(
                    tag.into_iter()
                        .chain(storage)
                        .chain(nested_lifetimes)
                        .flat_map(|lifetime| lifetime.terms.into_iter()),
                ))
                .filter(|lifetime| !lifetime.is_empty())
            }
            Type::Tuple { elements, .. } => {
                let nested_lifetimes = elements.iter().filter_map(|element| {
                    self.type_lifetime_inner(*element, lifetime_args, visited)
                });

                Some(Lifetime::new(
                    nested_lifetimes.flat_map(|lifetime| lifetime.terms.into_iter()),
                ))
                .filter(|lifetime| !lifetime.is_empty())
            }
            Type::FixedArray { element, .. }
            | Type::Slice { element, .. }
            | Type::Vector { element, .. }
            | Type::Tensor { element, .. }
            | Type::TensorView { element, .. }
            | Type::Atomic { value: element } => {
                self.type_lifetime_inner(*element, lifetime_args, visited)
            }
            _ => None,
        }
    }

    /// Return whether a type may contain borrowed references.
    pub fn type_contains_borrowed_refs(&self, ty: TypeId) -> bool {
        let ty = self.get(ty);
        if ty.is_borrowed_reference() {
            return true;
        }

        match ty {
            Type::Struct { fields, .. } => fields.iter().any(|field| {
                let field = self.get(*field);
                self.type_contains_borrowed_refs(field.ty)
            }),
            Type::Newtype { inner, .. } => self.type_contains_borrowed_refs(*inner),
            Type::Dynamic { constraint } => self.type_contains_borrowed_refs(*constraint),
            Type::WithLifetimes { base, .. } => self.type_contains_borrowed_refs(*base),
            Type::Uninit { value } => self.type_contains_borrowed_refs(*value),
            Type::Variant {
                tag,
                storage,
                cases,
                ..
            } => {
                self.type_contains_borrowed_refs(*tag)
                    || self.type_contains_borrowed_refs(*storage)
                    || cases
                        .iter()
                        .any(|case| self.type_contains_borrowed_refs(case.ty))
            }
            Type::Tuple { elements, .. } => elements
                .iter()
                .any(|element| self.type_contains_borrowed_refs(*element)),
            Type::FixedArray { element, .. }
            | Type::Slice { element, .. }
            | Type::Vector { element, .. }
            | Type::Tensor { element, .. }
            | Type::TensorView { element, .. }
            | Type::Atomic { value: element } => self.type_contains_borrowed_refs(*element),
            _ => false,
        }
    }

    /// Return borrowed reference-like paths carried by one type.
    pub fn type_borrowed_paths(&self, ty: TypeId) -> Vec<BorrowedPath> {
        self.type_borrowed_paths_with_lifetimes(ty, &[], false)
    }

    /// Return borrowed paths used for source tracking.
    pub fn type_borrowed_source_paths(&self, ty: TypeId) -> Vec<BorrowedPath> {
        self.type_borrowed_paths_with_lifetimes(ty, &[], true)
    }

    /// Return borrowed reference-like paths carried by one type under applied lifetimes.
    pub fn type_borrowed_paths_with_lifetimes(
        &self,
        ty: TypeId,
        lifetimes: &[Lifetime],
        is_empty_included: bool,
    ) -> Vec<BorrowedPath> {
        let mut borrowed_paths = Vec::new();

        self.collect_type_borrowed_paths(
            ty,
            lifetimes,
            is_empty_included,
            Path::root(),
            &mut borrowed_paths,
        );

        borrowed_paths
    }

    /// Collect borrowed paths carried by one type into an output vector.
    fn collect_type_borrowed_paths(
        &self,
        ty: TypeId,
        lifetimes: &[Lifetime],
        is_empty_included: bool,
        path: Path,
        borrowed_paths: &mut Vec<BorrowedPath>,
    ) {
        match self.get(ty) {
            // record borrowed reference-like leaves
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
            } => {
                let lifetime = self.substitute_lifetime(lifetime, lifetimes);
                if is_empty_included || !lifetime.is_empty() {
                    borrowed_paths.push(BorrowedPath { path, lifetime });
                }
            }
            // descend into named fields
            Type::Struct { fields, .. } => {
                for (index, field) in fields.iter().enumerate() {
                    let field = self.get(*field);
                    let path = path.clone().with_projection(Projection::Field {
                        index: index as u32,
                    });

                    self.collect_type_borrowed_paths(
                        field.ty,
                        lifetimes,
                        is_empty_included,
                        path,
                        borrowed_paths,
                    );
                }
            }
            // descend into positional fields
            Type::Tuple { elements, .. } => {
                for (index, element) in elements.iter().enumerate() {
                    let path = path.clone().with_projection(Projection::Field {
                        index: index as u32,
                    });

                    self.collect_type_borrowed_paths(
                        *element,
                        lifetimes,
                        is_empty_included,
                        path,
                        borrowed_paths,
                    );
                }
            }
            // substitute outer lifetime arguments
            Type::WithLifetimes { base, lifetimes } => {
                self.collect_type_borrowed_paths(
                    *base,
                    lifetimes,
                    is_empty_included,
                    path,
                    borrowed_paths,
                );
            }
            // descend through transparent storage wrappers
            Type::Newtype { inner, .. }
            | Type::Dynamic { constraint: inner }
            | Type::Uninit { value: inner }
            | Type::Atomic { value: inner } => {
                self.collect_type_borrowed_paths(
                    *inner,
                    lifetimes,
                    is_empty_included,
                    path,
                    borrowed_paths,
                );
            }
            // descend into each variant payload shape
            Type::Variant { cases, .. } => {
                for case in cases {
                    let path = path.clone().with_projection(Projection::Variant {
                        tag: case.tag.clone(),
                    });

                    self.collect_type_borrowed_paths(
                        case.ty,
                        lifetimes,
                        is_empty_included,
                        path,
                        borrowed_paths,
                    );
                }
            }
            // collapse indexed containers to any-element paths
            Type::FixedArray { element, .. }
            | Type::Slice { element, .. }
            | Type::Vector { element, .. }
            | Type::Tensor { element, .. }
            | Type::TensorView { element, .. } => {
                let path = path.with_projection(Projection::AnyElement);

                self.collect_type_borrowed_paths(
                    *element,
                    lifetimes,
                    is_empty_included,
                    path,
                    borrowed_paths,
                );
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
        self.origin_by_node_id.append();
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
        self.origin_by_node_id.append();
        self.source_index.append(empty_source_span());

        LocalNodeId::new(global_id)
    }

    /// Insert a node derived from an existing MIR node.
    pub fn insert_derived<T>(&mut self, node: T, from: u32, derivation: StringId) -> LocalNodeId<T>
    where
        T: Node,
        Self: TreeImpl<T>,
    {
        let id = self.insert(node);
        let index = self.node_index(id.id);
        self.origin_by_node_id
            .set(index, Origin::one(derivation, from));

        id
    }

    /// Insert a synthesized node with no single origin.
    pub fn insert_synthetic<T>(&mut self, node: T, derivation: StringId) -> LocalNodeId<T>
    where
        T: Node,
        Self: TreeImpl<T>,
    {
        let id = self.insert(node);
        let index = self.node_index(id.id);
        self.origin_by_node_id
            .set(index, Origin::synthetic(derivation));

        id
    }

    /// Set the origin for one node.
    #[inline]
    pub fn set_origin(&mut self, id: u32, origin: Origin) {
        let index = self.node_index(id);
        self.origin_by_node_id.set(index, origin);
    }

    /// Return the origin for one node.
    #[inline]
    pub fn origin(&self, id: u32) -> Option<&Origin> {
        self.origin_by_node_id.get(self.node_index(id))
    }

    /// Get the DIR source for a node, walking MIR derivation parents.
    pub fn dir_source(&self, id: u32) -> Option<u32> {
        // follow primary parents until a lowered node carries the DIR edge,
        // stopping at segment-foreign ids this tree cannot resolve
        let mut current = id;
        while self.has_node_id(current) {
            if let Some(source) = self.source_id_by_node_id[self.node_index(current)] {
                return Some(source);
            }

            let origin = self.origin_by_node_id.get(self.node_index(current))?;
            current = origin.parent()?;
        }

        None
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

    /// Replace a type node and update the primitive type cache.
    pub fn set_type(&mut self, type_id: LocalNodeId<Type>, ty: Type) -> Type {
        let local_id = self.node_local_id(type_id.id);
        let old = std::mem::replace(self.types.get_mut(local_id), ty);

        // remove stale primitive entries for overwritten placeholders
        self.metadata
            .types
            .primitive_types
            .retain(|_, cached_type| *cached_type != type_id);

        // record the new primitive shape when applicable
        if let Some(primitive) = TypeMetadata::primitive_type(self.types.get(local_id)) {
            self.metadata
                .types
                .record_primitive_type(type_id, primitive);
        }

        old
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

    /// Return lifetime parameters declared by one type.
    pub fn type_lifetimes(&self, ty: LocalNodeId<Type>) -> &[LifetimeParameter] {
        self.lifetimes_by_type
            .get(&ty)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Set lifetime parameters declared by one type.
    pub fn set_type_lifetimes(&mut self, ty: LocalNodeId<Type>, lifetimes: Vec<LifetimeParameter>) {
        if lifetimes.is_empty() {
            self.lifetimes_by_type.remove(&ty);
        } else {
            self.lifetimes_by_type.insert(ty, lifetimes);
        }
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

        unreachable!("missing boolean type id in MIR primitive type cache");
    }

    /// Return the void type id.
    pub fn void_type(&self) -> LocalNodeId<Type> {
        // use the primitive type cache when available
        if let Some(type_id) = self.metadata.types.void_type()
            && matches!(self.get(type_id), Type::Void)
        {
            return type_id;
        }

        // fall back to a structural lookup
        if let Some(type_id) = self.find_type_by_predicate(|ty| matches!(ty, Type::Void)) {
            return type_id;
        }

        unreachable!("missing void type id in MIR primitive type cache");
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

        unreachable!("missing type descriptor type id in MIR primitive type cache");
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

        unreachable!("missing type id type in MIR primitive type cache");
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

        unreachable!("missing isize type id in MIR primitive type cache");
    }

    /// Return lineage metadata for a type when present.
    pub fn type_lineage(&self, ty: LocalNodeId<Type>) -> Option<&TypeLineage> {
        self.metadata.types.lineage(ty)
    }

    /// Return the layout id for a type when present.
    pub fn type_layout_id(&self, ty: LocalNodeId<Type>) -> Option<LayoutId> {
        self.metadata.layouts.layout_id(ty)
    }

    /// Return the concrete layout for a type when present.
    pub fn type_layout(&self, ty: LocalNodeId<Type>) -> Option<&Layout> {
        let layout_id = self.type_layout_id(ty)?;
        Some(self.metadata.layouts.table.layout(layout_id))
    }

    /// Return the type descriptor global for a type when present.
    pub fn type_descriptor_global(&self, ty: LocalNodeId<Type>) -> Option<LocalNodeId<Global>> {
        self.metadata.types.descriptor_global(ty)
    }

    /// Return the virtual table metadata for a type when present.
    pub fn type_virtual_table(&self, ty: LocalNodeId<Type>) -> Option<&VirtualTable> {
        self.metadata.dispatch.virtual_table(ty)
    }

    /// Return the dynamic table for a concrete type and constraint when present.
    pub fn type_dynamic_table(
        &self,
        concrete: LocalNodeId<Type>,
        constraint: LocalNodeId<Type>,
    ) -> Option<&DynamicTable> {
        self.metadata.dispatch.dynamic_table(concrete, constraint)
    }

    /// Return the display name for a type when present.
    pub fn type_display_name(&self, ty: LocalNodeId<Type>) -> Option<destack_core::StringId> {
        self.metadata.types.display_name(ty)
    }

    /// Return the dynamic layout when present.
    pub fn dynamic_layout(&self, constraint: LocalNodeId<Type>) -> Option<&DynamicLayout> {
        self.metadata.dispatch.dynamic_layout(constraint)
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

        unreachable!("missing usize type id in MIR primitive type cache");
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

        unreachable!("missing int type id for width {width} signed {signed}");
    }

    /// Return a float type id for format.
    pub fn float_type(&self, format: FloatType) -> LocalNodeId<Type> {
        // use the primitive type cache when available
        if let Some(type_id) = self.metadata.types.float_type(format) {
            return type_id;
        }

        // fall back to a structural lookup
        if let Some(type_id) = self.find_type_by_predicate(
            |ty| matches!(ty, Type::Float(float_type) if *float_type == format),
        ) {
            return type_id;
        }

        unreachable!("missing float type id for {}", format.label());
    }

    /// Return the canonical storage type for the hidden environment field in one function.
    pub fn function_environment_type(&self) -> LocalNodeId<Type> {
        let void_type = if let Some(type_id) = self.metadata.types.void_type() {
            type_id
        } else if let Some(type_id) = self.find_type_by_predicate(|ty| matches!(ty, Type::Void)) {
            type_id
        } else {
            unreachable!("missing void type for function environment storage");
        };

        if let Some(type_id) = self.find_type_by_predicate(|ty| {
            matches!(
                ty,
                Type::Reference {
                    kind: ReferenceKind::Managed,
                    space: Space::Local,
                    access: Access::Mutable,
                    pointee,
                    nullability: Nullability::Null,
                    ..
                } if *pointee == void_type
            )
        }) {
            return type_id;
        }

        unreachable!("missing canonical function environment storage type");
    }

    /// Ensure the canonical storage type for the hidden environment field in one function.
    pub fn ensure_function_environment_type(&mut self) -> LocalNodeId<Type> {
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
                    nullability: Nullability::Null,
                    ..
                } if *pointee == void_type
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
            pointee: void_type,
            nullability: Nullability::Null,
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
                unreachable!("unsupported pointer size {pointer_bytes} bytes");
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
        let local_id = self.node_local_id(id.id);
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
        let local_id = self.node_local_id(id.id);
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
            .unwrap_or_else(|| unreachable!("MIR node id {node_id} is before this tree"));
        let index = index as usize;
        if index >= self.node_index_by_node_id.len() {
            unreachable!("MIR node id {node_id} is outside this tree");
        }

        index
    }

    /// Return the local arena id for one untyped node id.
    #[inline]
    pub(crate) fn node_local_id(&self, id: u32) -> u32 {
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

    /// Resolve the source span for one node through direct spans and MIR origins.
    pub fn source_span_by_id(&self, id: u32) -> Option<Span> {
        let mut current = id;
        let mut remaining = self.node_count();

        // walk primary origins until a lowered or parsed source span appears
        while remaining > 0 && self.has_node_id(current) {
            if let Some(span) = self.get_span_by_id(current) {
                return Some(span);
            }

            let origin = self.origin_by_node_id.get(self.node_index(current))?;
            current = origin.parent()?;
            remaining -= 1;
        }

        None
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
            .unwrap_or_else(|| unreachable!("MIR tree has no parsed source text"));

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

    /// Add values to the value buffer and return a `ValueSlice`.
    #[inline]
    pub fn add_values(&mut self, values: &[Value]) -> ValueSlice {
        let start = self.values.len() as u32;
        let count = values.len() as u16;
        self.values.extend_from_slice(values);
        ValueSlice::new(start, count)
    }

    /// Get values from the value buffer by slice.
    #[inline]
    pub fn get_values(&self, slice: ValueSlice) -> &[Value] {
        let start = slice.start as usize;
        let end = start + slice.count as usize;
        &self.values[start..end]
    }

    /// Add indices to the immediate buffer and return an `IndexSlice`.
    #[inline]
    pub fn add_indices(&mut self, values: &[u32]) -> IndexSlice {
        let start = self.indices.len() as u32;
        let count = values.len() as u16;
        self.indices.extend_from_slice(values);
        IndexSlice::new(start, count)
    }

    /// Get indices from the immediate buffer by slice.
    #[inline]
    pub fn get_indices(&self, slice: IndexSlice) -> &[u32] {
        let start = slice.start as usize;
        let end = start + slice.count as usize;
        &self.indices[start..end]
    }

    /// Add extents to the immediate buffer and return an `ExtentSlice`.
    #[inline]
    pub fn add_extents(&mut self, values: &[u64]) -> ExtentSlice {
        let start = self.extents.len() as u32;
        let count = values.len() as u16;
        self.extents.extend_from_slice(values);
        ExtentSlice::new(start, count)
    }

    /// Get extents from the immediate buffer by slice.
    #[inline]
    pub fn get_extents(&self, slice: ExtentSlice) -> &[u64] {
        let start = slice.start as usize;
        let end = start + slice.count as usize;
        &self.extents[start..end]
    }

    /// Add flags to the immediate buffer and return a `FlagSlice`.
    #[inline]
    pub fn add_flags(&mut self, values: &[u8]) -> FlagSlice {
        let start = self.flags.len() as u32;
        let count = values.len() as u16;
        self.flags.extend_from_slice(values);
        FlagSlice::new(start, count)
    }

    /// Get flags from the immediate buffer by slice.
    #[inline]
    pub fn get_flags(&self, slice: FlagSlice) -> &[u8] {
        let start = slice.start as usize;
        let end = start + slice.count as usize;
        &self.flags[start..end]
    }

    /// Add switch cases to the switch case buffer and return a `SwitchCaseSlice`.
    #[inline]
    pub fn add_switch_cases(&mut self, cases: &[SwitchCase]) -> SwitchCaseSlice {
        let start = self.switch_cases.len() as u32;
        let count = cases.len() as u16;
        self.switch_cases.extend_from_slice(cases);
        SwitchCaseSlice::new(start, count)
    }

    /// Get switch cases from the switch case buffer by slice.
    #[inline]
    pub fn get_switch_cases(&self, slice: SwitchCaseSlice) -> &[SwitchCase] {
        let start = slice.start as usize;
        let end = start + slice.count as usize;
        &self.switch_cases[start..end]
    }

    /// Add one tensor immediate and return its id.
    #[inline]
    pub fn add_tensor_immediate(&mut self, immediate: TensorImmediate) -> TensorImmediateId {
        let id = self.tensor_immediates.len() as u32;
        self.tensor_immediates.push(immediate);
        TensorImmediateId::new(id)
    }

    /// Get one tensor immediate by id.
    #[inline]
    pub fn get_tensor_immediate(&self, id: TensorImmediateId) -> &TensorImmediate {
        &self.tensor_immediates[id.id() as usize]
    }

    /// Store tensor dot dimension numbers as a tensor immediate.
    pub fn add_tensor_dot_immediate(
        &mut self,
        dimensions: TensorDotDimensionNumbers,
    ) -> TensorImmediateId {
        let lhs_batch = self.add_indices(&dimensions.lhs_batch);
        let rhs_batch = self.add_indices(&dimensions.rhs_batch);
        let lhs_contracting = self.add_indices(&dimensions.lhs_contracting);
        let rhs_contracting = self.add_indices(&dimensions.rhs_contracting);

        self.add_tensor_immediate(TensorImmediate::Dot {
            lhs_batch,
            rhs_batch,
            lhs_contracting,
            rhs_contracting,
        })
    }

    /// Store tensor convolution dimensions and window parameters as a tensor immediate.
    pub fn add_tensor_convolution_immediate(
        &mut self,
        dimensions: TensorConvolutionDimensionNumbers,
        window: TensorConvolutionWindow,
        feature_group_count: u32,
        batch_group_count: u32,
    ) -> TensorImmediateId {
        let input_spatial = self.add_indices(&dimensions.input_spatial);
        let kernel_spatial = self.add_indices(&dimensions.kernel_spatial);
        let output_spatial = self.add_indices(&dimensions.output_spatial);
        let strides = self.add_extents(&window.strides);
        let padding_low = self.add_extents(&window.padding_low);
        let padding_high = self.add_extents(&window.padding_high);
        let lhs_dilation = self.add_extents(&window.lhs_dilation);
        let rhs_dilation = self.add_extents(&window.rhs_dilation);
        let window_reversal = window
            .window_reversal
            .into_iter()
            .map(u8::from)
            .collect::<Vec<_>>();
        let window_reversal = self.add_flags(&window_reversal);

        self.add_tensor_immediate(TensorImmediate::Convolution {
            input_batch: dimensions.input_batch,
            input_feature: dimensions.input_feature,
            input_spatial,
            kernel_input_feature: dimensions.kernel_input_feature,
            kernel_output_feature: dimensions.kernel_output_feature,
            kernel_spatial,
            output_batch: dimensions.output_batch,
            output_feature: dimensions.output_feature,
            output_spatial,
            strides,
            padding_low,
            padding_high,
            lhs_dilation,
            rhs_dilation,
            window_reversal,
            feature_group_count,
            batch_group_count,
        })
    }

    /// Store tensor gather dimension numbers as a tensor immediate.
    pub fn add_tensor_gather_immediate(
        &mut self,
        dimensions: TensorGatherDimensionNumbers,
        slice_sizes: &[u32],
    ) -> TensorImmediateId {
        let offset_dims = self.add_indices(&dimensions.offset_dims);
        let collapsed_slice_dims = self.add_indices(&dimensions.collapsed_slice_dims);
        let start_index_map = self.add_indices(&dimensions.start_index_map);
        let slice_sizes = self.add_indices(slice_sizes);

        self.add_tensor_immediate(TensorImmediate::Gather {
            offset_dims,
            collapsed_slice_dims,
            start_index_map,
            index_vector_dim: dimensions.index_vector_dim,
            slice_sizes,
        })
    }

    /// Store tensor scatter dimension numbers as a tensor immediate.
    pub fn add_tensor_scatter_immediate(
        &mut self,
        dimensions: TensorScatterDimensionNumbers,
    ) -> TensorImmediateId {
        let update_window_dims = self.add_indices(&dimensions.update_window_dims);
        let inserted_window_dims = self.add_indices(&dimensions.inserted_window_dims);
        let scatter_dims_to_operand_dims =
            self.add_indices(&dimensions.scatter_dims_to_operand_dims);

        self.add_tensor_immediate(TensorImmediate::Scatter {
            update_window_dims,
            inserted_window_dims,
            scatter_dims_to_operand_dims,
            index_vector_dim: dimensions.index_vector_dim,
        })
    }

    /// Set a node in-place, preserving its source and origin.
    pub fn set<T>(&mut self, id: LocalNodeId<T>, replacement: T)
    where
        T: Node,
        Self: TreeImpl<T>,
    {
        *self.get_mut(id) = replacement;
    }

    /// Derive a replacement node from its previous contents.
    ///
    /// Returns the ID of the preserved original node.
    pub fn derive<T>(
        &mut self,
        id: LocalNodeId<T>,
        replacement: T,
        derivation: StringId,
    ) -> LocalNodeId<T>
    where
        T: Node + Clone,
        Self: TreeImpl<T>,
    {
        // preserve the original payload, DIR source, and origin at a new id
        let original = self.get(id).clone();
        let source_id = self.get_source(id.id);
        let preserved_id = self.insert(original);
        if let Some(source_id) = source_id {
            self.set_source(preserved_id.id, source_id);
        }
        let index = self.node_index(id.id);
        if let Some(origin) = self.origin_by_node_id.take(index) {
            let preserved_index = self.node_index(preserved_id.id);
            self.origin_by_node_id.set(preserved_index, origin);
        }

        // derive the slot from the preserved original
        self.set(id, replacement);
        let index = self.node_index(id.id);
        self.origin_by_node_id
            .set(index, Origin::one(derivation, preserved_id.id));

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
