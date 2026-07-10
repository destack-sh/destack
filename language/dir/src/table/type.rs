use std::hash::{Hash, Hasher};
use std::sync::Arc;

use destack_serde::Reflect;
use indexmap::IndexMap;
use rustc_hash::{FxHashMap, FxHasher};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use destack_core::{Arena, StringId};
use destack_source::ModuleId;

use crate::{
    Form, FunctionParameterType, GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, LocalTypeId,
    SegmentView, Type, TypeElement, TypeField, TypeFlags, TypeIndexSignature, TypeListId,
    TypeOperation,
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
        let mut entries = IndexMap::new();

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
        let mut entries = IndexMap::new();

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
        let mut entries = IndexMap::new();

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
        let mut current = type_id;
        // follow reductions until they reach a fixed point
        let mut seen = Vec::new();
        while !seen.contains(&current) {
            seen.push(current);
            let Some(reduced) = self.get_direct_reduced_type_id(current) else {
                return current;
            };
            current = reduced;
        }

        panic!("DIR type reduction cycle contains {current:?}");
    }

    /// Get the directly stored reduced type id for a checked type.
    fn get_direct_reduced_type_id(&self, type_id: GlobalTypeId) -> Option<GlobalTypeId> {
        for segment in self.segments.iter().rev() {
            if let Some(reduced) = segment.get_reduced_type_id(type_id) {
                return Some(reduced);
            }
        }

        None
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

    /// Get one type id list.
    pub fn type_ids(&self, list: TypeListId) -> &[GlobalTypeId] {
        self.slice(list, |segment| &segment.type_ids)
    }

    /// Get one tuple element list.
    pub fn elements(&self, list: TypeListId) -> &[TypeElement] {
        self.slice(list, |segment| &segment.elements)
    }

    /// Get one shape field list.
    pub fn fields(&self, list: TypeListId) -> &[TypeField] {
        self.slice(list, |segment| &segment.fields)
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
            | Type::Object
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
            Type::Instance(instance) => {
                for child in self.type_ids(instance.arguments) {
                    visit(*child);
                }
            }
            Type::Member(member) => {
                visit(member.owner);
                for child in self.type_ids(member.arguments) {
                    visit(*child);
                }
                if let Some(qualifier) = member.qualifier {
                    visit(qualifier);
                }
            }
            Type::EnumMember(member) => visit(member.owner),
            Type::Refined(refined) => {
                visit(refined.base);
                visit(refined.value);
            }

            // memory forms
            Type::Form(form) => {
                visit(form.value);
                match &form.form {
                    Form::Borrowed { lifetime, access } => {
                        visit(*lifetime);
                        visit(*access);
                    }
                    Form::Placed { place } => visit(*place),
                    Form::Managed | Form::Owned | Form::Raw | Form::Readonly => {}
                }
            }
            Type::Dynamic(dynamic) => visit(dynamic.constraint),

            // type operations
            Type::Operation(operation) => match operation {
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

            // structural shapes
            Type::Shape(shape) => {
                for field in self.fields(shape.fields) {
                    visit(field.ty);
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
                visit(function.environment);
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

/// Type slots added by one DIR phase.
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
    /// The shape field lists referenced by type payloads.
    pub(crate) fields: ListPool<TypeField>,
    /// The function parameter lists referenced by type payloads.
    pub(crate) parameters: ListPool<FunctionParameterType>,
    /// The index signature lists referenced by type payloads.
    pub(crate) index_signatures: ListPool<TypeIndexSignature>,
    /// The string lists referenced by type payloads.
    pub(crate) strings: ListPool<StringId>,

    /// The intern index from value hash to owned type slots.
    #[serde(skip)]
    index: FxHashMap<u64, SmallVec<[LocalTypeId; 1]>>,
    /// The value hash per owned type, parallel to `types`.
    #[serde(skip)]
    hashes: Vec<u64>,

    /// Effective checked type keyed by DIR node occurrence.
    pub(crate) node_types: IndexMap<GlobalNodeIdAny, GlobalTypeId>,
    /// Checked declaration type keyed by symbol.
    pub(crate) symbol_types: IndexMap<GlobalSymbolId, GlobalTypeId>,
    /// Reduced checked type keyed by surface type.
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
    /// The shape field element count at the mark.
    fields: u32,
    /// The function parameter element count at the mark.
    parameters: u32,
    /// The index signature element count at the mark.
    index_signatures: u32,
    /// The string element count at the mark.
    strings: u32,
}

impl TypeSegment {
    /// Create a new type segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            first_type_id: 0,
            types: Arena::new(),
            flags: Arena::new(),
            type_ids: ListPool::new(0),
            elements: ListPool::new(0),
            fields: ListPool::new(0),
            parameters: ListPool::new(0),
            index_signatures: ListPool::new(0),
            strings: ListPool::new(0),
            index: FxHashMap::default(),
            hashes: Vec::new(),
            node_types: IndexMap::new(),
            symbol_types: IndexMap::new(),
            reduced_types: IndexMap::new(),
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
            fields: ListPool::new(base.fields.element_count()),
            parameters: ListPool::new(base.parameters.element_count()),
            index_signatures: ListPool::new(base.index_signatures.element_count()),
            strings: ListPool::new(base.strings.element_count()),
            index: FxHashMap::default(),
            hashes: Vec::new(),
            node_types: IndexMap::new(),
            symbol_types: IndexMap::new(),
            reduced_types: IndexMap::new(),
        }
    }

    /// Intern one type whose payload lists are already interned.
    /// The caller supplies the joined structural flags of every child type.
    pub fn intern_type(&mut self, ty: Type, child_flags: TypeFlags) -> LocalTypeId {
        // probe the index for an existing structural hit
        let hash = fx_hash(&ty);
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

    /// Intern one type id list.
    pub fn intern_type_ids(&mut self, values: &[GlobalTypeId]) -> TypeListId {
        self.type_ids.intern(values)
    }

    /// Intern one tuple element list.
    pub fn intern_elements(&mut self, values: &[TypeElement]) -> TypeListId {
        self.elements.intern(values)
    }

    /// Intern one shape field list.
    pub fn intern_fields(&mut self, values: &[TypeField]) -> TypeListId {
        self.fields.intern(values)
    }

    /// Intern one function parameter list.
    pub fn intern_parameters(&mut self, values: &[FunctionParameterType]) -> TypeListId {
        self.parameters.intern(values)
    }

    /// Intern one index signature list.
    pub fn intern_index_signatures(&mut self, values: &[TypeIndexSignature]) -> TypeListId {
        self.index_signatures.intern(values)
    }

    /// Intern one string list.
    pub fn intern_strings(&mut self, values: &[StringId]) -> TypeListId {
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
    pub fn set_node_type(&mut self, node_id: GlobalNodeIdAny, ty: GlobalTypeId) {
        self.node_types.insert(node_id, ty);
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

    /// Set the reduced type for one checked surface type.
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

    /// Get one shape field list when this segment owns it.
    pub fn fields_maybe(&self, list: TypeListId) -> Option<&[TypeField]> {
        self.fields.get_maybe(list)
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
            fields: self.fields.element_count(),
            parameters: self.parameters.element_count(),
            index_signatures: self.index_signatures.element_count(),
            strings: self.strings.element_count(),
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
        self.fields.truncate_to(mark.fields);
        self.parameters.truncate_to(mark.parameters);
        self.index_signatures.truncate_to(mark.index_signatures);
        self.strings.truncate_to(mark.strings);
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
