use std::hash::{Hash, Hasher};
use std::sync::Arc;

use destack_core::FxIndexMap as IndexMap;
use destack_serde::Reflect;
use rustc_hash::{FxHashMap, FxHasher};
use serde::{Deserialize, Serialize};
use siphasher::sip128::{Hasher128, SipHasher13};
use smallvec::SmallVec;

use destack_core::{Arena, PoolId, StringId, ValueInterner, ValuePool};
use destack_source::ModuleId;
use elsa::sync::FrozenVec;

use crate::{
    BorrowForm, BorrowFormId, Form, FunctionParameterType, FunctionSignatureId,
    FunctionSignatureType, GenericArgumentBinding, GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId,
    LocalTypeId, MemberType, MemberTypeId, RefinedType, RefinedTypeId, SegmentView, Type,
    TypeElement, TypeFlags, TypeIndexSignature, TypeListId, TypeOperation, TypeOperationId,
    TypeProperty,
};

/// Cumulative type slots for one DIR module.
#[derive(Debug, Clone)]
pub struct TypeTable<'a> {
    /// The module id of the type table.
    pub module_id: ModuleId,
    /// The ordered type table segments.
    segments: SegmentView<'a, TypeSegment>,
    /// The lists an open tail interned, read ahead of the segments.
    lists: Option<&'a TypeListArena>,
}

impl TypeTable<'static> {
    /// Create a type table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<TypeSegment>>) -> Self {
        Self::from_view(SegmentView::from_segments(segments))
    }

    /// Create a type table from one segment.
    pub fn from_segment(segment: Arc<TypeSegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl TypeTable<'_> {
    /// Return the committed segments in stage order.
    pub fn segments(&self) -> &[Arc<TypeSegment>] {
        self.segments.committed()
    }
}

impl<'a> TypeTable<'a> {
    /// Create a type table from a segment view.
    pub fn from_view(segments: SegmentView<'a, TypeSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("type table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner and contiguous id ranges
        let mut count = 0;
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "type table segment belongs to a different module"
            );
            assert_eq!(
                segment.first_type_id, count,
                "type table segments must stack contiguously"
            );
            count += segment.types.len() as u32;
        }

        Self {
            module_id,
            segments,
            lists: None,
        }
    }

    /// Create a type table by appending a borrowed tail.
    pub fn with_tail<'b>(&'b self, tail: &'b TypeTail<'_>) -> TypeTable<'b> {
        let mut table = TypeTable::from_view(self.segments.with_tail(&tail.segment));
        table.lists = Some(tail.lists);

        table
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

    /// Get the effective checked type id for a node.
    pub fn get_node_type_id(&self, node_id: GlobalNodeIdAny) -> Option<GlobalTypeId> {
        for segment in self.segments.iter().rev() {
            if let Some(type_id) = segment.get_node_type_id(node_id) {
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

    /// Return the type one written head lowers through, recorded where it differs.
    pub fn reduction(&self, type_id: GlobalTypeId) -> Option<GlobalTypeId> {
        for segment in self.segments.iter().rev() {
            if let Some(reduced) = segment.reduction(type_id) {
                return Some(reduced);
            }
        }

        None
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
        self.slice(list, |segment| &segment.type_ids, |lists| &lists.type_ids)
    }

    /// Get one tuple element list.
    pub fn elements(&self, list: TypeListId) -> &[TypeElement] {
        self.slice(list, |segment| &segment.elements, |lists| &lists.elements)
    }

    /// Get one shape property list.
    pub fn properties(&self, list: TypeListId) -> &[TypeProperty] {
        self.slice(
            list,
            |segment| &segment.properties,
            |lists| &lists.properties,
        )
    }

    /// Get one function parameter list.
    pub fn parameters(&self, list: TypeListId) -> &[FunctionParameterType] {
        self.slice(
            list,
            |segment| &segment.parameters,
            |lists| &lists.parameters,
        )
    }

    /// Get one generic argument binding list.
    pub fn generic_arguments(&self, list: TypeListId) -> &[GenericArgumentBinding] {
        self.slice(
            list,
            |segment| &segment.generic_arguments,
            |lists| &lists.generic_arguments,
        )
    }

    /// Get one index signature list.
    pub fn index_signatures(&self, list: TypeListId) -> &[TypeIndexSignature] {
        self.slice(
            list,
            |segment| &segment.index_signatures,
            |lists| &lists.index_signatures,
        )
    }

    /// Get one string list.
    pub fn strings(&self, list: TypeListId) -> &[StringId] {
        self.slice(list, |segment| &segment.strings, |lists| &lists.strings)
    }

    /// Visit each direct child type id of one type owned by this module.
    pub fn for_each_child(&self, ty: &Type, mut visit: impl FnMut(GlobalTypeId)) {
        match ty {
            // leaves without child types
            Type::Variable(_)
            | Type::Error
            | Type::Never
            | Type::Unknown
            | Type::Void
            | Type::Null
            | Type::Undefined
            | Type::Primitive(_)
            | Type::Literal(_)
            | Type::Key(_)
            | Type::Static(_)
            | Type::Intrinsic
            | Type::Erased(_)
            | Type::Parameter(_)
            | Type::This
            | Type::Range(_) => {}

            // declaration references
            Type::Reference(reference) => {
                for child in self.type_ids(reference.arguments) {
                    visit(*child);
                }
            }

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

            // regions visit both coordinates
            Type::Region(region) => {
                visit(region.extent);
                visit(region.space);
            }

            // memory forms
            Type::Form(form) => {
                visit(form.value);
                match &form.form {
                    Form::Borrowed(borrow) => {
                        let borrow = self.borrow_form(*borrow);
                        visit(borrow.region);
                        visit(borrow.access);
                        visit(borrow.exclusivity);
                    }
                    Form::Managed { place } => visit(*place),
                    Form::Owned | Form::Raw | Form::Readonly => {}
                }
            }
            Type::Dynamic(dynamic) => {
                visit(dynamic.constraint);
                visit(dynamic.place);
            }

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
                TypeOperation::Instantiation(application) => {
                    visit(application.target);
                    for argument in self.type_ids(application.arguments) {
                        visit(*argument);
                    }
                }
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
            Type::FixedArray(array) => {
                visit(array.element);
                visit(array.count);
            }
            Type::Slice(slice) => {
                visit(slice.element);
                visit(slice.place);
            }
            Type::Tuple(tuple) => {
                for element in self.elements(tuple.elements) {
                    visit(element.ty);
                }
            }

            // concrete object classes
            Type::Object(shape) => {
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
                for binding in self.generic_arguments(function.arguments) {
                    visit(binding.argument);
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
                visit(function.receiver);
                visit(function.place);
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

    /// Strip outer form types down to the payload type id.
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
    fn slice<T>(
        &self,
        list: TypeListId,
        pool: impl Fn(&TypeSegment) -> &ListPool<T>,
        arena: impl Fn(&TypeListArena) -> &ListArena<T>,
    ) -> &[T] {
        if list.is_empty() {
            return &[];
        }

        // read the open tail's lists, then the segments from the newest backward
        if let Some(slice) = self.lists.and_then(|lists| arena(lists).get_maybe(list)) {
            return slice;
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

/// One module's layer of checked types over the committed base.
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
    /// The generic argument binding lists referenced by type payloads.
    pub(crate) generic_arguments: ListPool<GenericArgumentBinding>,
    /// The index signature lists referenced by type payloads.
    pub(crate) index_signatures: ListPool<TypeIndexSignature>,
    /// The string lists referenced by type payloads.
    pub(crate) strings: ListPool<StringId>,

    /// The interned type operation payloads.
    pub(crate) operations: ValuePool<TypeOperation>,
    /// The interned function signature payloads.
    pub(crate) signatures: ValuePool<FunctionSignatureType>,
    /// The interned member projection payloads.
    pub(crate) members: ValuePool<MemberType>,
    /// The interned refined application payloads.
    pub(crate) refinements: ValuePool<RefinedType>,
    /// The interned borrow form payloads.
    pub(crate) borrows: ValuePool<BorrowForm>,

    /// Effective checked type by node.
    pub(crate) node_types: IndexMap<GlobalNodeIdAny, GlobalTypeId>,
    /// Checked declaration type by symbol.
    pub(crate) symbol_types: IndexMap<GlobalSymbolId, GlobalTypeId>,
    /// The type each written head lowers through, by head, recorded where it differs.
    pub(crate) reductions: IndexMap<GlobalTypeId, GlobalTypeId>,
}

impl TypeSegment {
    /// Return the first type id owned by this segment.
    pub fn first_type_id(&self) -> u32 {
        self.first_type_id
    }

    /// Digest the segment content for artifact fingerprinting.
    pub fn content_digest(&self) -> u128 {
        let mut hasher = SipHasher13::new();
        Hasher::write(&mut hasher, b"destack.dir.types.digest.v1");
        self.module_id.hash(&mut hasher);
        self.first_type_id.hash(&mut hasher);

        // hash the dense pools directly
        self.types.hash(&mut hasher);
        self.flags.hash(&mut hasher);
        self.type_ids.hash(&mut hasher);
        self.elements.hash(&mut hasher);
        self.properties.hash(&mut hasher);
        self.parameters.hash(&mut hasher);
        self.generic_arguments.hash(&mut hasher);
        self.index_signatures.hash(&mut hasher);
        self.strings.hash(&mut hasher);
        self.operations.hash(&mut hasher);
        self.signatures.hash(&mut hasher);
        self.members.hash(&mut hasher);
        self.refinements.hash(&mut hasher);
        self.borrows.hash(&mut hasher);

        // hash the assignment maps in their insertion order
        self.node_types.len().hash(&mut hasher);
        for (node, ty) in &self.node_types {
            node.hash(&mut hasher);
            ty.hash(&mut hasher);
        }
        self.symbol_types.len().hash(&mut hasher);
        for (symbol, ty) in &self.symbol_types {
            symbol.hash(&mut hasher);
            ty.hash(&mut hasher);
        }
        self.reductions.len().hash(&mut hasher);
        for (ty, reduced) in &self.reductions {
            ty.hash(&mut hasher);
            reduced.hash(&mut hasher);
        }

        hasher.finish128().as_u128()
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
            generic_arguments: ListPool::new(0),
            index_signatures: ListPool::new(0),
            strings: ListPool::new(0),
            operations: ValuePool::new(0),
            signatures: ValuePool::new(0),
            members: ValuePool::new(0),
            refinements: ValuePool::new(0),
            borrows: ValuePool::new(0),
            node_types: IndexMap::default(),
            symbol_types: IndexMap::default(),
            reductions: IndexMap::default(),
        }
    }

    /// Create a new empty segment after an existing type table segment.
    pub fn from_base(base: &Self) -> Self {
        Self {
            module_id: base.module_id,
            first_type_id: base.type_count(),
            types: Arena::new(),
            flags: Arena::new(),
            type_ids: ListPool::new(base.type_ids.list_count()),
            elements: ListPool::new(base.elements.list_count()),
            properties: ListPool::new(base.properties.list_count()),
            parameters: ListPool::new(base.parameters.list_count()),
            generic_arguments: ListPool::new(base.generic_arguments.list_count()),
            index_signatures: ListPool::new(base.index_signatures.list_count()),
            strings: ListPool::new(base.strings.list_count()),
            operations: ValuePool::new(base.operations.count()),
            signatures: ValuePool::new(base.signatures.count()),
            members: ValuePool::new(base.members.count()),
            refinements: ValuePool::new(base.refinements.count()),
            borrows: ValuePool::new(base.borrows.count()),
            node_types: IndexMap::default(),
            symbol_types: IndexMap::default(),
            reductions: IndexMap::default(),
        }
    }

    /// Allocate one type entry without probing for duplicates.
    fn allocate_type(&mut self, ty: Type, child_flags: TypeFlags) -> LocalTypeId {
        let type_id = LocalTypeId::new(self.type_count());
        let flags = ty.own_flags() | child_flags;
        self.types.allocate(ty);
        self.flags.allocate(flags);

        type_id
    }

    /// Return the number of operations owned up to and including this segment.
    pub fn operation_count(&self) -> u32 {
        self.operations.count()
    }

    /// Return one operation payload, when owned by this segment.
    pub fn operation(&self, id: TypeOperationId) -> Option<&TypeOperation> {
        self.operations.get(id)
    }

    /// Return the number of signatures owned up to and including this segment.
    pub fn signature_count(&self) -> u32 {
        self.signatures.count()
    }

    /// Return one signature payload, when owned by this segment.
    pub fn signature(&self, id: FunctionSignatureId) -> Option<&FunctionSignatureType> {
        self.signatures.get(id)
    }

    /// Return the number of members owned up to and including this segment.
    pub fn member_count(&self) -> u32 {
        self.members.count()
    }

    /// Return one member payload, when owned by this segment.
    pub fn member(&self, id: MemberTypeId) -> Option<&MemberType> {
        self.members.get(id)
    }

    /// Return the number of refined payloads owned up to and including this segment.
    pub fn refined_count(&self) -> u32 {
        self.refinements.count()
    }

    /// Return one refined payload, when owned by this segment.
    pub fn refined(&self, id: RefinedTypeId) -> Option<&RefinedType> {
        self.refinements.get(id)
    }

    /// Return the number of borrows owned up to and including this segment.
    pub fn borrow_count(&self) -> u32 {
        self.borrows.count()
    }

    /// Return one borrow payload, when owned by this segment.
    pub fn borrow_form(&self, id: BorrowFormId) -> Option<&BorrowForm> {
        self.borrows.get(id)
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

    /// Set the effective checked type for a node.
    pub fn set_node_type(&mut self, node_id: GlobalNodeIdAny, type_id: GlobalTypeId) {
        self.node_types.insert(node_id, type_id);
    }

    /// Get the effective checked type id for a node.
    pub fn get_node_type_id(&self, node_id: GlobalNodeIdAny) -> Option<GlobalTypeId> {
        self.node_types.get(&node_id).copied()
    }

    /// Set the solved type for a symbol.
    pub fn set_symbol_type(&mut self, symbol_id: GlobalSymbolId, ty: GlobalTypeId) {
        self.symbol_types.insert(symbol_id, ty);
    }

    /// Get the solved type id for a symbol.
    pub fn get_symbol_type_id(&self, symbol_id: GlobalSymbolId) -> Option<GlobalTypeId> {
        self.symbol_types.get(&symbol_id).copied()
    }

    /// Return the type one written head lowers through, recorded where it differs.
    pub fn reduction(&self, type_id: GlobalTypeId) -> Option<GlobalTypeId> {
        self.reductions.get(&type_id).copied()
    }

    /// Record the type one written head lowers through.
    pub fn set_reduction(&mut self, type_id: GlobalTypeId, reduced: GlobalTypeId) {
        self.reductions.insert(type_id, reduced);
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

    /// Get one generic argument binding list when this segment owns it.
    pub fn generic_arguments_maybe(&self, list: TypeListId) -> Option<&[GenericArgumentBinding]> {
        self.generic_arguments.get_maybe(list)
    }

    /// Get one index signature list when this segment owns it.
    pub fn index_signatures_maybe(&self, list: TypeListId) -> Option<&[TypeIndexSignature]> {
        self.index_signatures.get_maybe(list)
    }

    /// Get one string list when this segment owns it.
    pub fn strings_maybe(&self, list: TypeListId) -> Option<&[StringId]> {
        self.strings.get_maybe(list)
    }

    /// Strip outer form types down to the payload type id.
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

    /// Return the number of entries in this table.
    pub fn len(&self) -> u32 {
        self.type_count()
    }

    /// Return true when this table has no entries.
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
            && self.node_types.is_empty()
            && self.symbol_types.is_empty()
            && self.reductions.is_empty()
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

/// Interned lists of one type payload kind, each list its own allocation.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub(crate) struct ListPool<T> {
    /// The index of the first list owned by this segment.
    first: u32,
    /// The lists in allocation order.
    lists: Vec<Vec<T>>,
}

impl<T: Hash> Hash for ListPool<T> {
    /// Hash the lists behind the base index.
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.first.hash(state);
        self.lists.hash(state);
    }
}

/// Intern bookkeeping growing one list pool.
#[derive(Debug, Clone, Default)]
pub(crate) struct ListInterner {
    /// The intern index from list hash to owned list ids.
    index: FxHashMap<u64, SmallVec<[TypeListId; 1]>>,
    /// The intern index over the committed segments beneath this tail.
    committed: FxHashMap<u64, SmallVec<[TypeListId; 1]>>,
    /// The intern log of owned list ids, in allocation order.
    log: Vec<(u64, TypeListId)>,
}

impl<T> ListPool<T> {
    /// Create empty list storage starting at one cumulative list index.
    fn new(first: u32) -> Self {
        Self {
            first,
            lists: Vec::new(),
        }
    }

    /// Create list storage taking the lists one arena interned.
    fn from_arena(arena: ListArena<T>) -> Self {
        Self {
            first: arena.first,
            lists: arena.lists.into_vec(),
        }
    }

    /// Get one owned list when present.
    fn get_maybe(&self, list: TypeListId) -> Option<&[T]> {
        if list.is_empty() {
            return Some(&[]);
        }
        let position = list.index.checked_sub(self.first)? as usize;

        self.lists
            .get(position)
            .filter(|values| values.len() == list.count as usize)
            .map(Vec::as_slice)
    }

    /// Return the cumulative list count.
    fn list_count(&self) -> u32 {
        self.first + self.lists.len() as u32
    }
}

/// The lists one pass interns, appended through shared references and read past later appends.
pub struct TypeListArena {
    /// The type id lists.
    type_ids: ListArena<GlobalTypeId>,
    /// The tuple element lists.
    elements: ListArena<TypeElement>,
    /// The shape property lists.
    properties: ListArena<TypeProperty>,
    /// The function parameter lists.
    parameters: ListArena<FunctionParameterType>,
    /// The generic argument binding lists.
    generic_arguments: ListArena<GenericArgumentBinding>,
    /// The index signature lists.
    index_signatures: ListArena<TypeIndexSignature>,
    /// The string lists.
    strings: ListArena<StringId>,
}

impl std::fmt::Debug for TypeListArena {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypeListArena").finish_non_exhaustive()
    }
}

impl TypeListArena {
    /// Create the arena continuing one segment's pools.
    pub fn following(base: &TypeSegment) -> Self {
        Self {
            type_ids: ListArena::new(base.type_ids.list_count()),
            elements: ListArena::new(base.elements.list_count()),
            properties: ListArena::new(base.properties.list_count()),
            parameters: ListArena::new(base.parameters.list_count()),
            generic_arguments: ListArena::new(base.generic_arguments.list_count()),
            index_signatures: ListArena::new(base.index_signatures.list_count()),
            strings: ListArena::new(base.strings.list_count()),
        }
    }

    /// Move the interned lists into the segment the pass finished.
    pub fn finish_into(self, segment: &mut TypeSegment) {
        segment.type_ids = ListPool::from_arena(self.type_ids);
        segment.elements = ListPool::from_arena(self.elements);
        segment.properties = ListPool::from_arena(self.properties);
        segment.parameters = ListPool::from_arena(self.parameters);
        segment.generic_arguments = ListPool::from_arena(self.generic_arguments);
        segment.index_signatures = ListPool::from_arena(self.index_signatures);
        segment.strings = ListPool::from_arena(self.strings);
    }

    /// Create the arena of a module without committed lists.
    pub fn new() -> Self {
        Self {
            type_ids: ListArena::new(0),
            elements: ListArena::new(0),
            properties: ListArena::new(0),
            parameters: ListArena::new(0),
            generic_arguments: ListArena::new(0),
            index_signatures: ListArena::new(0),
            strings: ListArena::new(0),
        }
    }
}

impl Default for TypeListArena {
    fn default() -> Self {
        Self::new()
    }
}

/// One kind's lists, each stored at its cumulative index.
struct ListArena<T> {
    /// The index of the first list this arena allocates.
    first: u32,
    /// The lists in allocation order.
    lists: FrozenVec<Vec<T>>,
}

impl<T> ListArena<T> {
    /// Create empty storage starting at one cumulative list index.
    fn new(first: u32) -> Self {
        Self {
            first,
            lists: FrozenVec::new(),
        }
    }

    /// Get one list when this arena holds it.
    fn get_maybe(&self, list: TypeListId) -> Option<&[T]> {
        if list.is_empty() {
            return Some(&[]);
        }
        let position = list.index.checked_sub(self.first)? as usize;

        self.lists
            .get(position)
            .filter(|values| values.len() == list.count as usize)
    }
}

impl<T: Copy> ListArena<T> {
    /// Allocate one list at the arena tail.
    fn allocate_list(&self, values: &[T]) -> TypeListId {
        let list = TypeListId::new(self.first + self.lists.len() as u32, values.len() as u32);
        self.lists.push(values.to_vec());

        list
    }
}

impl ListInterner {
    /// Seed the committed index from one committed segment's pool.
    fn seed<T: Copy + Eq + Hash>(&mut self, pool: &ListPool<T>) {
        for (position, values) in pool.lists.iter().enumerate() {
            let list = TypeListId::new(pool.first + position as u32, values.len() as u32);
            let hash = fx_hash(&values.as_slice());
            self.committed.entry(hash).or_default().push(list);
        }
    }

    /// Intern one list into the pool, reusing committed content.
    fn intern<T: Copy + Eq + Hash>(
        &mut self,
        arena: &ListArena<T>,
        committed_holds: impl Fn(TypeListId, &[T]) -> bool,
        values: &[T],
    ) -> TypeListId {
        // canonicalize the empty list without touching storage
        if values.is_empty() {
            return TypeListId::EMPTY;
        }

        // probe the committed segments beneath this tail
        let hash = fx_hash(&values);
        if let Some(lists) = self.committed.get(&hash) {
            for list in lists {
                if committed_holds(*list, values) {
                    return *list;
                }
            }
        }

        // probe the index for an existing content hit
        if let Some(lists) = self.index.get(&hash) {
            for list in lists {
                if arena.get_maybe(*list) == Some(values) {
                    return *list;
                }
            }
        }

        // append and index the new list
        let list = arena.allocate_list(values);
        self.index.entry(hash).or_default().push(list);
        self.log.push((hash, list));

        list
    }
}

/// Compute the intern hash of one value.
fn fx_hash(value: &impl Hash) -> u64 {
    let mut hasher = FxHasher::default();
    value.hash(&mut hasher);

    hasher.finish()
}

/// One growing type segment interning over committed bases.
#[derive(Debug, Clone)]
pub struct TypeTail<'a> {
    /// The entries built by this pass.
    segment: TypeSegment,
    /// The lists this pass interns, owned past the pass by the frame running it.
    lists: &'a TypeListArena,
    /// The intern index from value hash to owned type slots.
    index: FxHashMap<u64, SmallVec<[LocalTypeId; 1]>>,
    /// The value hash per owned type, parallel to the segment's types.
    hashes: Vec<u64>,
    /// The committed segments beneath this tail.
    committed: Vec<Arc<TypeSegment>>,
    /// The intern index over the committed segments' types.
    committed_index: FxHashMap<u64, SmallVec<[LocalTypeId; 1]>>,
    /// Intern bookkeeping for the type id pool.
    type_ids: ListInterner,
    /// Intern bookkeeping for the tuple element pool.
    elements: ListInterner,
    /// Intern bookkeeping for the shape property pool.
    properties: ListInterner,
    /// Intern bookkeeping for the function parameter pool.
    parameters: ListInterner,
    /// Intern bookkeeping for the generic argument binding pool.
    generic_arguments: ListInterner,
    /// Intern bookkeeping for the index signature pool.
    index_signatures: ListInterner,
    /// Intern bookkeeping for the string pool.
    strings: ListInterner,

    /// Intern bookkeeping for the type operation pool.
    operations: ValueInterner<TypeOperationId>,
    /// Intern bookkeeping for the function signature pool.
    signatures: ValueInterner<FunctionSignatureId>,
    /// Intern bookkeeping for the member projection pool.
    members: ValueInterner<MemberTypeId>,
    /// Intern bookkeeping for the refined application pool.
    refinements: ValueInterner<RefinedTypeId>,
    /// Intern bookkeeping for the borrow form pool.
    borrows: ValueInterner<BorrowFormId>,
}

impl std::ops::Deref for TypeTail<'_> {
    type Target = TypeSegment;

    fn deref(&self) -> &TypeSegment {
        &self.segment
    }
}

impl std::ops::DerefMut for TypeTail<'_> {
    fn deref_mut(&mut self) -> &mut TypeSegment {
        &mut self.segment
    }
}

impl<'a> TypeTail<'a> {
    /// Create an empty tail over one fresh module segment.
    pub fn new(module_id: ModuleId, lists: &'a TypeListArena) -> Self {
        Self::wrap(TypeSegment::new(module_id), Vec::new(), lists)
    }

    /// Create an empty tail continuing one open base segment.
    pub fn from_base(base: &TypeSegment, lists: &'a TypeListArena) -> Self {
        Self::wrap(TypeSegment::from_base(base), Vec::new(), lists)
    }

    /// Create an empty tail whose interning reuses one committed segment's types.
    pub fn over_base(base: Arc<TypeSegment>, lists: &'a TypeListArena) -> Self {
        Self::over(vec![base], lists)
    }

    /// Create an empty tail whose interning reuses stacked committed segments' types.
    pub fn over(bases: Vec<Arc<TypeSegment>>, lists: &'a TypeListArena) -> Self {
        let last = bases
            .last()
            .expect("committed tail requires at least one base");
        let mut tail = Self::wrap(TypeSegment::from_base(last), bases, lists);

        // index the committed types so identical structures reuse their ids
        for base in &tail.committed {
            for (slot, ty) in base.types.iter().enumerate() {
                let id = LocalTypeId::new(base.first_type_id + slot as u32);
                tail.committed_index
                    .entry(fx_hash(ty))
                    .or_default()
                    .push(id);
            }
        }

        // index the committed lists so identical content reuses their ids
        for base in tail.committed.clone() {
            tail.type_ids.seed(&base.type_ids);
            tail.elements.seed(&base.elements);
            tail.properties.seed(&base.properties);
            tail.parameters.seed(&base.parameters);
            tail.generic_arguments.seed(&base.generic_arguments);
            tail.index_signatures.seed(&base.index_signatures);
            tail.strings.seed(&base.strings);
            tail.operations.seed(&base.operations);
            tail.signatures.seed(&base.signatures);
            tail.members.seed(&base.members);
            tail.refinements.seed(&base.refinements);
            tail.borrows.seed(&base.borrows);
        }

        tail
    }

    /// Finish this tail into its pure entry segment, the frame moving the interned lists in.
    pub fn finish(self) -> TypeSegment {
        self.segment
    }

    /// Get one type id list this pass interned, read past later interns.
    pub fn type_ids_maybe(&self, list: TypeListId) -> Option<&'a [GlobalTypeId]> {
        self.lists.type_ids.get_maybe(list)
    }

    /// Get one tuple element list this pass interned, read past later interns.
    pub fn elements_maybe(&self, list: TypeListId) -> Option<&'a [TypeElement]> {
        self.lists.elements.get_maybe(list)
    }

    /// Get one shape property list this pass interned, read past later interns.
    pub fn properties_maybe(&self, list: TypeListId) -> Option<&'a [TypeProperty]> {
        self.lists.properties.get_maybe(list)
    }

    /// Get one function parameter list this pass interned, read past later interns.
    pub fn parameters_maybe(&self, list: TypeListId) -> Option<&'a [FunctionParameterType]> {
        self.lists.parameters.get_maybe(list)
    }

    /// Get one generic argument binding list this pass interned, read past later interns.
    pub fn generic_arguments_maybe(
        &self,
        list: TypeListId,
    ) -> Option<&'a [GenericArgumentBinding]> {
        self.lists.generic_arguments.get_maybe(list)
    }

    /// Get one index signature list this pass interned, read past later interns.
    pub fn index_signatures_maybe(&self, list: TypeListId) -> Option<&'a [TypeIndexSignature]> {
        self.lists.index_signatures.get_maybe(list)
    }

    /// Get one string list this pass interned, read past later interns.
    pub fn strings_maybe(&self, list: TypeListId) -> Option<&'a [StringId]> {
        self.lists.strings.get_maybe(list)
    }

    /// Record one symbol's type in this tail.
    pub fn set_symbol_type(&mut self, symbol_id: GlobalSymbolId, ty: GlobalTypeId) {
        self.segment.set_symbol_type(symbol_id, ty);
    }

    /// Record one node's type in this tail.
    pub fn set_node_type(&mut self, node_id: GlobalNodeIdAny, ty: GlobalTypeId) {
        self.segment.set_node_type(node_id, ty);
    }

    /// Wrap one segment with empty intern bookkeeping.
    fn wrap(
        segment: TypeSegment,
        committed: Vec<Arc<TypeSegment>>,
        lists: &'a TypeListArena,
    ) -> Self {
        debug_assert_eq!(lists.type_ids.first, segment.type_ids.first);
        Self {
            segment,
            lists,
            index: FxHashMap::default(),
            hashes: Vec::new(),
            committed,
            committed_index: FxHashMap::default(),
            type_ids: ListInterner::default(),
            elements: ListInterner::default(),
            properties: ListInterner::default(),
            parameters: ListInterner::default(),
            generic_arguments: ListInterner::default(),
            index_signatures: ListInterner::default(),
            strings: ListInterner::default(),
            operations: ValueInterner::new(),
            signatures: ValueInterner::new(),
            members: ValueInterner::new(),
            refinements: ValueInterner::new(),
            borrows: ValueInterner::new(),
        }
    }

    /// Intern one type whose payload lists are already interned, with its joined child flags.
    pub fn intern_type(&mut self, ty: Type, child_flags: TypeFlags) -> LocalTypeId {
        self.intern_type_inserted(ty, child_flags).0
    }

    /// Intern one type entry, returning whether this call inserted it.
    pub fn intern_type_inserted(
        &mut self,
        ty: Type,
        child_flags: TypeFlags,
    ) -> (LocalTypeId, bool) {
        // probe the committed segments beneath this tail, skipping pass-local variables
        let hash = fx_hash(&ty);
        if !matches!(ty, Type::Variable(_))
            && let Some(slots) = self.committed_index.get(&hash)
        {
            for slot in slots {
                let committed = self
                    .committed
                    .iter()
                    .find_map(|base| base.get_type_maybe(*slot));
                if committed.as_ref() == Some(&ty) {
                    return (*slot, false);
                }
            }
        }

        // probe the index for an existing structural hit
        if let Some(slots) = self.index.get(&hash) {
            for slot in slots {
                if self.segment.owned_type(*slot) == &ty {
                    return (*slot, false);
                }
            }
        }

        // allocate and index the new slot
        let type_id = self.segment.allocate_type(ty, child_flags);
        self.hashes.push(hash);
        self.index.entry(hash).or_default().push(type_id);

        (type_id, true)
    }

    /// Intern one type operation payload.
    pub fn intern_operation(&mut self, operation: TypeOperation) -> TypeOperationId {
        self.operations
            .intern_with(&mut self.segment.operations, operation, |id| {
                self.committed
                    .iter()
                    .find_map(|base| base.operations.get(id).copied())
            })
    }

    /// Intern one function signature payload.
    pub fn intern_signature(&mut self, signature: FunctionSignatureType) -> FunctionSignatureId {
        self.signatures
            .intern_with(&mut self.segment.signatures, signature, |id| {
                self.committed
                    .iter()
                    .find_map(|base| base.signatures.get(id).copied())
            })
    }

    /// Intern one member projection payload.
    pub fn intern_member(&mut self, member: MemberType) -> MemberTypeId {
        self.members
            .intern_with(&mut self.segment.members, member, |id| {
                self.committed
                    .iter()
                    .find_map(|base| base.members.get(id).copied())
            })
    }

    /// Intern one refined application payload.
    pub fn intern_refined(&mut self, refined: RefinedType) -> RefinedTypeId {
        self.refinements
            .intern_with(&mut self.segment.refinements, refined, |id| {
                self.committed
                    .iter()
                    .find_map(|base| base.refinements.get(id).copied())
            })
    }

    /// Intern one borrow form payload.
    pub fn intern_borrow(&mut self, borrow: BorrowForm) -> BorrowFormId {
        self.borrows
            .intern_with(&mut self.segment.borrows, borrow, |id| {
                self.committed
                    .iter()
                    .find_map(|base| base.borrows.get(id).copied())
            })
    }

    /// Intern one type id list.
    pub fn intern_type_ids(&mut self, values: &[GlobalTypeId]) -> TypeListId {
        intern_list(
            &mut self.type_ids,
            &self.lists.type_ids,
            &self.committed,
            |base| &base.type_ids,
            values,
        )
    }

    /// Intern one tuple element list.
    pub fn intern_elements(&mut self, values: &[TypeElement]) -> TypeListId {
        intern_list(
            &mut self.elements,
            &self.lists.elements,
            &self.committed,
            |base| &base.elements,
            values,
        )
    }

    /// Intern one shape property list.
    pub fn intern_properties(&mut self, values: &[TypeProperty]) -> TypeListId {
        intern_list(
            &mut self.properties,
            &self.lists.properties,
            &self.committed,
            |base| &base.properties,
            values,
        )
    }

    /// Intern one function parameter list.
    pub fn intern_parameters(&mut self, values: &[FunctionParameterType]) -> TypeListId {
        intern_list(
            &mut self.parameters,
            &self.lists.parameters,
            &self.committed,
            |base| &base.parameters,
            values,
        )
    }

    /// Intern one generic argument binding list.
    pub fn intern_generic_arguments(&mut self, values: &[GenericArgumentBinding]) -> TypeListId {
        intern_list(
            &mut self.generic_arguments,
            &self.lists.generic_arguments,
            &self.committed,
            |base| &base.generic_arguments,
            values,
        )
    }

    /// Intern one index signature list.
    pub fn intern_index_signatures(&mut self, values: &[TypeIndexSignature]) -> TypeListId {
        intern_list(
            &mut self.index_signatures,
            &self.lists.index_signatures,
            &self.committed,
            |base| &base.index_signatures,
            values,
        )
    }

    /// Intern one string list.
    pub fn intern_strings(&mut self, values: &[StringId]) -> TypeListId {
        intern_list(
            &mut self.strings,
            &self.lists.strings,
            &self.committed,
            |base| &base.strings,
            values,
        )
    }
}

/// Intern one list into a kind's pool, reusing content the committed segments already hold.
fn intern_list<T: Copy + Eq + Hash>(
    interner: &mut ListInterner,
    arena: &ListArena<T>,
    committed: &[Arc<TypeSegment>],
    select: impl Fn(&TypeSegment) -> &ListPool<T>,
    values: &[T],
) -> TypeListId {
    interner.intern(
        arena,
        |list, values| {
            committed
                .iter()
                .any(|base| select(base).get_maybe(list) == Some(values))
        },
        values,
    )
}
