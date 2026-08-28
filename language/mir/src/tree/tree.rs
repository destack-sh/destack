use destack_serde::Reflect;
use std::fmt::{Debug, Formatter};

use destack_core::{Arena, FxIndexMap, FxIndexSet};
use destack_source::{NodeSpanType, ProvenanceId, ProvenanceJournal, Span};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use super::intern::{TypeEntry, TypeIndexKey};
use crate::source::{Document, Token, TokenType};
use crate::{
    Access, Attribute, Block, BorrowedPath, CommentSpan, ExtentSlice, Field, FieldId, FieldSpan,
    FlagSlice, FloatType, Function, FunctionHeaderSpans, Global, IndexSlice, Instruction, Lifetime,
    LifetimeParameter, LifetimeTerm, Local, LocalNodeId, Node, NodeIndexEntry, NodeType,
    Nullability, Path, Projection, ReferenceKind, Static, StaticId, Storage, SwitchCase,
    SwitchCaseSlice, Terminator, Type, TypeDeclaration, TypeDeclarationSpans, TypeId,
    TypedValueSpan, Value, ValueSlice, VariantCase, VariantCaseId,
};

/// MIR tree for a single unit.
#[derive(Clone, Serialize, Deserialize, Reflect)]
pub struct Tree {
    /// Dense local id and node type by node id.
    pub(crate) node_index_by_node_id: Vec<NodeIndexEntry>,
    /// Attached attributes keyed by MIR node id.
    pub(crate) attributes_by_node_id: FxIndexMap<u32, Vec<Attribute>>,
    /// The provenance of each MIR node.
    pub(crate) provenance: Vec<ProvenanceId>,
    /// The provenance of each declared field or variant case.
    pub(crate) type_member_provenance: Vec<Vec<ProvenanceId>>,

    /// The textual MIR document when this tree came from text.
    pub(super) document: Option<Document>,

    // node arenas
    pub(crate) functions: Arena<Function>,
    pub(crate) blocks: Arena<Block>,
    pub(crate) instructions: Arena<Instruction>,
    pub(crate) terminators: Arena<Terminator>,
    pub(crate) locals: Arena<Local>,
    pub(crate) types: Arena<TypeEntry>,
    pub(crate) type_declarations: Arena<TypeDeclaration>,
    pub(crate) globals: Arena<Global>,
    /// Interned compile-time values.
    pub(crate) statics: Arena<Static>,

    /// Canonical type ids grouped by structural hash or identified symbol.
    pub(crate) type_index: FxIndexMap<TypeIndexKey, SmallVec<[TypeId; 1]>>,
    /// Cycle type ids keyed by canonical serialization.
    pub(crate) canonical_index: FxIndexMap<String, TypeId>,
    /// Canonical compile-time values grouped by structural hash.
    pub(crate) static_index: FxIndexMap<u64, SmallVec<[StaticId; 1]>>,

    /// Lifetime parameters keyed by canonical type id.
    pub(crate) lifetimes_by_type: FxIndexMap<TypeId, Vec<LifetimeParameter>>,

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
            .field("globals", &self.globals.len())
            .field("statics", &self.statics.len())
            .field(
                "tokens",
                &self
                    .document
                    .as_ref()
                    .map_or(0, |document| document.tokens.len()),
            )
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
            node_index_by_node_id: Vec::with_capacity(capacity),
            attributes_by_node_id: FxIndexMap::with_capacity_and_hasher(
                capacity,
                Default::default(),
            ),
            provenance: Vec::with_capacity(capacity),
            type_member_provenance: Vec::new(),
            document: None,

            functions: Arena::new(),
            blocks: Arena::new(),
            instructions: Arena::new(),
            terminators: Arena::new(),
            locals: Arena::new(),
            types: Arena::new(),
            type_declarations: Arena::new(),
            globals: Arena::new(),
            statics: Arena::new(),
            type_index: FxIndexMap::default(),
            canonical_index: FxIndexMap::default(),
            static_index: FxIndexMap::default(),
            lifetimes_by_type: FxIndexMap::default(),

            values: Vec::new(),
            indices: Vec::new(),
            extents: Vec::new(),
            flags: Vec::new(),
            switch_cases: Vec::new(),
        }
    }

    /// Create a tree for one textual MIR document.
    pub(crate) fn with_document(text: String, tokens: Vec<Token>) -> Self {
        let mut tree = Self::new();
        tree.document = Some(Document::new(text, tokens));
        tree
    }

    /// Substitute type-local lifetime slots with applied lifetimes.
    pub fn substitute_lifetime(&self, lifetime: &Lifetime, lifetime_args: &[Lifetime]) -> Lifetime {
        let mut terms = Vec::new();
        for term in &lifetime.terms {
            match term {
                LifetimeTerm::Static => terms.push(LifetimeTerm::Static),
                LifetimeTerm::Frame => terms.push(LifetimeTerm::Frame),
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

    /// Split one optional lifetime application into its base and arguments.
    pub fn split_lifetime_application(&self, ty: TypeId) -> (TypeId, &[Lifetime]) {
        match self.ty(ty) {
            Type::Application { base, lifetimes } => (*base, lifetimes),
            _ => (ty, &[]),
        }
    }

    /// Return the transparent representation type.
    pub fn repr_type(&self, mut ty: TypeId) -> TypeId {
        loop {
            match self.ty(ty) {
                Type::Newtype { inner, .. } | Type::Application { base: inner, .. } => {
                    ty = *inner;
                }
                _ => return ty,
            }
        }
    }

    /// Return the transparent storage type.
    pub fn storage_type(&self, mut ty: TypeId) -> TypeId {
        loop {
            ty = match self.ty(ty) {
                Type::Uninit { value: base }
                | Type::Atomic { value: base }
                | Type::ManuallyDrop { value: base } => *base,
                Type::Newtype { inner, .. } => *inner,
                Type::Application { base, .. } => *base,
                _ => return ty,
            };
        }
    }

    /// Return the heap storage carried by one managed allocation result.
    pub fn managed_storage(&self, ty: TypeId) -> Option<Storage> {
        let ty = self.storage_type(ty);
        let ty = self.ty(ty);
        let storage = ty.reference_storage()?;

        (ty.reference_kind() == Some(ReferenceKind::Managed) && storage.heap_space().is_some())
            .then_some(storage)
    }

    /// Return the explicit lifetime carried by a type.
    pub fn type_lifetime(&self, ty: TypeId) -> Option<Lifetime> {
        let mut visited = FxIndexSet::default();

        self.type_lifetime_inner(ty, &[], &mut visited)
    }

    /// Return the explicit lifetime carried by a type under applied lifetimes.
    pub fn type_lifetime_with_lifetimes(
        &self,
        ty: TypeId,
        lifetimes: &[Lifetime],
    ) -> Option<Lifetime> {
        let mut visited = FxIndexSet::default();

        self.type_lifetime_inner(ty, lifetimes, &mut visited)
    }

    /// Return the explicit lifetime carried by a type.
    fn type_lifetime_inner(
        &self,
        ty: TypeId,
        lifetime_args: &[Lifetime],
        visited: &mut FxIndexSet<TypeId>,
    ) -> Option<Lifetime> {
        if !visited.insert(ty) {
            return None;
        }

        let lifetime = match self.ty(ty) {
            Type::Dynamic {
                kind: ReferenceKind::Borrowed,
                lifetime,
                ..
            }
            | Type::Function {
                kind: ReferenceKind::Borrowed,
                lifetime,
                ..
            } if !lifetime.is_empty() => Some(self.substitute_lifetime(lifetime, lifetime_args)),
            Type::Reference {
                kind,
                lifetime,
                pointee,
                ..
            } => {
                let own = (*kind == ReferenceKind::Borrowed)
                    .then(|| self.substitute_lifetime(lifetime, lifetime_args));
                let nested = self.type_lifetime_inner(*pointee, lifetime_args, visited);
                let terms = own
                    .into_iter()
                    .chain(nested)
                    .flat_map(|lifetime| lifetime.terms);

                Some(Lifetime::new(terms)).filter(|lifetime| !lifetime.is_empty())
            }
            Type::Slice {
                kind,
                lifetime,
                element,
                ..
            } => {
                let own = (*kind == ReferenceKind::Borrowed)
                    .then(|| self.substitute_lifetime(lifetime, lifetime_args));
                let nested = self.type_lifetime_inner(*element, lifetime_args, visited);
                let terms = own
                    .into_iter()
                    .chain(nested)
                    .flat_map(|lifetime| lifetime.terms);

                Some(Lifetime::new(terms)).filter(|lifetime| !lifetime.is_empty())
            }
            Type::Struct { fields, .. } => {
                let nested_lifetimes = fields
                    .iter()
                    .filter_map(|field| self.type_lifetime_inner(field.ty, lifetime_args, visited));

                Some(Lifetime::new(
                    nested_lifetimes.flat_map(|lifetime| lifetime.terms.into_iter()),
                ))
                .filter(|lifetime| !lifetime.is_empty())
            }
            Type::Newtype { inner, .. } => self.type_lifetime_inner(*inner, lifetime_args, visited),
            Type::Uninit { value } => self.type_lifetime_inner(*value, lifetime_args, visited),
            Type::Variant {
                discriminant,
                cases,
                ..
            } => {
                let discriminant = self.type_lifetime_inner(*discriminant, lifetime_args, visited);
                let nested_lifetimes = cases
                    .iter()
                    .filter_map(|case| self.type_lifetime_inner(case.ty, lifetime_args, visited));

                Some(Lifetime::new(
                    discriminant
                        .into_iter()
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
            | Type::Vector { element, .. }
            | Type::Atomic { value: element } => {
                self.type_lifetime_inner(*element, lifetime_args, visited)
            }
            Type::Application { base, lifetimes } => {
                self.type_lifetime_inner(*base, lifetimes, visited)
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
        visited: &mut FxIndexSet<TypeId>,
    ) -> bool {
        if !visited.insert(ty) {
            return false;
        }

        let type_id = ty;
        let ty = self.ty(ty);
        if ty.is_borrowed_reference() {
            visited.swap_remove(&type_id);

            return true;
        }

        let contains = match ty {
            Type::Struct { fields, .. } => fields
                .iter()
                .any(|field| self.type_contains_borrowed_refs_inner(field.ty, visited)),
            Type::Newtype { inner, .. } => self.type_contains_borrowed_refs_inner(*inner, visited),
            Type::Uninit { value } => self.type_contains_borrowed_refs_inner(*value, visited),
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
            | Type::Vector { element, .. }
            | Type::Atomic { value: element } => {
                self.type_contains_borrowed_refs_inner(*element, visited)
            }
            Type::Application { base, .. } => {
                self.type_contains_borrowed_refs_inner(*base, visited)
            }
            _ => false,
        };
        visited.swap_remove(&type_id);

        contains
    }

    /// Return borrowed reference-like paths carried by one type.
    pub fn type_borrowed_paths(&self, ty: TypeId) -> Vec<BorrowedPath> {
        self.type_borrowed_paths_with_lifetimes(ty, &[], false)
    }

    /// Return borrowed paths used for provenance tracking.
    pub fn type_provenance_paths(&self, ty: TypeId) -> Vec<BorrowedPath> {
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
        match self.ty(ty) {
            // record borrowed reference-like leaves
            Type::Dynamic {
                kind: ReferenceKind::Borrowed,
                lifetime,
                access,
                ..
            }
            | Type::Reference {
                kind: ReferenceKind::Borrowed,
                lifetime,
                access,
                ..
            }
            | Type::Slice {
                kind: ReferenceKind::Borrowed,
                lifetime,
                access,
                ..
            }
            | Type::Function {
                kind: ReferenceKind::Borrowed,
                lifetime,
                access,
                ..
            } => {
                let lifetime = self.substitute_lifetime(lifetime, lifetimes);
                if is_empty_included || !lifetime.is_empty() {
                    borrowed_paths.push(BorrowedPath {
                        path,
                        lifetime,
                        access: *access,
                    });
                }
            }
            // descend into named fields
            Type::Struct { fields, .. } => {
                for (index, field) in fields.iter().enumerate() {
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
            // descend through transparent storage wrappers
            Type::Newtype { inner, .. }
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
            // descend into each possible storage shape
            Type::Variant { cases, .. } => {
                for (index, case) in cases.iter().enumerate() {
                    let path = path
                        .clone()
                        .with_projection(Projection::Variant { case: index as u32 });

                    self.collect_type_borrowed_paths(
                        case.ty,
                        lifetimes,
                        is_empty_included,
                        path,
                        borrowed_paths,
                    );
                }
            }
            // summarize repeated element lifetimes at the container path
            Type::FixedArray { element, .. }
            | Type::Slice { element, .. }
            | Type::Vector { element, .. } => {
                let mut element_paths = Vec::new();
                self.collect_type_borrowed_paths(
                    *element,
                    lifetimes,
                    is_empty_included,
                    Path::root(),
                    &mut element_paths,
                );

                if !element_paths.is_empty() {
                    let access = element_paths
                        .iter()
                        .map(|borrowed| borrowed.access)
                        .max()
                        .unwrap_or(Access::Readonly);
                    let lifetime = Lifetime::new(
                        element_paths
                            .into_iter()
                            .flat_map(|borrowed| borrowed.lifetime.terms),
                    );
                    borrowed_paths.push(BorrowedPath {
                        path,
                        lifetime,
                        access,
                    });
                }
            }
            // substitute outer lifetime arguments
            Type::Application { base, lifetimes } => {
                self.collect_type_borrowed_paths(
                    *base,
                    lifetimes,
                    is_empty_included,
                    path,
                    borrowed_paths,
                );
            }
            _ => {}
        }
    }

    /// Insert one mutable node.
    pub fn insert<T>(&mut self, node: T, provenance: ProvenanceId) -> LocalNodeId<T>
    where
        T: Node,
        Self: TreeMut<T>,
    {
        let local_id = <Self as TreeMut<T>>::allocate(self, node);

        self.insert_node(local_id, provenance)
    }

    /// Insert one node derived from another MIR node.
    pub fn insert_from<T, U>(
        &mut self,
        node: T,
        source: LocalNodeId<U>,
        provenance: &mut ProvenanceJournal<'_>,
    ) -> LocalNodeId<T>
    where
        T: Node,
        U: Node,
        Self: TreeMut<T>,
    {
        let source = self.provenance(source.id);
        let output = provenance.derive(source);

        self.insert(node, output)
    }

    /// Record one already allocated node in the shared node index.
    pub(crate) fn insert_node<T: Node>(
        &mut self,
        local_id: u32,
        provenance: ProvenanceId,
    ) -> LocalNodeId<T> {
        let id = self.node_index_by_node_id.len() as u32;

        self.node_index_by_node_id
            .push(NodeIndexEntry::new(local_id, T::TYPE));
        self.provenance.push(provenance);

        LocalNodeId::new(id)
    }

    /// Return the provenance of one MIR node.
    #[inline]
    pub fn provenance(&self, id: u32) -> ProvenanceId {
        self.provenance[self.node_index(id)]
    }

    /// Return lifetime parameters declared by one type.
    pub fn type_lifetimes(&self, ty: TypeId) -> &[LifetimeParameter] {
        self.lifetimes_by_type
            .get(&ty)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Set lifetime parameters declared by one type.
    pub fn set_type_lifetimes(&mut self, ty: TypeId, lifetimes: Vec<LifetimeParameter>) {
        assert!(
            self.is_identified_type(ty),
            "structural MIR types cannot own lifetime parameters"
        );

        if lifetimes.is_empty() {
            self.lifetimes_by_type.shift_remove(&ty);
        } else {
            self.lifetimes_by_type.insert(ty, lifetimes);
        }
    }

    /// Return the boolean type id.
    pub fn boolean_type(&self) -> TypeId {
        if let Some(type_id) = self.find_type_by_predicate(|ty| matches!(ty, Type::Boolean)) {
            return type_id;
        }

        unreachable!("missing boolean type id in MIR tree");
    }

    /// Return the character type id.
    pub fn character_type(&self) -> TypeId {
        if let Some(type_id) = self.find_type_by_predicate(|ty| matches!(ty, Type::Character)) {
            return type_id;
        }

        unreachable!("missing character type id in MIR tree");
    }

    /// Return the void type id.
    pub fn void_type(&self) -> TypeId {
        if let Some(type_id) = self.find_type_by_predicate(|ty| matches!(ty, Type::Void)) {
            return type_id;
        }

        unreachable!("missing void type id in MIR tree");
    }

    /// Return the isize type id.
    pub fn isize_type(&self) -> TypeId {
        if let Some(type_id) = self.find_type_by_predicate(|ty| matches!(ty, Type::Isize)) {
            return type_id;
        }

        unreachable!("missing isize type id in MIR tree");
    }

    /// Return the usize type id.
    pub fn usize_type(&self) -> TypeId {
        if let Some(type_id) = self.find_type_by_predicate(|ty| matches!(ty, Type::Usize)) {
            return type_id;
        }

        unreachable!("missing usize type id in MIR tree");
    }

    /// Return an integer type id for width and signedness.
    pub fn int_type(&self, width: u16, signed: bool) -> TypeId {
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
    pub fn float_type(&self, format: FloatType) -> TypeId {
        if let Some(type_id) = self.find_type_by_predicate(
            |ty| matches!(ty, Type::Float(float_type) if *float_type == format),
        ) {
            return type_id;
        }

        unreachable!("missing float type id for {}", format.label());
    }

    /// Return the canonical storage type for the hidden environment field in one function.
    pub fn function_environment_type(&self) -> TypeId {
        let Some(void_type) = self.find_type_by_predicate(|ty| matches!(ty, Type::Void)) else {
            unreachable!("missing void type for function environment storage");
        };

        if let Some(type_id) = self.find_type_by_predicate(|ty| {
            matches!(
                ty,
                Type::Reference {
                    kind: ReferenceKind::Managed,
                    storage: Storage::LocalHeap,
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
    pub fn ensure_function_environment_type(&mut self) -> TypeId {
        let void_type =
            if let Some(type_id) = self.find_type_by_predicate(|ty| matches!(ty, Type::Void)) {
                type_id
            } else {
                self.intern_type(Type::Void)
            };

        // reuse the canonical erased environment reference when present
        if let Some(type_id) = self.find_type_by_predicate(|ty| {
            matches!(
                ty,
                Type::Reference {
                    kind: ReferenceKind::Managed,
                    storage: Storage::LocalHeap,
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
        self.intern_type(Type::Reference {
            kind: ReferenceKind::Managed,
            lifetime: Lifetime::empty(),
            storage: Storage::LocalHeap,
            access: Access::Mutable,
            pointee: void_type,
            nullability: Nullability::Null,
        })
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

    /// Return one canonical type.
    #[inline]
    pub fn ty(&self, id: TypeId) -> &Type {
        match self.types.get(id.id) {
            TypeEntry::Structural { ty } | TypeEntry::Identified { ty, .. } => ty,
            TypeEntry::Reserved { symbol } => {
                panic!("reserved MIR type {symbol:?} was read before its definition")
            }
        }
    }

    /// Return one canonical field.
    #[inline]
    pub fn field(&self, id: FieldId) -> &Field {
        let FieldId(declaration, index) = id;
        let declaration = self.get(declaration);
        let Type::Struct { fields, .. } = self.ty(declaration.ty) else {
            unreachable!("MIR field id names a non-struct type declaration");
        };

        fields
            .get(index as usize)
            .unwrap_or_else(|| unreachable!("MIR field index is outside its declaration"))
    }

    /// Return the provenance of one declared field.
    #[inline]
    pub fn field_provenance(&self, id: FieldId) -> ProvenanceId {
        let FieldId(declaration, index) = id;

        self.type_member_provenance(declaration, index)
    }

    /// Return one canonical variant case.
    #[inline]
    pub fn variant_case(&self, id: VariantCaseId) -> &VariantCase {
        let VariantCaseId(declaration, index) = id;
        let declaration = self.get(declaration);
        let Type::Variant { cases, .. } = self.ty(declaration.ty) else {
            unreachable!("MIR variant case id names a non-variant type declaration");
        };

        cases
            .get(index as usize)
            .unwrap_or_else(|| unreachable!("MIR variant case index is outside its declaration"))
    }

    /// Return the provenance of one declared variant case.
    #[inline]
    pub fn variant_case_provenance(&self, id: VariantCaseId) -> ProvenanceId {
        let VariantCaseId(declaration, index) = id;

        self.type_member_provenance(declaration, index)
    }

    /// Return the provenance of one declared type member.
    fn type_member_provenance(
        &self,
        declaration: LocalNodeId<TypeDeclaration>,
        index: u32,
    ) -> ProvenanceId {
        let declaration_index = self.node_local_id(declaration.id) as usize;

        *self.type_member_provenance[declaration_index]
            .get(index as usize)
            .unwrap_or_else(|| unreachable!("MIR type member index is outside its declaration"))
    }

    /// Find the first type id matching a predicate.
    fn find_type_by_predicate(&self, predicate: impl Fn(&Type) -> bool) -> Option<TypeId> {
        self.types.iter().enumerate().find_map(|(index, entry)| {
            let ty = match entry {
                TypeEntry::Structural { ty } | TypeEntry::Identified { ty, .. } => ty,
                TypeEntry::Reserved { .. } => return None,
            };

            predicate(ty).then_some(TypeId::new(index as u32))
        })
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
        provenance: &mut ProvenanceJournal<'_>,
    ) {
        let mut function_data = self.get(function).clone();
        function_data.replace_block_instruction_index(block, &instructions);
        self.rewrite(function, function_data, provenance);

        let mut block_data = self.get(block).clone();
        block_data.instructions = instructions;
        self.rewrite(block, block_data, provenance);
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

    /// Return the dense index for one MIR node id.
    #[inline]
    pub(crate) fn node_index(&self, node_id: u32) -> usize {
        let index = node_id as usize;
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
        (node_id as usize) < self.node_index_by_node_id.len()
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
        let source = self.document.as_ref()?;
        if !source.index.contains_node(id) {
            return None;
        }

        Some(source.index.get(id))
    }

    /// Set the parsed span for one MIR node.
    #[inline]
    pub fn set_span<T>(&mut self, id: LocalNodeId<T>, span: Span)
    where
        T: Node,
    {
        self.node_index(id.id);
        let source = self.document_mut();
        let next_id = source.index.len() as u32;

        // append the next parsed node
        if id.id == next_id {
            source.index.append(span);
        }
        // update an existing parsed node
        else if source.index.contains_node(id.id) {
            source.index.set(id.id, span);
        }
        // reject a gap in the parsed node prefix
        else {
            unreachable!("parsed MIR node {id:?} follows a node without a source span");
        }
    }

    /// Get the main source span for a MIR node when present.
    #[inline]
    pub fn get_main_span<T>(&self, id: LocalNodeId<T>) -> Option<Span>
    where
        T: Node,
    {
        self.get_main_span_by_id(id.id)
    }

    /// Get the main source span for a MIR node by raw id.
    #[inline]
    pub fn get_main_span_by_id(&self, id: u32) -> Option<Span> {
        let source = self.document.as_ref()?;
        if !source.index.contains_node(id) {
            return None;
        }

        source.index.get_main(id)
    }

    /// Set the main source span for a MIR node.
    #[inline]
    pub fn set_main_span<T>(&mut self, id: LocalNodeId<T>, span: Span, provenance: ProvenanceId)
    where
        T: Node,
    {
        self.node_index(id.id);
        self.document_mut().index.set_main(id.id, span, provenance);
    }

    /// Get one side span for a MIR node when present.
    #[inline]
    pub fn get_side_span<T>(&self, id: LocalNodeId<T>, span_type: NodeSpanType) -> Option<Span>
    where
        T: Node,
    {
        self.get_side_span_by_id(id.id, span_type)
    }

    /// Get one side span for a MIR node by raw id when present.
    #[inline]
    pub fn get_side_span_by_id(&self, id: u32, span_type: NodeSpanType) -> Option<Span> {
        let source = self.document.as_ref()?;
        if !source.index.contains_node(id) {
            return None;
        }

        source.index.get_side(id, span_type)
    }

    /// Set one side span for a MIR node.
    #[inline]
    pub fn set_side_span<T>(
        &mut self,
        id: LocalNodeId<T>,
        span_type: NodeSpanType,
        span: Span,
        provenance: ProvenanceId,
    ) where
        T: Node,
    {
        self.node_index(id.id);
        self.document_mut()
            .index
            .set_side(id.id, span_type, span, provenance);
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
        self.document
            .as_ref()?
            .leading_comments
            .get(self.node_index(id.id))
            .copied()
            .flatten()
    }

    /// Set the leading comment span for one raw node id.
    pub(crate) fn set_leading_comment_span_by_id(&mut self, id: u32, span: Span) {
        let index = self.node_index(id);
        let source = self.document_mut();

        if index >= source.leading_comments.len() {
            source.leading_comments.resize(index + 1, None);
        }

        source.leading_comments[index] = Some(span);
    }

    /// Return all parsed tokens.
    pub(crate) fn tokens(&self) -> &[Token] {
        self.document
            .as_ref()
            .map(|source| source.tokens.as_slice())
            .unwrap_or(&[])
    }

    /// Return one source slice for the provided span.
    pub(crate) fn source_text(&self, span: Span) -> &str {
        let start = span.start as usize;
        let end = span.end as usize;

        let source_text = self.parsed_text();

        &source_text[start..end]
    }

    /// Return the complete parsed source text.
    pub(crate) fn parsed_text(&self) -> &str {
        &self
            .document
            .as_ref()
            .unwrap_or_else(|| unreachable!("MIR tree has no parsed source text"))
            .text
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
        self.document
            .as_ref()
            .and_then(|source| source.attributes.get(&id.id))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Set the parsed attribute spans for one node.
    pub fn set_attribute_spans<T>(&mut self, id: LocalNodeId<T>, spans: Vec<Span>)
    where
        T: Node,
    {
        let source = self.document_mut();
        if spans.is_empty() {
            source.attributes.shift_remove(&id.id);
        } else {
            source.attributes.insert(id.id, spans);
        }
    }

    /// Return the parsed declaration keyword span for one node.
    pub fn keyword_span<T>(&self, id: LocalNodeId<T>) -> Option<Span>
    where
        T: Node,
    {
        self.document.as_ref()?.keywords.get(&id.id).copied()
    }

    /// Set the parsed declaration keyword span for one node.
    pub fn set_keyword_span<T>(&mut self, id: LocalNodeId<T>, span: Span)
    where
        T: Node,
    {
        self.document_mut().keywords.insert(id.id, span);
    }

    /// Return the parsed function parameter spans for one function.
    pub fn function_parameter_spans(&self, id: LocalNodeId<Function>) -> &[TypedValueSpan] {
        self.document
            .as_ref()
            .and_then(|source| source.function_parameters.get(&id.id))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Set the parsed function parameter spans for one function.
    pub fn set_function_parameter_spans(
        &mut self,
        id: LocalNodeId<Function>,
        spans: Vec<TypedValueSpan>,
    ) {
        let source = self.document_mut();
        if spans.is_empty() {
            source.function_parameters.shift_remove(&id.id);
        } else {
            source.function_parameters.insert(id.id, spans);
        }
    }

    /// Return the parsed function header spans for one function.
    pub fn function_header_spans(&self, id: LocalNodeId<Function>) -> Option<&FunctionHeaderSpans> {
        self.document.as_ref()?.function_headers.get(&id.id)
    }

    /// Set the parsed function header spans for one function.
    pub fn set_function_header_spans(
        &mut self,
        id: LocalNodeId<Function>,
        spans: FunctionHeaderSpans,
    ) {
        self.document_mut().function_headers.insert(id.id, spans);
    }

    /// Return the parsed field spans for one type declaration.
    pub fn type_field_spans(&self, id: LocalNodeId<TypeDeclaration>) -> &[FieldSpan] {
        self.document
            .as_ref()
            .and_then(|source| source.type_fields.get(&id.id))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Set the parsed field spans for one type declaration.
    pub fn set_type_field_spans(
        &mut self,
        id: LocalNodeId<TypeDeclaration>,
        spans: Vec<FieldSpan>,
    ) {
        let source = self.document_mut();
        if spans.is_empty() {
            source.type_fields.shift_remove(&id.id);
        } else {
            source.type_fields.insert(id.id, spans);
        }
    }

    /// Return the parsed type declaration spans for one type declaration.
    pub fn type_declaration_spans(
        &self,
        id: LocalNodeId<TypeDeclaration>,
    ) -> Option<&TypeDeclarationSpans> {
        self.document.as_ref()?.type_declarations.get(&id.id)
    }

    /// Set the parsed type declaration spans for one type declaration.
    pub fn set_type_declaration_spans(
        &mut self,
        id: LocalNodeId<TypeDeclaration>,
        spans: TypeDeclarationSpans,
    ) {
        self.document_mut().type_declarations.insert(id.id, spans);
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
        for token in self.tokens() {
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

    /// Return the mutable textual document.
    fn document_mut(&mut self) -> &mut Document {
        self.document
            .as_mut()
            .unwrap_or_else(|| unreachable!("MIR tree has no parsed source text"))
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
            .filter_map(|(index, &entry)| {
                if entry.node_type() == T::TYPE {
                    let id = LocalNodeId::new(index as u32);
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

    /// Set one node payload.
    pub fn set_payload<T>(&mut self, id: LocalNodeId<T>, replacement: T)
    where
        T: Node,
        Self: TreeMut<T>,
    {
        *self.get_mut(id) = replacement;
    }

    /// Replace one node payload and its provenance.
    pub fn replace<T>(&mut self, id: LocalNodeId<T>, replacement: T, provenance: ProvenanceId)
    where
        T: Node,
        Self: TreeMut<T>,
    {
        self.set_payload(id, replacement);
        let index = self.node_index(id.id);
        self.provenance[index] = provenance;
    }

    /// Replace one provenance and record the transform that produced it.
    pub fn rewrite<T>(
        &mut self,
        id: LocalNodeId<T>,
        replacement: T,
        provenance: &mut ProvenanceJournal<'_>,
    ) where
        T: Node,
        Self: TreeMut<T>,
    {
        let source = self.provenance(id.id);
        let output = provenance.derive(source);

        self.replace(id, replacement, output);
    }

    /// Derive and store one node's transformed provenance.
    pub fn rewrite_provenance<T>(
        &mut self,
        id: LocalNodeId<T>,
        provenance: &mut ProvenanceJournal<'_>,
    ) where
        T: Node,
    {
        let index = self.node_index(id.id);
        let source = self.provenance[index];
        let output = provenance.derive(source);

        self.provenance[index] = output;
    }

    /// Record one node as removed by the active transform.
    pub fn record_removal<T>(&self, id: LocalNodeId<T>, provenance: &mut ProvenanceJournal<'_>)
    where
        T: Node,
    {
        let source = self.provenance(id.id);
        provenance.remove(&[source]);
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

impl TreeImpl<TypeDeclaration> for Tree {
    #[inline]
    fn get(tree: &Tree, idx: u32) -> &TypeDeclaration {
        tree.type_declarations.get(idx)
    }
}
