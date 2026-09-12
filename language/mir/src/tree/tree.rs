use destack_serde::Reflect;
use std::fmt::{Debug, Formatter};

use destack_core::{Arena, FxIndexMap, FxIndexSet, StringId};
use destack_source::{FileId, NodeSpanType, SourceIndex, Span};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::source::{Token, TokenType};
use crate::{
    Attribute, Block, BorrowedPath, CommentSpan, ExtentSlice, Field, FieldSpan, FlagSlice,
    FloatType, Function, FunctionHeaderSpans, GenericArgument, Global, IndexSlice, Instruction,
    Lifetime, Local, LocalNodeId, Node, NodeIndexEntry, NodeType, Path, Projection, Provenance,
    ProvenanceTable, Reference, Space, Static, StaticId, Storage, Substitution, SwitchCase,
    SwitchCaseSlice, Symbol, Terminator, Type, TypeDeclaration, TypeDeclarationSpans, TypeId,
    TypedValueSpan, Value, ValueSlice,
};

/// MIR tree for a single unit.
#[derive(Clone, Serialize, Deserialize, Reflect)]
pub struct Tree {
    /// The first global node id stored in this tree.
    pub(crate) first_global_id: u32,
    /// The next global node id to allocate.
    pub(crate) next_global_id: u32,
    /// Dense local id and node type by node id.
    pub(crate) node_index_by_node_id: Vec<NodeIndexEntry>,
    /// Maps global node id → attached attributes.
    pub(crate) attributes_by_node_id: FxIndexMap<u32, Vec<Attribute>>,
    /// DIR source id keyed by MIR node id.
    pub(crate) source_id_by_node_id: Vec<Option<u32>>,
    /// How each pass-created node came to be.
    pub(crate) provenance_by_node_id: ProvenanceTable,

    /// Source ranges and anchors for parsed MIR node ownership.
    pub source_index: SourceIndex,
    /// The parsed MIR source text.
    pub(crate) source_text: Option<String>,

    /// The full parsed token stream.
    pub(crate) tokens: Vec<Token>,
    /// Leading comment spans keyed by global node id.
    pub(crate) leading_comment_spans_by_node_id: Vec<Option<Span>>,
    /// Parsed attribute spans keyed by global node id.
    pub(crate) attribute_spans_by_node_id: FxIndexMap<u32, Vec<Span>>,
    /// Parsed declaration keyword spans keyed by global node id.
    pub(crate) keyword_spans_by_node_id: FxIndexMap<u32, Span>,
    /// Parsed function parameter spans keyed by global node id.
    pub(crate) function_parameter_spans_by_node_id: FxIndexMap<u32, Vec<TypedValueSpan>>,
    /// Parsed function header spans keyed by global node id.
    pub(crate) function_header_spans_by_node_id: FxIndexMap<u32, FunctionHeaderSpans>,
    /// Parsed type field spans keyed by global node id.
    pub(crate) type_field_spans_by_node_id: FxIndexMap<u32, Vec<FieldSpan>>,
    /// Parsed type declaration spans keyed by global node id.
    pub(crate) type_declaration_spans_by_node_id: FxIndexMap<u32, TypeDeclarationSpans>,

    // node arenas
    pub(crate) functions: Arena<Function>,
    pub(crate) blocks: Arena<Block>,
    pub(crate) instructions: Arena<Instruction>,
    pub(crate) terminators: Arena<Terminator>,
    pub(crate) locals: Arena<Local>,
    pub(crate) types: Arena<Type>,
    pub(crate) type_declarations: Arena<TypeDeclaration>,
    pub(crate) fields: Arena<Field>,
    pub(crate) globals: Arena<Global>,
    /// Interned compile-time values.
    pub(crate) statics: Arena<Static>,
    /// Interned space join expressions.
    pub(crate) space_joins: Vec<Vec<Space>>,
    /// Interned storage join expressions.
    pub(crate) storage_joins: Vec<Vec<Storage>>,

    /// Interned type ids grouped by structural hash.
    pub(crate) type_index: FxIndexMap<u64, SmallVec<[TypeId; 1]>>,
    /// Type declarations indexed by persistent symbol.
    pub(crate) declared_types: FxIndexMap<Symbol, LocalNodeId<TypeDeclaration>>,
    /// Structural field ids grouped by hash.
    pub(crate) field_index: FxIndexMap<u64, SmallVec<[LocalNodeId<Field>; 1]>>,
    /// Canonical compile-time values grouped by structural hash.
    pub(crate) static_index: FxIndexMap<u64, SmallVec<[StaticId; 1]>>,

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
            .field("type_declarations", &self.type_declarations.len())
            .field("fields", &self.fields.len())
            .field("globals", &self.globals.len())
            .field("statics", &self.statics.len())
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
            attributes_by_node_id: FxIndexMap::with_capacity_and_hasher(
                capacity,
                Default::default(),
            ),
            source_id_by_node_id: Vec::with_capacity(capacity),
            provenance_by_node_id: ProvenanceTable::default(),
            source_index: SourceIndex::with_capacity(capacity),
            source_text: None,
            tokens: Vec::new(),
            leading_comment_spans_by_node_id: Vec::new(),
            attribute_spans_by_node_id: FxIndexMap::default(),
            keyword_spans_by_node_id: FxIndexMap::default(),
            function_parameter_spans_by_node_id: FxIndexMap::default(),
            function_header_spans_by_node_id: FxIndexMap::default(),
            type_field_spans_by_node_id: FxIndexMap::default(),
            type_declaration_spans_by_node_id: FxIndexMap::default(),

            functions: Arena::new(),
            blocks: Arena::new(),
            instructions: Arena::new(),
            terminators: Arena::new(),
            locals: Arena::new(),
            types: Arena::new(),
            type_declarations: Arena::new(),
            fields: Arena::new(),
            globals: Arena::new(),
            statics: Arena::new(),
            space_joins: Vec::new(),
            storage_joins: Vec::new(),
            type_index: FxIndexMap::default(),
            declared_types: FxIndexMap::default(),
            field_index: FxIndexMap::default(),
            static_index: FxIndexMap::default(),

            values: Vec::new(),
            indices: Vec::new(),
            extents: Vec::new(),
            flags: Vec::new(),
            switch_cases: Vec::new(),
        }
    }

    /// Create a tree with parsed source metadata.
    pub(crate) fn with_parsed_source(source_text: String, tokens: Vec<Token>) -> Self {
        let mut tree = Self::new();
        tree.source_text = Some(source_text);
        tree.tokens = tokens;
        tree
    }

    /// Return the declared representation or the type itself.
    pub fn type_definition(&self, ty: TypeId) -> &Type {
        match self.get(ty) {
            Type::Declaration { declaration } => match self.get(*declaration).definition {
                Some(definition) => self.get(definition),
                None => self.get(ty),
            },
            definition => definition,
        }
    }

    /// Return the transparent representation type.
    pub fn repr_type(&mut self, mut ty: TypeId) -> TypeId {
        loop {
            ty = Substitution::resolve(ty, self);
            match self.get(ty) {
                Type::Newtype { inner, .. } => ty = *inner,
                _ => return ty,
            }
        }
    }

    /// Return the initialized storage type beneath transparent forms.
    pub fn storage_type(&mut self, mut ty: TypeId) -> TypeId {
        loop {
            ty = Substitution::resolve(ty, self);
            match self.get(ty) {
                Type::Uninit { value } | Type::ManuallyDrop { value } => ty = *value,
                Type::Newtype { inner, .. } => ty = *inner,
                _ => return ty,
            }
        }
    }

    /// Return the heap storage carried by one managed allocation result.
    pub fn managed_storage(&mut self, ty: TypeId) -> Option<Storage> {
        let ty = self.storage_type(ty);
        let ty = self.get(ty);
        let storage = ty.reference_storage()?;

        (ty.reference_kind() == Some(Reference::Managed) && storage.heap_space().is_some())
            .then_some(storage)
    }

    /// Return the extents one type stores across its regions.
    pub fn type_lifetime(&self, ty: TypeId) -> Option<Lifetime> {
        let mut visited = FxIndexSet::default();

        self.type_lifetime_inner(ty, &mut visited)
    }

    /// Return the explicit lifetime carried by a type.
    fn type_lifetime_inner(
        &self,
        ty: TypeId,
        visited: &mut FxIndexSet<LocalNodeId<Type>>,
    ) -> Option<Lifetime> {
        if !visited.insert(ty) {
            return None;
        }

        // inspect nominal arguments without expanding recursive definitions
        let lifetime = match self.type_definition(ty) {
            Type::Application { arguments, .. } => {
                let lifetimes = arguments.iter().filter_map(|argument| match argument {
                    GenericArgument::Type(ty) => self.type_lifetime_inner(*ty, visited),
                    GenericArgument::Region { lifetime, .. } => Some(lifetime.clone()),
                    _ => None,
                });

                Some(Lifetime::new(
                    lifetimes.flat_map(|lifetime| lifetime.extents),
                ))
                .filter(|lifetime| !lifetime.is_empty())
            }
            Type::Dynamic {
                kind: Reference::Borrowed,
                lifetime,
                ..
            }
            | Type::Function {
                kind: Reference::Borrowed,
                lifetime,
                ..
            } if !lifetime.is_empty() => Some(lifetime.clone()),
            Type::Reference {
                kind,
                lifetime,
                pointee,
                ..
            } => {
                let own = (matches!(kind, Reference::Borrowed)).then(|| lifetime.clone());
                let nested = self.type_lifetime_inner(*pointee, visited);
                let terms = own
                    .into_iter()
                    .chain(nested)
                    .flat_map(|lifetime| lifetime.extents);

                Some(Lifetime::new(terms)).filter(|lifetime| !lifetime.is_empty())
            }
            Type::Slice {
                kind,
                lifetime,
                element,
                ..
            } => {
                let own = (matches!(kind, Reference::Borrowed)).then(|| lifetime.clone());
                let nested = self.type_lifetime_inner(*element, visited);
                let terms = own
                    .into_iter()
                    .chain(nested)
                    .flat_map(|lifetime| lifetime.extents);

                Some(Lifetime::new(terms)).filter(|lifetime| !lifetime.is_empty())
            }
            Type::Struct { fields, .. } => {
                let nested_lifetimes = fields.iter().filter_map(|field| {
                    let field = self.get(*field);
                    self.type_lifetime_inner(field.ty, visited)
                });

                Some(Lifetime::new(
                    nested_lifetimes.flat_map(|lifetime| lifetime.extents.into_iter()),
                ))
                .filter(|lifetime| !lifetime.is_empty())
            }
            Type::Newtype { inner, .. } => self.type_lifetime_inner(*inner, visited),
            Type::Uninit { value } | Type::ManuallyDrop { value } => {
                self.type_lifetime_inner(*value, visited)
            }
            Type::Variant {
                discriminant,
                cases,
                ..
            } => {
                let discriminant = self.type_lifetime_inner(*discriminant, visited);
                let nested_lifetimes = cases
                    .iter()
                    .filter_map(|case| self.type_lifetime_inner(case.ty, visited));

                Some(Lifetime::new(
                    discriminant
                        .into_iter()
                        .chain(nested_lifetimes)
                        .flat_map(|lifetime| lifetime.extents.into_iter()),
                ))
                .filter(|lifetime| !lifetime.is_empty())
            }
            Type::Tuple { elements, .. } => {
                let nested_lifetimes = elements
                    .iter()
                    .filter_map(|element| self.type_lifetime_inner(*element, visited));

                Some(Lifetime::new(
                    nested_lifetimes.flat_map(|lifetime| lifetime.extents.into_iter()),
                ))
                .filter(|lifetime| !lifetime.is_empty())
            }
            Type::FixedArray { element, .. } | Type::Vector { element, .. } => {
                self.type_lifetime_inner(*element, visited)
            }
            _ => None,
        };
        visited.swap_remove(&ty);

        lifetime
    }

    /// Return whether a type may contain borrowed references.
    pub fn type_contains_borrowed_refs(&self, ty: TypeId) -> bool {
        let mut visited = FxIndexSet::default();

        self.type_contains_borrowed_refs_inner(ty, &mut visited)
    }

    /// Return whether a type path contains borrowed references.
    fn type_contains_borrowed_refs_inner(
        &self,
        ty: TypeId,
        visited: &mut FxIndexSet<LocalNodeId<Type>>,
    ) -> bool {
        if !visited.insert(ty) {
            return false;
        }

        let type_id = ty;
        let ty = self.type_definition(ty);
        if ty.is_borrowed_reference() {
            visited.swap_remove(&type_id);

            return true;
        }

        // inspect nominal arguments without expanding recursive definitions
        let contains = match ty {
            Type::Application { arguments, .. } => {
                arguments.iter().any(|argument| match argument {
                    GenericArgument::Type(ty) => {
                        self.type_contains_borrowed_refs_inner(*ty, visited)
                    }
                    GenericArgument::Region { .. } => true,
                    _ => false,
                })
            }
            Type::Struct { fields, .. } => fields.iter().any(|field| {
                let field = self.get(*field);
                self.type_contains_borrowed_refs_inner(field.ty, visited)
            }),
            Type::Newtype { inner, .. } => self.type_contains_borrowed_refs_inner(*inner, visited),
            Type::Uninit { value } | Type::ManuallyDrop { value } => {
                self.type_contains_borrowed_refs_inner(*value, visited)
            }
            Type::Variant {
                discriminant,
                cases,
                ..
            } => {
                self.type_contains_borrowed_refs_inner(*discriminant, visited)
                    || cases
                        .iter()
                        .any(|case| self.type_contains_borrowed_refs_inner(case.ty, visited))
            }
            Type::Tuple { elements, .. } => elements
                .iter()
                .any(|element| self.type_contains_borrowed_refs_inner(*element, visited)),
            Type::Reference {
                pointee: element, ..
            }
            | Type::FixedArray { element, .. }
            | Type::Slice { element, .. }
            | Type::Vector { element, .. } => {
                self.type_contains_borrowed_refs_inner(*element, visited)
            }
            _ => false,
        };
        visited.swap_remove(&type_id);

        contains
    }

    /// Return borrowed reference-like paths carried by one type.
    pub fn type_borrowed_paths(&mut self, ty: TypeId) -> Vec<BorrowedPath> {
        self.collect_borrowed_paths(ty, false)
    }

    /// Return borrowed paths used for origin tracking.
    pub fn type_origin_paths(&mut self, ty: TypeId) -> Vec<BorrowedPath> {
        self.collect_borrowed_paths(ty, true)
    }

    /// Return borrowed reference-like paths carried by one type.
    fn collect_borrowed_paths(&mut self, ty: TypeId, is_tracking: bool) -> Vec<BorrowedPath> {
        let mut borrowed_paths = Vec::new();

        self.collect_type_borrowed_paths(ty, is_tracking, Path::root(), &mut borrowed_paths);

        borrowed_paths
    }

    /// Collect borrowed paths carried by one type into an output vector.
    fn collect_type_borrowed_paths(
        &mut self,
        ty: TypeId,
        is_tracking: bool,
        path: Path,
        borrowed_paths: &mut Vec<BorrowedPath>,
    ) {
        let resolved = Substitution::resolve(ty, self);
        let definition = self.get(resolved).clone();

        match &definition {
            // record reference-like leaves
            Type::Dynamic {
                kind,
                lifetime,
                access,
                ..
            }
            | Type::Reference {
                kind,
                lifetime,
                access,
                ..
            }
            | Type::Slice {
                kind,
                lifetime,
                access,
                ..
            }
            | Type::Function {
                kind,
                lifetime,
                access,
                ..
            } => {
                let lifetime = lifetime.clone();
                // track managed handles and empty borrows only for origin
                let is_included = match kind {
                    Reference::Borrowed => is_tracking || !lifetime.is_empty(),
                    Reference::Managed => is_tracking,
                    Reference::Unique | Reference::Raw => false,
                };
                if is_included {
                    borrowed_paths.push(BorrowedPath {
                        path,
                        lifetime,
                        access: *access,
                        kind: *kind,
                    });
                }
            }
            // descend into named fields
            Type::Struct { fields, .. } => {
                for (index, field) in fields.iter().enumerate() {
                    let ty = self.get(*field).ty;
                    let path = path.clone().with_projection(Projection::Field {
                        index: index as u32,
                    });

                    self.collect_type_borrowed_paths(ty, is_tracking, path, borrowed_paths);
                }
            }
            // descend into positional fields
            Type::Tuple { elements, .. } => {
                for (index, element) in elements.iter().enumerate() {
                    let path = path.clone().with_projection(Projection::Field {
                        index: index as u32,
                    });

                    self.collect_type_borrowed_paths(*element, is_tracking, path, borrowed_paths);
                }
            }
            // select the stored field of a newtype
            Type::Newtype { inner, .. } => {
                let path = path.with_projection(Projection::Field { index: 0 });

                self.collect_type_borrowed_paths(*inner, is_tracking, path, borrowed_paths);
            }
            // preserve paths through initialization and destruction modifiers
            Type::Uninit { value: inner } | Type::ManuallyDrop { value: inner } => {
                self.collect_type_borrowed_paths(*inner, is_tracking, path, borrowed_paths);
            }
            // descend into each possible storage shape
            Type::Variant { cases, .. } => {
                for (index, case) in cases.iter().enumerate() {
                    let path = path
                        .clone()
                        .with_projection(Projection::Variant { case: index as u32 });

                    self.collect_type_borrowed_paths(case.ty, is_tracking, path, borrowed_paths);
                }
            }
            // retain each reference path within repeated elements
            Type::FixedArray { element, .. } | Type::Vector { element, .. } => {
                let path = path.with_projection(Projection::Elements);
                self.collect_type_borrowed_paths(*element, is_tracking, path, borrowed_paths);
            }
            _ => {}
        }
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

    /// Insert a synthesized mutable node.
    pub fn insert<T>(&mut self, node: T) -> LocalNodeId<T>
    where
        T: Node,
        Self: TreeMut<T>,
    {
        let local_id = <Self as TreeMut<T>>::allocate(self, node);

        self.insert_node(local_id)
    }

    /// Insert a node into the tree with a source DIR node id for diagnostics.
    pub fn insert_from<T>(&mut self, node: T, source_dir_id: u32) -> LocalNodeId<T>
    where
        T: Node,
        Self: TreeMut<T>,
    {
        let id = self.insert(node);
        self.set_source(id.id, source_dir_id);

        id
    }

    /// Record one already allocated node in the shared node index.
    pub(crate) fn insert_node<T: Node>(&mut self, local_id: u32) -> LocalNodeId<T> {
        let global_id = self.next_global_id;
        self.next_global_id += 1;

        self.node_index_by_node_id
            .push(NodeIndexEntry::new(local_id, T::TYPE));
        self.source_id_by_node_id.push(None);
        self.provenance_by_node_id.append();
        self.source_index.append(Span::empty(FileId::new(0)));

        LocalNodeId::new(global_id)
    }

    /// Insert a node derived from an existing MIR node.
    pub fn insert_derived<T>(&mut self, node: T, from: u32, derivation: StringId) -> LocalNodeId<T>
    where
        T: Node,
        Self: TreeMut<T>,
    {
        let id = self.insert(node);
        let index = self.node_index(id.id);
        self.provenance_by_node_id
            .set(index, Provenance::one(derivation, from));

        id
    }

    /// Insert a synthesized node with no single origin.
    pub fn insert_synthetic<T>(&mut self, node: T, derivation: StringId) -> LocalNodeId<T>
    where
        T: Node,
        Self: TreeMut<T>,
    {
        let id = self.insert(node);
        let index = self.node_index(id.id);
        self.provenance_by_node_id
            .set(index, Provenance::synthetic(derivation));

        id
    }

    /// Set the origin for one node.
    #[inline]
    pub fn set_provenance(&mut self, id: u32, origin: Provenance) {
        let index = self.node_index(id);
        self.provenance_by_node_id.set(index, origin);
    }

    /// Return the origin for one node.
    #[inline]
    pub fn provenance(&self, id: u32) -> Option<&Provenance> {
        self.provenance_by_node_id.get(self.node_index(id))
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

            let origin = self.provenance_by_node_id.get(self.node_index(current))?;
            current = origin.parent()?;
        }

        None
    }

    /// Return the boolean type id.
    pub fn boolean_type(&self) -> LocalNodeId<Type> {
        if let Some(type_id) = self.find_type(&Type::Boolean) {
            return type_id;
        }

        unreachable!("missing boolean type id in MIR tree");
    }

    /// Return the character type id.
    pub fn character_type(&self) -> LocalNodeId<Type> {
        if let Some(type_id) = self.find_type(&Type::Character) {
            return type_id;
        }

        unreachable!("missing character type id in MIR tree");
    }

    /// Return the void type id.
    pub fn void_type(&self) -> LocalNodeId<Type> {
        if let Some(type_id) = self.find_type(&Type::Void) {
            return type_id;
        }

        unreachable!("missing void type id in MIR tree");
    }

    /// Return the isize type id.
    pub fn isize_type(&self) -> LocalNodeId<Type> {
        if let Some(type_id) = self.find_type(&Type::Isize) {
            return type_id;
        }

        unreachable!("missing isize type id in MIR tree");
    }

    /// Return the usize type id.
    pub fn usize_type(&self) -> LocalNodeId<Type> {
        if let Some(type_id) = self.find_type(&Type::Usize) {
            return type_id;
        }

        unreachable!("missing usize type id in MIR tree");
    }

    /// Return an integer type id for width and signedness.
    pub fn int_type(&self, width: u16, signed: bool) -> LocalNodeId<Type> {
        if let Some(type_id) = self.find_type(&Type::Int {
            width,
            is_signed: signed,
        }) {
            return type_id;
        }

        unreachable!("missing int type id for width {width} signed {signed}");
    }

    /// Return a float type id for format.
    pub fn float_type(&self, format: FloatType) -> LocalNodeId<Type> {
        if let Some(type_id) = self.find_type(&Type::Float(format)) {
            return type_id;
        }

        unreachable!("missing float type id for {}", format.label());
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

    /// Get a mutable reference to a node by id.
    #[inline]
    pub fn get_mut<T>(&mut self, id: LocalNodeId<T>) -> &mut T
    where
        T: Node,
        Self: TreeMut<T>,
    {
        let local_id = self.node_local_id(id.id);
        <Self as TreeMut<T>>::get_mut(self, local_id)
    }

    /// Replace one function block's instruction list.
    pub fn replace_block_instructions(
        &mut self,
        function: LocalNodeId<Function>,
        block: LocalNodeId<Block>,
        instructions: Vec<LocalNodeId<Instruction>>,
    ) {
        self.get_mut(function)
            .replace_block_instruction_index(block, &instructions);
        self.get_mut(block).instructions = instructions;
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

    /// Return the local node index for one global node id.
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

    /// Return the source DIR node for one MIR node.
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

    /// Append one attribute to a node.
    pub fn push_attribute<T>(&mut self, id: LocalNodeId<T>, attribute: Attribute)
    where
        T: Node,
    {
        let mut attributes = self.attributes(id).to_vec();
        attributes.push(attribute);
        self.set_attributes(id, attributes);
    }

    /// Set the attributes for a node.
    #[inline]
    pub(crate) fn set_attributes<T>(&mut self, id: LocalNodeId<T>, attributes: Vec<Attribute>)
    where
        T: Node,
    {
        if attributes.is_empty() {
            self.attributes_by_node_id.shift_remove(&id.id);
        } else {
            self.attributes_by_node_id.insert(id.id, attributes);
        }
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

            let origin = self.provenance_by_node_id.get(self.node_index(current))?;
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
            self.attribute_spans_by_node_id.shift_remove(&id.id);
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
            self.function_parameter_spans_by_node_id
                .shift_remove(&id.id);
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

    /// Return the parsed field spans for one type declaration.
    pub fn type_field_spans(&self, id: LocalNodeId<TypeDeclaration>) -> &[FieldSpan] {
        self.type_field_spans_by_node_id
            .get(&id.id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Set the parsed field spans for one type declaration.
    pub fn set_type_field_spans(
        &mut self,
        id: LocalNodeId<TypeDeclaration>,
        spans: Vec<FieldSpan>,
    ) {
        if spans.is_empty() {
            self.type_field_spans_by_node_id.shift_remove(&id.id);
        } else {
            self.type_field_spans_by_node_id.insert(id.id, spans);
        }
    }

    /// Return the parsed type declaration spans for one type declaration.
    pub fn type_declaration_spans(
        &self,
        id: LocalNodeId<TypeDeclaration>,
    ) -> Option<&TypeDeclarationSpans> {
        self.type_declaration_spans_by_node_id.get(&id.id)
    }

    /// Set the parsed type declaration spans for one type declaration.
    pub fn set_type_declaration_spans(
        &mut self,
        id: LocalNodeId<TypeDeclaration>,
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

    /// Set a node in-place, preserving its source and origin.
    pub fn set<T>(&mut self, id: LocalNodeId<T>, replacement: T)
    where
        T: Node,
        Self: TreeMut<T>,
    {
        *self.get_mut(id) = replacement;
    }

    /// Derive a replacement while preserving the original node.
    pub fn derive<T>(
        &mut self,
        id: LocalNodeId<T>,
        replacement: T,
        derivation: StringId,
    ) -> LocalNodeId<T>
    where
        T: Node + Clone,
        Self: TreeMut<T>,
    {
        // preserve the original payload, DIR source, and origin at a new id
        let original = self.get(id).clone();
        let source_id = self.get_source(id.id);
        let preserved_id = self.insert(original);
        if let Some(source_id) = source_id {
            self.set_source(preserved_id.id, source_id);
        }
        let index = self.node_index(id.id);
        if let Some(origin) = self.provenance_by_node_id.take(index) {
            let preserved_index = self.node_index(preserved_id.id);
            self.provenance_by_node_id.set(preserved_index, origin);
        }

        // derive the slot from the preserved original
        self.set(id, replacement);
        let index = self.node_index(id.id);
        self.provenance_by_node_id
            .set(index, Provenance::one(derivation, preserved_id.id));

        preserved_id
    }
}

/// Trait for mapping node types to arenas.
pub trait TreeImpl<T: Node> {
    /// Get a node from the arena by local index.
    fn get(tree: &Tree, idx: u32) -> &T;
}

/// Mutable MIR node arena operations.
pub trait TreeMut<T: Node>: TreeImpl<T> {
    /// Allocate a node in the arena and return its local index.
    fn allocate(tree: &mut Tree, node: T) -> u32;
    /// Get a mutable node from the arena by local index.
    fn get_mut(tree: &mut Tree, idx: u32) -> &mut T;
}

macro_rules! impl_tree {
    ($ty:ty, $field:ident) => {
        impl TreeImpl<$ty> for Tree {
            #[inline]
            fn get(tree: &Tree, idx: u32) -> &$ty {
                tree.$field.get(idx)
            }
        }

        impl TreeMut<$ty> for Tree {
            #[inline]
            fn allocate(tree: &mut Tree, node: $ty) -> u32 {
                tree.$field.allocate(node)
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
impl_tree!(Global, globals);

impl_tree!(TypeDeclaration, type_declarations);

impl TreeImpl<Type> for Tree {
    #[inline]
    fn get(tree: &Tree, idx: u32) -> &Type {
        tree.types.get(idx)
    }
}

impl TreeImpl<Field> for Tree {
    #[inline]
    fn get(tree: &Tree, idx: u32) -> &Field {
        tree.fields.get(idx)
    }
}
