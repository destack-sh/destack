use std::hash::{Hash, Hasher};
use std::sync::Arc;

use destack_core::FxIndexMap as IndexMap;
use destack_serde::Reflect;
use rustc_hash::{FxHashMap, FxHasher};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use destack_core::{Arena, PoolId, StringId, ValuePool};
use destack_source::ModuleId;

use crate::{
    BorrowForm, BorrowFormId, Form, FunctionParameterType, FunctionSignatureId,
    FunctionSignatureType, GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, LocalTypeId, MemberType,
    MemberTypeId, RefinedType, RefinedTypeId, SegmentView, Type, TypeElement, TypeFlags,
    TypeIndexSignature, TypeListId, TypeOperation, TypeOperationId, TypeProperty,
};

/// Cumulative type slots for one DIR module.
#[derive(Debug, Clone)]
pub struct TypeTable<'a> {
    /// The module id of the type table.
    pub module_id: ModuleId,
    /// The ordered type table segments.
    segments: SegmentView<'a, TypeSegment>,
}

impl TypeTable<'static> {
    /// Create a type table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<TypeSegment>>) -> Self {
        let segments = SegmentView::from_segments(segments);

        Self::from_view(segments)
    }

    /// Create a type table from one segment.
    pub fn from_segment(segment: Arc<TypeSegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl<'a> TypeTable<'a> {
    /// Create a type table from a segment view.
    pub fn from_view(segments: SegmentView<'a, TypeSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("type table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "type table segment belongs to a different module"
            );
        }

        Self {
            module_id,
            segments,
        }
    }

    /// Create a type table by appending a borrowed tail segment.
    pub fn with_tail<'b>(&'b self, tail: &'b TypeSegment) -> TypeTable<'b> {
        TypeTable::from_view(self.segments.with_tail(tail))
    }

    /// Iterate effective checked types keyed by DIR node.
    pub fn node_types(&self) -> impl Iterator<Item = (GlobalNodeIdAny, GlobalTypeId)> + '_ {
        let mut entries = IndexMap::default();

        // apply later segment values over earlier ones
        for segment in self.segments.iter() {
            for (node_id, type_id) in &segment.node_types {
                entries.insert(*node_id, *type_id);
            }
        }

        entries.into_iter()
    }

    /// Iterate solved symbol types.
    pub fn symbol_types(&self) -> impl Iterator<Item = (GlobalSymbolId, GlobalTypeId)> + '_ {
        let mut entries = IndexMap::default();

        // apply later segment values over earlier ones
        for segment in self.segments.iter() {
            for (symbol_id, type_id) in &segment.symbol_types {
                entries.insert(*symbol_id, *type_id);
            }
        }

        entries.into_iter()
    }

    /// Iterate checked reduced types.
    pub fn reduced_types(&self) -> impl Iterator<Item = (GlobalTypeId, GlobalTypeId)> + '_ {
        let mut entries = IndexMap::default();

        // apply later segment values over earlier ones
        for segment in self.segments.iter() {
            for (source, target) in &segment.reduced_types {
                entries.insert(*source, *target);
            }
        }

        entries.into_iter()
    }

    /// Get the effective checked type id for a node.
    pub fn get_node_type_id(&self, node_id: GlobalNodeIdAny) -> Option<GlobalTypeId> {
        for segment in self.segments.iter().rev() {
            if let Some(type_id) = segment.get_node_type_id(node_id) {
                return Some(type_id);
            }
        }

        None
    }

    /// Get the contextual expected type id for a node.
    pub fn get_expected_type_id(&self, node_id: GlobalNodeIdAny) -> Option<GlobalTypeId> {
        for segment in self.segments.iter().rev() {
            if let Some(type_id) = segment.get_expected_type_id(node_id) {
                return Some(type_id);
            }
        }

        None
    }

    /// Get the solved type id for a symbol.
    pub fn get_symbol_type_id(&self, symbol_id: GlobalSymbolId) -> Option<GlobalTypeId> {
        for segment in self.segments.iter().rev() {
            if let Some(type_id) = segment.get_symbol_type_id(symbol_id) {
                return Some(type_id);
            }
        }

        None
    }

    /// Get the reduced type id for a checked type.
    pub fn get_reduced_type_id(&self, type_id: GlobalTypeId) -> GlobalTypeId {
        // stored reductions are fixed points, so one lookup resolves
        for segment in self.segments.iter().rev() {
            if let Some(reduced) = segment.get_reduced_type_id(type_id) {
                return reduced;
            }
        }

        type_id
    }

    /// Get the reduced checked type id for a node.
    pub fn get_reduced_node_type_id(&self, node_id: GlobalNodeIdAny) -> Option<GlobalTypeId> {
        self.get_node_type_id(node_id)
            .map(|type_id| self.get_reduced_type_id(type_id))
    }

    /// Get the reduced checked type id for a symbol.
    pub fn get_reduced_symbol_type_id(&self, symbol_id: GlobalSymbolId) -> Option<GlobalTypeId> {
        self.get_symbol_type_id(symbol_id)
            .map(|type_id| self.get_reduced_type_id(type_id))
    }

    /// Get a type by its id.
    pub fn get_type(&self, type_id: LocalTypeId) -> Type {
        self.get_type_maybe(type_id)
            .unwrap_or_else(|| panic!("DIR type {type_id:?} is not visible"))
    }

    /// Get a type by its id when present.
    pub fn get_type_maybe(&self, type_id: LocalTypeId) -> Option<Type> {
        for segment in self.segments.iter().rev() {
            if let Some(ty) = segment.get_type_maybe(type_id) {
                return Some(ty);
            }
        }

        None
    }

    /// Get the structural flags for a type.
    pub fn get_type_flags(&self, type_id: LocalTypeId) -> TypeFlags {
        for segment in self.segments.iter().rev() {
            if let Some(flags) = segment.get_type_flags_maybe(type_id) {
                return flags;
            }
        }

        panic!("DIR type {type_id:?} is not visible")
    }

    /// Get one interned borrow form payload when present.
    pub fn borrow_form_maybe(&self, id: BorrowFormId) -> Option<&BorrowForm> {
        for segment in self.segments.iter().rev() {
            if let Some(borrow) = segment.borrow_form(id) {
                return Some(borrow);
            }
        }

        None
    }

    /// Get one interned borrow form payload.
    pub fn borrow_form(&self, id: BorrowFormId) -> &BorrowForm {
        self.borrow_form_maybe(id)
            .unwrap_or_else(|| panic!("DIR borrow form {id:?} is not visible"))
    }

    /// Get one interned member projection payload when present.
    pub fn member_maybe(&self, id: MemberTypeId) -> Option<&MemberType> {
        for segment in self.segments.iter().rev() {
            if let Some(member) = segment.member(id) {
                return Some(member);
            }
        }

        None
    }

    /// Get one interned member projection payload.
    pub fn member(&self, id: MemberTypeId) -> &MemberType {
        self.member_maybe(id)
            .unwrap_or_else(|| panic!("DIR member type {id:?} is not visible"))
    }

    /// Get one interned refined application payload when present.
    pub fn refined_maybe(&self, id: RefinedTypeId) -> Option<&RefinedType> {
        for segment in self.segments.iter().rev() {
            if let Some(refined) = segment.refined(id) {
                return Some(refined);
            }
        }

        None
    }

    /// Get one interned refined application payload.
    pub fn refined(&self, id: RefinedTypeId) -> &RefinedType {
        self.refined_maybe(id)
            .unwrap_or_else(|| panic!("DIR refined type {id:?} is not visible"))
    }

    /// Get one interned function signature payload when present.
    pub fn signature_maybe(&self, id: FunctionSignatureId) -> Option<&FunctionSignatureType> {
        for segment in self.segments.iter().rev() {
            if let Some(signature) = segment.signature(id) {
                return Some(signature);
            }
        }

        None
    }

    /// Get one interned function signature payload.
    pub fn signature(&self, id: FunctionSignatureId) -> &FunctionSignatureType {
        self.signature_maybe(id)
            .unwrap_or_else(|| panic!("DIR function signature {id:?} is not visible"))
    }

    /// Get one interned type operation payload when present.
    pub fn operation_maybe(&self, id: TypeOperationId) -> Option<&TypeOperation> {
        for segment in self.segments.iter().rev() {
            if let Some(operation) = segment.operation(id) {
                return Some(operation);
            }
        }

        None
    }

    /// Get one interned type operation payload.
    pub fn operation(&self, id: TypeOperationId) -> &TypeOperation {
        for segment in self.segments.iter().rev() {
            if let Some(operation) = segment.operation(id) {
                return operation;
            }
        }

        panic!("DIR type operation {id:?} is not visible")
    }

    /// Get one type id list.
    pub fn type_ids(&self, list: TypeListId) -> &[GlobalTypeId] {
        self.slice(list, |segment| &segment.type_ids)
    }

    /// Get one tuple element list.
    pub fn elements(&self, list: TypeListId) -> &[TypeElement] {
        self.slice(list, |segment| &segment.elements)
    }

    /// Get one shape property list.
    pub fn properties(&self, list: TypeListId) -> &[TypeProperty] {
        self.slice(list, |segment| &segment.properties)
    }

    /// Get one function parameter list.
    pub fn parameters(&self, list: TypeListId) -> &[FunctionParameterType] {
        self.slice(list, |segment| &segment.parameters)
    }

    /// Get one index signature list.
    pub fn index_signatures(&self, list: TypeListId) -> &[TypeIndexSignature] {
        self.slice(list, |segment| &segment.index_signatures)
    }

    /// Get one string list.
    pub fn strings(&self, list: TypeListId) -> &[StringId] {
        self.slice(list, |segment| &segment.strings)
    }

    /// Visit each direct child type id of one type owned by this module.
    pub fn for_each_child(&self, ty: &Type, mut visit: impl FnMut(GlobalTypeId)) {
        match ty {
            // leaves without child types
            Type::Variable(_)
            | Type::Error
            | Type::Never
            | Type::Any
            | Type::Unknown
            | Type::Void
            | Type::Null
            | Type::Undefined
            | Type::Primitive(_)
            | Type::Literal(_)
            | Type::Key(_)
            | Type::Memory(_)
            | Type::Static(_)
            | Type::Intrinsic
            | Type::Erased(_)
            | Type::Parameter(_)
            | Type::This
            | Type::Range(_)
            | Type::Reference(_) => {}

            // declaration applications
            Type::Application(instance) => {
                for child in self.type_ids(instance.arguments) {
                    visit(*child);
                }
            }
            Type::Member(member) => {
                let member = self.member(*member);
                visit(member.owner);
                for child in self.type_ids(member.arguments) {
                    visit(*child);
                }
                if let Some(qualifier) = member.qualifier {
                    visit(qualifier);
                }
            }
            Type::Variant(variant) => visit(variant.owner),
            Type::Refined(refined) => {
                let refined = self.refined(*refined);
                visit(refined.base);
                visit(refined.value);
            }

            // memory forms
            Type::Form(form) => {
                visit(form.value);
                match &form.form {
                    Form::Borrowed(borrow) => {
                        let borrow = self.borrow_form(*borrow);
                        visit(borrow.lifetime);
                        visit(borrow.access);
                    }
                    Form::Placed { place } => visit(*place),
                    Form::Managed | Form::Owned | Form::Raw | Form::Readonly => {}
                }
            }
            Type::Dynamic(dynamic) => visit(dynamic.constraint),

            // type operations resolve their interned payload
            Type::Operation(operation) => match self.operation(*operation) {
                TypeOperation::StringMapping { mapping: _, target } => visit(*target),
                TypeOperation::Conditional(conditional) => {
                    visit(conditional.left);
                    visit(conditional.right);
                    visit(conditional.then_type);
                    visit(conditional.else_type);
                }
                TypeOperation::Narrow(narrow) => {
                    visit(narrow.source);
                    visit(narrow.target);
                }
                TypeOperation::Mapped(mapped) => {
                    visit(mapped.parameter.constraint);
                    if let Some(key_remap) = mapped.parameter.key_remap {
                        visit(key_remap);
                    }
                    if let Some(modifiers_type) = mapped.parameter.modifiers_type {
                        visit(modifiers_type);
                    }
                    visit(mapped.value);
                }
                TypeOperation::Index(index) => {
                    visit(index.left);
                    visit(index.index);
                }
                TypeOperation::TemplateLiteral(template) => {
                    for child in self.type_ids(template.spans) {
                        visit(*child);
                    }
                }
                TypeOperation::Infer(infer) => {
                    if let Some(constraint) = infer.constraint {
                        visit(constraint);
                    }
                }
                TypeOperation::TypeOf(_) => {}
                TypeOperation::KeyOf(unary) => visit(unary.target),
                TypeOperation::NoInfer(unary) => visit(unary.target),
                TypeOperation::Awaited(unary) => visit(unary.target),
                TypeOperation::TryOutput { value } | TypeOperation::TryResidual { value } => {
                    visit(*value)
                }
                TypeOperation::StaticBinary(binary) => {
                    visit(binary.left);
                    visit(binary.right);
                }
                TypeOperation::StaticUnary(unary) => visit(unary.target),
            },

            // collections
            Type::Array(array) => visit(array.element),
            Type::FixedArray(array) => {
                visit(array.element);
                visit(array.count);
            }
            Type::Slice(slice) => visit(slice.element),
            Type::Tuple(tuple) => {
                for element in self.elements(tuple.elements) {
                    visit(element.ty);
                }
            }

            // structural shapes and concrete object classes
            Type::Shape(shape) | Type::Object(shape) => {
                for property in self.properties(shape.properties) {
                    if let Some(read) = property.access.read() {
                        visit(read);
                    }
                    if let Some(write) = property.access.write() {
                        visit(write);
                    }
                }
                for child in self.type_ids(shape.call_signatures) {
                    visit(*child);
                }
                for child in self.type_ids(shape.construct_signatures) {
                    visit(*child);
                }
                for signature in self.index_signatures(shape.index_signatures) {
                    visit(signature.key_type);
                    visit(signature.value_type);
                }
            }
            Type::FunctionSignature(function) => {
                let function = self.signature(*function);
                if let Some(this_parameter) = function.this_parameter {
                    visit(this_parameter);
                }
                for parameter in self.parameters(function.parameters) {
                    visit(parameter.ty);
                }
                if let Some(return_type) = function.return_type {
                    visit(return_type);
                }
            }
            Type::Function(function) => {
                visit(function.signature);
            }
            Type::FunctionPointer(function) => {
                visit(function.signature);
            }

            // algebraic composites
            Type::Union(union) => {
                for child in self.type_ids(union.elements) {
                    visit(*child);
                }
            }
            Type::Intersection(intersection) => {
                for child in self.type_ids(intersection.elements) {
                    visit(*child);
                }
            }
        }
    }

    /// Strip outer form types to reach the payload type id.
    pub fn unwrap_form_payload_type_id(&self, type_id: LocalTypeId) -> GlobalTypeId {
        let mut current = type_id;
        loop {
            match self.get_type(current) {
                Type::Form(form) if form.value.module_id == self.module_id => {
                    current = form.value.local_id;
                }
                Type::Form(form) => return form.value,
                _ => return current.into_global(self.module_id),
            }
        }
    }

    /// Iterate over all type ids.
    pub fn iter_type_ids(&self) -> impl Iterator<Item = LocalTypeId> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.iter_type_ids())
    }

    /// Get the number of types in the table.
    pub fn type_count(&self) -> u32 {
        self.segments
            .last()
            .map(|segment| segment.type_count())
            .unwrap_or(0)
    }

    /// Return the number of entries in this table.
    pub fn len(&self) -> u32 {
        self.type_count()
    }

    /// Return true when this table has no entries.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(|segment| segment.is_empty())
    }

    /// Resolve one list inside its owning segment.
    fn slice<T>(&self, list: TypeListId, pool: impl Fn(&TypeSegment) -> &ListPool<T>) -> &[T] {
        if list.is_empty() {
            return &[];
        }

        for segment in self.segments.iter().rev() {
            if let Some(slice) = pool(segment).get_maybe(list) {
                return slice;
            }
        }

        panic!("DIR type list {list:?} is not visible")
    }
}

macro_rules! pool_id {
    ($id:ty) => {
        impl PoolId for $id {
            fn from_raw(raw: u32) -> Self {
                Self(raw)
            }

            fn raw(self) -> u32 {
                self.0
            }
        }
    };
}

pool_id!(TypeOperationId);
pool_id!(FunctionSignatureId);
pool_id!(MemberTypeId);
pool_id!(RefinedTypeId);
pool_id!(BorrowFormId);

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct TypeSegment {
    /// The module id of the type segment.
    pub module_id: ModuleId,
    /// The first type id owned by this table segment.
    pub(crate) first_type_id: u32,
    /// The interned type entries.
    pub(crate) types: Arena<Type>,
    /// The structural flags per type, computed at intern time.
    pub(crate) flags: Arena<TypeFlags>,

    /// The type id lists referenced by type payloads.
    pub(crate) type_ids: ListPool<GlobalTypeId>,
    /// The tuple element lists referenced by type payloads.
    pub(crate) elements: ListPool<TypeElement>,
    /// The shape property lists referenced by type payloads.
    pub(crate) properties: ListPool<TypeProperty>,
    /// The function parameter lists referenced by type payloads.
    pub(crate) parameters: ListPool<FunctionParameterType>,
    /// The index signature lists referenced by type payloads.
    pub(crate) index_signatures: ListPool<TypeIndexSignature>,
    /// The string lists referenced by type payloads.
    pub(crate) strings: ListPool<StringId>,

    /// The interned type operation payloads.
    pub(crate) operations: ValuePool<TypeOperationId, TypeOperation>,
    /// The interned function signature payloads.
    pub(crate) signatures: ValuePool<FunctionSignatureId, FunctionSignatureType>,
    /// The interned member projection payloads.
    pub(crate) members: ValuePool<MemberTypeId, MemberType>,
    /// The interned refined application payloads.
    pub(crate) refinements: ValuePool<RefinedTypeId, RefinedType>,
    /// The interned borrow form payloads.
    pub(crate) borrows: ValuePool<BorrowFormId, BorrowForm>,

    /// The intern index from value hash to owned type slots.
    #[serde(skip)]
    index: FxHashMap<u64, SmallVec<[LocalTypeId; 1]>>,
    /// The value hash per owned type, parallel to `types`.
    #[serde(skip)]
    hashes: Vec<u64>,
    /// The sealed segment beneath this tail, interning reuses its types.
    #[serde(skip)]
    sealed: Option<Arc<TypeSegment>>,
    /// The intern index over the sealed segment's types.
    #[serde(skip)]
    sealed_index: FxHashMap<u64, SmallVec<[LocalTypeId; 1]>>,

    /// Effective checked type by node.
    pub(crate) node_types: IndexMap<GlobalNodeIdAny, GlobalTypeId>,
    /// Contextual expected type by node when different from the effective type.
    pub(crate) expected_types: IndexMap<GlobalNodeIdAny, GlobalTypeId>,
    /// Checked declaration type by symbol.
    pub(crate) symbol_types: IndexMap<GlobalSymbolId, GlobalTypeId>,
    /// Reduced type by source type.
    pub(crate) reduced_types: IndexMap<GlobalTypeId, GlobalTypeId>,
}

/// Mark of one type segment for speculative rollback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeMark {
    /// The owned type count at the mark.
    types: u32,
    /// The type id list count at the mark.
    type_ids: u32,
    /// The tuple element element count at the mark.
    elements: u32,
    /// The shape property element count at the mark.
    properties: u32,
    /// The function parameter element count at the mark.
    parameters: u32,
    /// The index signature element count at the mark.
    index_signatures: u32,
    /// The string element count at the mark.
    strings: u32,
    /// The owned operation count at the mark.
    operations: u32,
    /// The owned signature count at the mark.
    signatures: u32,
    /// The owned member count at the mark.
    members: u32,
    /// The owned refined count at the mark.
    refinements: u32,
    /// The owned borrow count at the mark.
    borrows: u32,
}

impl TypeSegment {
    /// Return the first type id owned by this segment.
    pub fn first_type_id(&self) -> u32 {
        self.first_type_id
    }

    /// Create a new type segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            first_type_id: 0,
            types: Arena::new(),
            flags: Arena::new(),
            type_ids: ListPool::new(0),
            elements: ListPool::new(0),
            properties: ListPool::new(0),
            parameters: ListPool::new(0),
            index_signatures: ListPool::new(0),
            strings: ListPool::new(0),
            operations: ValuePool::new(0),
            signatures: ValuePool::new(0),
            members: ValuePool::new(0),
            refinements: ValuePool::new(0),
            borrows: ValuePool::new(0),
            index: FxHashMap::default(),
            hashes: Vec::new(),
            sealed: None,
            sealed_index: FxHashMap::default(),
            node_types: IndexMap::default(),
            expected_types: IndexMap::default(),
            symbol_types: IndexMap::default(),
            reduced_types: IndexMap::default(),
        }
    }

    /// Create a new empty segment after an existing type table segment.
    pub fn from_base(base: &Self) -> Self {
        Self {
            module_id: base.module_id,
            first_type_id: base.type_count(),
            types: Arena::new(),
            flags: Arena::new(),
            type_ids: ListPool::new(base.type_ids.element_count()),
            elements: ListPool::new(base.elements.element_count()),
            properties: ListPool::new(base.properties.element_count()),
            parameters: ListPool::new(base.parameters.element_count()),
            index_signatures: ListPool::new(base.index_signatures.element_count()),
            strings: ListPool::new(base.strings.element_count()),
            operations: ValuePool::new(base.operations.count()),
            signatures: ValuePool::new(base.signatures.count()),
            members: ValuePool::new(base.members.count()),
            refinements: ValuePool::new(base.refinements.count()),
            borrows: ValuePool::new(base.borrows.count()),
            index: FxHashMap::default(),
            hashes: Vec::new(),
            sealed: None,
            sealed_index: FxHashMap::default(),
            node_types: IndexMap::default(),
            expected_types: IndexMap::default(),
            symbol_types: IndexMap::default(),
            reduced_types: IndexMap::default(),
        }
    }

    /// Create an empty tail whose interning reuses one sealed segment's types.
    pub fn from_sealed_base(base: Arc<TypeSegment>) -> Self {
        let mut tail = Self::from_base(&base);

        // index the sealed types so identical structures reuse their ids
        let mut sealed_index = FxHashMap::<u64, SmallVec<[LocalTypeId; 1]>>::default();
        for (slot, ty) in base.types.iter().enumerate() {
            let id = LocalTypeId::new(base.first_type_id + slot as u32);
            sealed_index.entry(fx_hash(ty)).or_default().push(id);
        }
        tail.sealed = Some(base);
        tail.sealed_index = sealed_index;

        tail
    }

    /// Intern one type whose payload lists are already interned.
    /// The caller supplies the joined structural flags of every child type.
    pub fn intern_type(&mut self, ty: Type, child_flags: TypeFlags) -> LocalTypeId {
        // probe the sealed segment beneath this tail first
        let hash = fx_hash(&ty);
        if let Some(sealed) = &self.sealed
            && let Some(slots) = self.sealed_index.get(&hash)
        {
            for slot in slots {
                if sealed.get_type_maybe(*slot).as_ref() == Some(&ty) {
                    return *slot;
                }
            }
        }

        // probe the index for an existing structural hit
        if let Some(slots) = self.index.get(&hash) {
            for slot in slots {
                if self.owned_type(*slot) == &ty {
                    return *slot;
                }
            }
        }

        // allocate and index the new slot
        let type_id = LocalTypeId::new(self.type_count());
        let flags = ty.own_flags() | child_flags;
        self.types.allocate(ty);
        self.flags.allocate(flags);
        self.hashes.push(hash);
        self.index.entry(hash).or_default().push(type_id);

        type_id
    }

    /// Intern one type operation payload.
    pub fn intern_operation(&mut self, operation: TypeOperation) -> TypeOperationId {
        // reuse what the sealed segment beneath this tail already interned
        if let Some(sealed) = &self.sealed
            && let Some(id) = sealed.operations.find(&operation)
        {
            return id;
        }

        self.operations.intern(operation)
    }

    /// Return the number of operations owned up to and including this segment.
    pub fn operation_count(&self) -> u32 {
        self.operations.count()
    }

    /// Return one operation payload, when owned by this segment.
    pub fn operation(&self, id: TypeOperationId) -> Option<&TypeOperation> {
        self.operations.get(id)
    }

    /// Intern one function signature payload.
    pub fn intern_signature(&mut self, signature: FunctionSignatureType) -> FunctionSignatureId {
        // reuse what the sealed segment beneath this tail already interned
        if let Some(sealed) = &self.sealed
            && let Some(id) = sealed.signatures.find(&signature)
        {
            return id;
        }

        self.signatures.intern(signature)
    }

    /// Return the number of signatures owned up to and including this segment.
    pub fn signature_count(&self) -> u32 {
        self.signatures.count()
    }

    /// Return one signature payload, when owned by this segment.
    pub fn signature(&self, id: FunctionSignatureId) -> Option<&FunctionSignatureType> {
        self.signatures.get(id)
    }

    /// Intern one member projection payload.
    pub fn intern_member(&mut self, member: MemberType) -> MemberTypeId {
        // reuse what the sealed segment beneath this tail already interned
        if let Some(sealed) = &self.sealed
            && let Some(id) = sealed.members.find(&member)
        {
            return id;
        }

        self.members.intern(member)
    }

    /// Return the number of members owned up to and including this segment.
    pub fn member_count(&self) -> u32 {
        self.members.count()
    }

    /// Return one member payload, when owned by this segment.
    pub fn member(&self, id: MemberTypeId) -> Option<&MemberType> {
        self.members.get(id)
    }

    /// Intern one refined application payload.
    pub fn intern_refined(&mut self, refined: RefinedType) -> RefinedTypeId {
        // reuse what the sealed segment beneath this tail already interned
        if let Some(sealed) = &self.sealed
            && let Some(id) = sealed.refinements.find(&refined)
        {
            return id;
        }

        self.refinements.intern(refined)
    }

    /// Return the number of refined payloads owned up to and including this segment.
    pub fn refined_count(&self) -> u32 {
        self.refinements.count()
    }

    /// Return one refined payload, when owned by this segment.
    pub fn refined(&self, id: RefinedTypeId) -> Option<&RefinedType> {
        self.refinements.get(id)
    }

    /// Intern one borrow form payload.
    pub fn intern_borrow(&mut self, borrow: BorrowForm) -> BorrowFormId {
        // reuse what the sealed segment beneath this tail already interned
        if let Some(sealed) = &self.sealed
            && let Some(id) = sealed.borrows.find(&borrow)
        {
            return id;
        }

        self.borrows.intern(borrow)
    }

    /// Return the number of borrows owned up to and including this segment.
    pub fn borrow_count(&self) -> u32 {
        self.borrows.count()
    }

    /// Return one borrow payload, when owned by this segment.
    pub fn borrow_form(&self, id: BorrowFormId) -> Option<&BorrowForm> {
        self.borrows.get(id)
    }

    /// Intern one type id list.
    pub fn intern_type_ids(&mut self, values: &[GlobalTypeId]) -> TypeListId {
        // reuse what the sealed segment beneath this tail already interned
        if let Some(sealed) = &self.sealed
            && let Some(list) = sealed.type_ids.find(values)
        {
            return list;
        }

        self.type_ids.intern(values)
    }

    /// Intern one tuple element list.
    pub fn intern_elements(&mut self, values: &[TypeElement]) -> TypeListId {
        // reuse what the sealed segment beneath this tail already interned
        if let Some(sealed) = &self.sealed
            && let Some(list) = sealed.elements.find(values)
        {
            return list;
        }

        self.elements.intern(values)
    }

    /// Intern one shape property list.
    pub fn intern_properties(&mut self, values: &[TypeProperty]) -> TypeListId {
        // reuse what the sealed segment beneath this tail already interned
        if let Some(sealed) = &self.sealed
            && let Some(list) = sealed.properties.find(values)
        {
            return list;
        }

        self.properties.intern(values)
    }

    /// Intern one function parameter list.
    pub fn intern_parameters(&mut self, values: &[FunctionParameterType]) -> TypeListId {
        // reuse what the sealed segment beneath this tail already interned
        if let Some(sealed) = &self.sealed
            && let Some(list) = sealed.parameters.find(values)
        {
            return list;
        }

        self.parameters.intern(values)
    }

    /// Intern one index signature list.
    pub fn intern_index_signatures(&mut self, values: &[TypeIndexSignature]) -> TypeListId {
        // reuse what the sealed segment beneath this tail already interned
        if let Some(sealed) = &self.sealed
            && let Some(list) = sealed.index_signatures.find(values)
        {
            return list;
        }

        self.index_signatures.intern(values)
    }

    /// Intern one string list.
    pub fn intern_strings(&mut self, values: &[StringId]) -> TypeListId {
        // reuse what the sealed segment beneath this tail already interned
        if let Some(sealed) = &self.sealed
            && let Some(list) = sealed.strings.find(values)
        {
            return list;
        }

        self.strings.intern(values)
    }

    /// Iterate effective checked types keyed by DIR node.
    pub fn node_types(&self) -> impl Iterator<Item = (GlobalNodeIdAny, GlobalTypeId)> + '_ {
        self.node_types
            .iter()
            .map(|(node_id, type_id)| (*node_id, *type_id))
    }

    /// Iterate solved symbol types.
    pub fn symbol_types(&self) -> impl Iterator<Item = (GlobalSymbolId, GlobalTypeId)> + '_ {
        self.symbol_types
            .iter()
            .map(|(symbol_id, type_id)| (*symbol_id, *type_id))
    }

    /// Iterate checked reduced types.
    pub fn reduced_types(&self) -> impl Iterator<Item = (GlobalTypeId, GlobalTypeId)> + '_ {
        self.reduced_types
            .iter()
            .map(|(source, target)| (*source, *target))
    }

    /// Set the effective checked type for a node.
    pub fn set_node_type(&mut self, node_id: GlobalNodeIdAny, type_id: GlobalTypeId) {
        self.node_types.insert(node_id, type_id);
    }

    /// Get the effective checked type id for a node.
    pub fn get_node_type_id(&self, node_id: GlobalNodeIdAny) -> Option<GlobalTypeId> {
        self.node_types.get(&node_id).copied()
    }

    /// Set the contextual expected type for a node.
    pub fn set_expected_type(&mut self, node_id: GlobalNodeIdAny, type_id: GlobalTypeId) {
        self.expected_types.insert(node_id, type_id);
    }

    /// Get the contextual expected type id for a node.
    pub fn get_expected_type_id(&self, node_id: GlobalNodeIdAny) -> Option<GlobalTypeId> {
        self.expected_types.get(&node_id).copied()
    }

    /// Set the solved type for a symbol.
    pub fn set_symbol_type(&mut self, symbol_id: GlobalSymbolId, ty: GlobalTypeId) {
        self.symbol_types.insert(symbol_id, ty);
    }

    /// Get the solved type id for a symbol.
    pub fn get_symbol_type_id(&self, symbol_id: GlobalSymbolId) -> Option<GlobalTypeId> {
        self.symbol_types.get(&symbol_id).copied()
    }

    /// Set the reduced type for one source type.
    pub fn set_type_reduction(&mut self, source: GlobalTypeId, target: GlobalTypeId) {
        if source == target {
            self.reduced_types.shift_remove(&source);
        } else {
            self.reduced_types.insert(source, target);
        }
    }

    /// Get the reduced type id for a checked type.
    pub fn get_reduced_type_id(&self, type_id: GlobalTypeId) -> Option<GlobalTypeId> {
        self.reduced_types.get(&type_id).copied()
    }

    /// Get a type by its id.
    pub fn get_type(&self, type_id: LocalTypeId) -> Type {
        self.get_type_maybe(type_id)
            .unwrap_or_else(|| panic!("DIR type {type_id:?} is not allocated in this segment"))
    }

    /// Get a type by its id when present.
    pub fn get_type_maybe(&self, type_id: LocalTypeId) -> Option<Type> {
        self.contains_type_id(type_id)
            .then(|| *self.types.get(type_id.0 - self.first_type_id))
    }

    /// Get the structural flags for a type when present.
    pub fn get_type_flags_maybe(&self, type_id: LocalTypeId) -> Option<TypeFlags> {
        self.contains_type_id(type_id)
            .then(|| *self.flags.get(type_id.0 - self.first_type_id))
    }

    /// Get one type id list when this segment owns it.
    pub fn type_ids_maybe(&self, list: TypeListId) -> Option<&[GlobalTypeId]> {
        self.type_ids.get_maybe(list)
    }

    /// Get one tuple element list when this segment owns it.
    pub fn elements_maybe(&self, list: TypeListId) -> Option<&[TypeElement]> {
        self.elements.get_maybe(list)
    }

    /// Get one shape property list when this segment owns it.
    pub fn properties_maybe(&self, list: TypeListId) -> Option<&[TypeProperty]> {
        self.properties.get_maybe(list)
    }

    /// Get one function parameter list when this segment owns it.
    pub fn parameters_maybe(&self, list: TypeListId) -> Option<&[FunctionParameterType]> {
        self.parameters.get_maybe(list)
    }

    /// Get one index signature list when this segment owns it.
    pub fn index_signatures_maybe(&self, list: TypeListId) -> Option<&[TypeIndexSignature]> {
        self.index_signatures.get_maybe(list)
    }

    /// Get one string list when this segment owns it.
    pub fn strings_maybe(&self, list: TypeListId) -> Option<&[StringId]> {
        self.strings.get_maybe(list)
    }

    /// Strip outer form types to reach the payload type id.
    pub fn unwrap_form_payload_type_id(&self, type_id: LocalTypeId) -> GlobalTypeId {
        let mut current = type_id;
        loop {
            match self.get_type(current) {
                Type::Form(form) if form.value.module_id == self.module_id => {
                    current = form.value.local_id;
                }
                Type::Form(form) => return form.value,
                _ => return current.into_global(self.module_id),
            }
        }
    }

    /// Iterate over all type ids.
    pub fn iter_type_ids(&self) -> impl Iterator<Item = LocalTypeId> + '_ {
        let end = self.type_count();

        (self.first_type_id..end).map(LocalTypeId::new)
    }

    /// Get the number of types in the table.
    pub fn type_count(&self) -> u32 {
        self.first_type_id + self.types.len() as u32
    }

    /// Mark this segment for speculative rollback.
    pub fn mark(&self) -> TypeMark {
        TypeMark {
            types: self.type_count(),
            type_ids: self.type_ids.element_count(),
            elements: self.elements.element_count(),
            properties: self.properties.element_count(),
            parameters: self.parameters.element_count(),
            index_signatures: self.index_signatures.element_count(),
            strings: self.strings.element_count(),
            operations: self.operations.count(),
            signatures: self.signatures.count(),
            members: self.members.count(),
            refinements: self.refinements.count(),
            borrows: self.borrows.count(),
        }
    }

    /// Drop every type and list interned after one mark.
    pub fn truncate_to(&mut self, mark: TypeMark) {
        // unindex the dropped type slots
        let keep = mark.types.saturating_sub(self.first_type_id) as usize;
        for slot in keep..self.types.len() {
            let hash = self.hashes[slot];
            let type_id = LocalTypeId::new(self.first_type_id + slot as u32);
            if let Some(slots) = self.index.get_mut(&hash) {
                slots.retain(|entry| *entry != type_id);
            }
        }

        // drop the type slots and their lists
        self.types.truncate(keep);
        self.flags.truncate(keep);
        self.hashes.truncate(keep);
        self.type_ids.truncate_to(mark.type_ids);
        self.elements.truncate_to(mark.elements);
        self.properties.truncate_to(mark.properties);
        self.parameters.truncate_to(mark.parameters);
        self.index_signatures.truncate_to(mark.index_signatures);
        self.strings.truncate_to(mark.strings);

        // drop the payload slots
        self.operations.truncate_to(mark.operations);
        self.signatures.truncate_to(mark.signatures);
        self.members.truncate_to(mark.members);
        self.refinements.truncate_to(mark.refinements);
        self.borrows.truncate_to(mark.borrows);
    }

    /// Return the number of entries in this table.
    pub fn len(&self) -> u32 {
        self.type_count()
    }

    /// Return true when this table has no entries.
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
            && self.node_types.is_empty()
            && self.expected_types.is_empty()
            && self.symbol_types.is_empty()
            && self.reduced_types.is_empty()
    }

    /// Return whether this segment contains the given type id.
    fn contains_type_id(&self, type_id: LocalTypeId) -> bool {
        type_id.0 >= self.first_type_id && type_id.0 < self.type_count()
    }

    /// Return one owned type slot.
    fn owned_type(&self, type_id: LocalTypeId) -> &Type {
        self.types.get(type_id.0 - self.first_type_id)
    }
}

/// Interned lists of one type payload kind.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub(crate) struct ListPool<T> {
    /// The first element owned by this segment.
    first: u32,
    /// The stored list elements.
    elements: Arena<T>,
    /// The intern index from list hash to owned list ids.
    #[serde(skip)]
    index: FxHashMap<u64, SmallVec<[TypeListId; 1]>>,
    /// The intern log of owned list ids, in allocation order.
    #[serde(skip)]
    log: Vec<(u64, TypeListId)>,
}

impl<T> ListPool<T> {
    /// Create empty list storage starting at one cumulative offset.
    fn new(first: u32) -> Self {
        Self {
            first,
            elements: Arena::new(),
            index: FxHashMap::default(),
            log: Vec::new(),
        }
    }

    /// Get one owned list.
    fn get(&self, list: TypeListId) -> &[T] {
        self.get_maybe(list)
            .unwrap_or_else(|| panic!("DIR type list {list:?} is not allocated in this segment"))
    }

    /// Get one owned list when present.
    fn get_maybe(&self, list: TypeListId) -> Option<&[T]> {
        if list.is_empty() {
            return Some(&[]);
        }

        let start = list.start.checked_sub(self.first)? as usize;
        let end = start + list.count as usize;

        self.elements.as_slice().get(start..end)
    }

    /// Return the cumulative element count.
    fn element_count(&self) -> u32 {
        self.first + self.elements.len() as u32
    }

    /// Drop every list interned after one cumulative count.
    fn truncate_to(&mut self, count: u32) {
        // unindex the dropped lists
        while let Some((hash, list)) = self.log.last().copied() {
            if list.start < count {
                break;
            }
            if let Some(lists) = self.index.get_mut(&hash) {
                lists.retain(|entry| *entry != list);
            }
            self.log.pop();
        }

        // drop the elements
        let keep = count.saturating_sub(self.first) as usize;
        self.elements.truncate(keep);
    }
}

impl<T: Copy + Eq + Hash> ListPool<T> {
    /// Find one already-interned list without allocating.
    fn find(&self, values: &[T]) -> Option<TypeListId> {
        // canonicalize the empty list without touching storage
        if values.is_empty() {
            return Some(TypeListId::EMPTY);
        }

        // probe the index for an existing content hit
        let hash = fx_hash(&values);
        let lists = self.index.get(&hash)?;

        lists
            .iter()
            .find(|list| self.get(**list) == values)
            .copied()
    }

    /// Intern one list.
    fn intern(&mut self, values: &[T]) -> TypeListId {
        // canonicalize the empty list without touching storage
        if values.is_empty() {
            return TypeListId::EMPTY;
        }

        // probe the index for an existing content hit
        let hash = fx_hash(&values);
        if let Some(lists) = self.index.get(&hash) {
            for list in lists {
                if self.get(*list) == values {
                    return *list;
                }
            }
        }

        // append and index the new list
        let list = TypeListId::new(self.element_count(), values.len() as u32);
        for value in values {
            self.elements.allocate(*value);
        }
        self.index.entry(hash).or_default().push(list);
        self.log.push((hash, list));

        list
    }
}

/// Return the FxHasher hash of one value.
fn fx_hash(value: &impl Hash) -> u64 {
    let mut hasher = FxHasher::default();
    value.hash(&mut hasher);

    hasher.finish()
}
