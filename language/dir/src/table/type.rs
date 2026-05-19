use std::sync::Arc;

use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{
    Addressability, Arena, BindingTable, EnumBackingType, EnumFieldValue, Extension,
    GlobalNodeIdAny, GlobalSymbolId, IntersectionType, LocalExtensionId, LocalNodeId,
    LocalNodeIdAny, LocalTypeId, Node, SegmentView, StaticTerm, SymbolForm, Type, UnionType,
    VarianceModifier,
};

/// Cumulative type slots and relations for one DIR module.
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

    /// Iterate type attachments keyed by DIR node.
    pub fn node_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &NodeEntry)> + '_ {
        self.segments
            .iter()
            .enumerate()
            .flat_map(move |(segment_index, segment)| {
                segment.nodes.iter().filter_map(move |(node_id, entry)| {
                    let is_shadowed = self
                        .segments
                        .iter()
                        .skip(segment_index + 1)
                        .any(|segment| segment.nodes.contains_key(node_id));

                    (!is_shadowed).then_some((*node_id, entry))
                })
            })
    }

    /// Iterate type attachments keyed by DIR symbol.
    pub fn symbol_entries(&self) -> impl Iterator<Item = (GlobalSymbolId, &SymbolEntry)> + '_ {
        self.segments
            .iter()
            .enumerate()
            .flat_map(move |(segment_index, segment)| {
                segment
                    .symbols
                    .iter()
                    .filter_map(move |(symbol_id, entry)| {
                        let is_shadowed = self
                            .segments
                            .iter()
                            .skip(segment_index + 1)
                            .any(|segment| segment.symbols.contains_key(symbol_id));

                        (!is_shadowed).then_some((*symbol_id, entry))
                    })
            })
    }

    /// Get an extension by its id.
    pub fn get_extension(&self, extension_id: LocalExtensionId) -> &Extension {
        for segment in self.segments.iter() {
            if let Some(extension) = segment.get_local_extension(extension_id) {
                return extension;
            }
        }

        panic!("DIR extension {extension_id:?} is not visible")
    }

    /// Get an extension id by its symbol.
    pub fn symbol_extension_id(
        &self,
        extension_symbol: GlobalSymbolId,
    ) -> Option<LocalExtensionId> {
        self.symbol_entry(extension_symbol)
            .and_then(|entry| entry.extension_id)
    }

    /// Iterate extensions targeting a specific type symbol.
    pub fn target_extensions(
        &self,
        target_symbol: GlobalSymbolId,
    ) -> impl Iterator<Item = LocalExtensionId> + '_ {
        self.segments.iter().flat_map(move |segment| {
            segment
                .target_extensions(target_symbol)
                .into_iter()
                .flatten()
                .copied()
        })
    }

    /// Iterate over all extensions.
    pub fn iter_extensions(&self) -> impl Iterator<Item = (LocalExtensionId, &Extension)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.iter_extensions())
    }

    /// Get the declared type id for a node.
    pub fn get_declared_type_id(&self, node_id: GlobalNodeIdAny) -> Option<LocalTypeId> {
        self.node_entry(node_id).and_then(|entry| entry.declared)
    }

    /// Get the inferred type id for a node.
    pub fn get_inferred_type_id(&self, node_id: GlobalNodeIdAny) -> Option<LocalTypeId> {
        self.node_entry(node_id).and_then(|entry| entry.inferred)
    }

    /// Get the declared or inferred type id for a node.
    pub fn get_declared_or_inferred_type_id(
        &self,
        node_id: GlobalNodeIdAny,
    ) -> Option<LocalTypeId> {
        let entry = self.node_entry(node_id)?;

        entry.declared.or(entry.inferred)
    }

    /// Get the member receiver type id for a node.
    pub fn member_receiver_type_id(&self, node_id: GlobalNodeIdAny) -> Option<LocalTypeId> {
        self.node_entry(node_id).and_then(|entry| entry.receiver)
    }

    /// Get the contextual object type id for a node.
    pub fn contextual_object_type_id(&self, node_id: GlobalNodeIdAny) -> Option<LocalTypeId> {
        self.node_entry(node_id).and_then(|entry| entry.contextual)
    }

    /// Get the signature type id for a node.
    pub fn signature_type_id(&self, node_id: GlobalNodeIdAny) -> Option<LocalTypeId> {
        self.node_entry(node_id).and_then(|entry| entry.signature)
    }

    /// Get the addressability for a node.
    pub fn get_addressability(&self, node_id: GlobalNodeIdAny) -> Option<Addressability> {
        self.node_entry(node_id)
            .and_then(|entry| entry.addressability)
    }

    /// Get the instance type id for a symbol.
    pub fn get_instance_type_id(&self, symbol_id: GlobalSymbolId) -> Option<LocalTypeId> {
        self.symbol_entry(symbol_id)
            .and_then(|entry| entry.instance_type)
    }

    /// Find the symbol that owns an instance type id.
    pub fn symbol_for_instance_type(
        &self,
        instance_type_id: LocalTypeId,
    ) -> Option<GlobalSymbolId> {
        self.symbol_entries().find_map(|(symbol, entry)| {
            (entry.instance_type == Some(instance_type_id)).then_some(symbol)
        })
    }

    /// Get the value type id for a symbol.
    pub fn get_value_type_id(&self, symbol_id: GlobalSymbolId) -> Option<LocalTypeId> {
        self.symbol_entry(symbol_id)
            .and_then(|entry| entry.value_type)
    }

    /// Get the declared target type id for an alias symbol.
    pub fn get_alias_target_type_id(&self, symbol_id: GlobalSymbolId) -> Option<LocalTypeId> {
        self.symbol_entry(symbol_id)
            .and_then(|entry| entry.alias_target_type)
    }

    /// Get the backing type for an enum symbol.
    pub fn get_enum_backing_type(&self, symbol_id: GlobalSymbolId) -> Option<EnumBackingType> {
        self.symbol_entry(symbol_id)
            .and_then(|entry| entry.enum_backing)
    }

    /// Get the resolved enum field value.
    pub fn get_enum_field_value(&self, symbol_id: GlobalSymbolId) -> Option<EnumFieldValue> {
        self.symbol_entry(symbol_id)
            .and_then(|entry| entry.enum_field_value)
    }

    /// Get the type id for a symbol through its declaration form.
    pub fn symbol_type_id(
        &self,
        symbols: &BindingTable<'_>,
        symbol_id: GlobalSymbolId,
    ) -> Option<LocalTypeId> {
        let symbol = symbols.get_symbol(symbol_id.local_id);

        match symbol.form {
            SymbolForm::TypeAlias => self.get_alias_target_type_id(symbol_id),
            SymbolForm::Newtype => self
                .get_instance_type_id(symbol_id)
                .or_else(|| self.get_alias_target_type_id(symbol_id)),
            _ => self
                .get_value_type_id(symbol_id)
                .or_else(|| self.get_instance_type_id(symbol_id)),
        }
    }

    /// Get a type by its id.
    pub fn get_type(&self, type_id: LocalTypeId) -> &Type {
        self.get_type_maybe(type_id)
            .unwrap_or_else(|| panic!("DIR type {type_id:?} is not visible"))
    }

    /// Get a type by its id when present.
    pub fn get_type_maybe(&self, type_id: LocalTypeId) -> Option<&Type> {
        for segment in self.segments.iter().rev() {
            if let Some(ty) = segment.get_type_maybe(type_id) {
                return Some(ty);
            }
        }

        None
    }

    /// Strip outer form types to reach the payload type id.
    pub fn unwrap_form_payload_type_id(&self, type_id: LocalTypeId) -> LocalTypeId {
        let mut current = type_id;
        loop {
            match self.get_type(current) {
                Type::Form(form) => current = form.value,
                _ => return current,
            }
        }
    }

    /// Iterate over all type ids.
    pub fn iter_type_ids(&self) -> impl Iterator<Item = LocalTypeId> + '_ {
        let end = self.type_count();

        (0..end).map(LocalTypeId::new)
    }

    /// Return the origin for a type id.
    pub fn type_origin(&self, type_id: LocalTypeId) -> TypeOrigin {
        self.type_source(type_id).origin
    }

    /// Get the source id for a type.
    pub fn get_type_source(&self, type_id: LocalTypeId) -> LocalNodeIdAny {
        self.type_source(type_id).source_id
    }

    /// Return true when a type originated from an imported module.
    pub fn is_imported_type(&self, type_id: LocalTypeId) -> bool {
        matches!(self.type_origin(type_id), TypeOrigin::Imported)
    }

    /// Get the number of types in the table.
    pub fn type_count(&self) -> u32 {
        self.segments
            .last()
            .map(|segment| segment.type_count())
            .unwrap_or(0)
    }

    /// Get the number of extensions in the table.
    pub fn extension_count(&self) -> u32 {
        self.segments
            .last()
            .map(|segment| segment.extension_count())
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

    /// Return a node entry when present.
    fn node_entry(&self, node_id: GlobalNodeIdAny) -> Option<&NodeEntry> {
        for segment in self.segments.iter().rev() {
            if let Some(entry) = segment.node_entry(node_id) {
                return Some(entry);
            }
        }

        None
    }

    /// Return a symbol entry when present.
    fn symbol_entry(&self, symbol_id: GlobalSymbolId) -> Option<&SymbolEntry> {
        for segment in self.segments.iter().rev() {
            if let Some(entry) = segment.symbol_entry(symbol_id) {
                return Some(entry);
            }
        }

        None
    }

    /// Return the source for a type id.
    fn type_source(&self, type_id: LocalTypeId) -> TypeSource {
        for segment in self.segments.iter() {
            if segment.contains_type_id(type_id) {
                return segment.type_source(type_id);
            }
        }

        panic!("missing type source for type id {type_id:?}");
    }
}

/// Type slots and relations added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeSegment {
    /// The module id of the type store.
    pub module_id: ModuleId,
    /// The first type id owned by this table segment.
    pub(crate) first_type_id: u32,
    /// The first extension id owned by this table segment.
    pub(crate) first_extension_id: u32,
    /// Canonical type entries.
    pub(crate) types: Arena<Type>,

    /// The source for each type id.
    pub(crate) sources: Arena<TypeSource>,
    /// Type attachments keyed by DIR node.
    pub(crate) nodes: IndexMap<GlobalNodeIdAny, NodeEntry>,
    /// Type attachments keyed by DIR symbol.
    pub(crate) symbols: IndexMap<GlobalSymbolId, SymbolEntry>,

    /// Extension records.
    pub(crate) extensions: Arena<Extension>,
    /// Extension ids by target symbol.
    pub(crate) extensions_by_target_symbol: IndexMap<GlobalSymbolId, Vec<LocalExtensionId>>,
}

impl TypeSegment {
    /// Create a new type segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            first_type_id: 0,
            first_extension_id: 0,
            types: Arena::new(),
            sources: Arena::new(),
            nodes: IndexMap::new(),
            symbols: IndexMap::new(),
            extensions: Arena::new(),
            extensions_by_target_symbol: IndexMap::new(),
        }
    }

    /// Create a new empty segment after an existing type table segment.
    pub fn from_base(base: &Self) -> Self {
        Self {
            module_id: base.module_id,
            first_type_id: base.type_count(),
            first_extension_id: base.extension_count(),
            types: Arena::new(),
            sources: Arena::new(),
            nodes: IndexMap::new(),
            symbols: IndexMap::new(),
            extensions: Arena::new(),
            extensions_by_target_symbol: IndexMap::new(),
        }
    }

    /// Allocate one type slot with canonical metadata.
    fn allocate_type(
        &mut self,
        ty: Type,
        source_id: LocalNodeIdAny,
        origin: TypeOrigin,
    ) -> LocalTypeId {
        self.assert_type_table_invariants_debug("allocate_type:start");

        let type_id = LocalTypeId::new(self.type_count());

        self.types.allocate(ty);
        self.sources.allocate(TypeSource { source_id, origin });
        self.assert_type_table_invariants_debug("allocate_type:end");

        type_id
    }

    /// Insert a type derived from some source node.
    pub fn insert_type_from<T: Node>(&mut self, ty: Type, node_id: LocalNodeId<T>) -> LocalTypeId {
        self.allocate_type(ty, node_id.into_any(), TypeOrigin::Local)
    }

    /// Insert a type derived from some source node (any node type).
    pub fn insert_type_from_any(&mut self, ty: Type, node_id: LocalNodeIdAny) -> LocalTypeId {
        self.allocate_type(ty, node_id, TypeOrigin::Local)
    }

    /// Insert a type that originates from an imported module.
    pub fn insert_imported_type_from_any(
        &mut self,
        ty: Type,
        node_id: LocalNodeIdAny,
    ) -> LocalTypeId {
        self.allocate_type(ty, node_id, TypeOrigin::Imported)
    }

    /// Insert a type derived from another type id.
    pub fn insert_type_from_type(&mut self, ty: Type, source_type_id: LocalTypeId) -> LocalTypeId {
        let source_id = self.get_type_source(source_type_id);
        let origin = self.type_origin(source_type_id);

        self.allocate_type(ty, source_id, origin)
    }

    /// Intern one union type by deterministic structural scan.
    pub fn intern_union_type(
        &mut self,
        source_type_id: LocalTypeId,
        elements: Vec<LocalTypeId>,
    ) -> LocalTypeId {
        for type_id in self.iter_type_ids() {
            if let Type::Union(existing) = self.get_type(type_id)
                && existing.elements == elements
            {
                return type_id;
            }
        }

        self.insert_type_from_type(Type::Union(UnionType { elements }), source_type_id)
    }

    /// Intern one intersection type by deterministic structural scan.
    pub fn intern_intersection_type(
        &mut self,
        source_type_id: LocalTypeId,
        elements: Vec<LocalTypeId>,
    ) -> LocalTypeId {
        for type_id in self.iter_type_ids() {
            if let Type::Intersection(existing) = self.get_type(type_id)
                && existing.elements == elements
            {
                return type_id;
            }
        }

        self.insert_type_from_type(
            Type::Intersection(IntersectionType { elements }),
            source_type_id,
        )
    }

    /// Set generic parameter metadata for a parameter symbol.
    pub fn set_generic_parameter(
        &mut self,
        symbol_id: GlobalSymbolId,
        parameter: GenericParameterEntry,
    ) {
        self.symbol_entry_mut(symbol_id).generic_parameter = Some(parameter);
    }

    /// Get generic parameter metadata for a parameter symbol.
    pub fn generic_parameter(&self, symbol_id: GlobalSymbolId) -> Option<&GenericParameterEntry> {
        self.symbol_entry(symbol_id)
            .and_then(|entry| entry.generic_parameter.as_ref())
    }

    /// Set generic parameter symbols for a generic declaration symbol.
    pub fn set_generic_parameter_symbols(
        &mut self,
        symbol_id: GlobalSymbolId,
        symbols: Vec<GlobalSymbolId>,
    ) {
        self.symbol_entry_mut(symbol_id).generic_parameter_symbols = Some(symbols);
    }

    /// Get generic parameter symbols for a generic declaration symbol.
    pub fn generic_parameter_symbols(
        &self,
        symbol_id: GlobalSymbolId,
    ) -> Option<&[GlobalSymbolId]> {
        self.symbol_entry(symbol_id)
            .and_then(|entry| entry.generic_parameter_symbols.as_deref())
    }

    /// Iterate type attachments keyed by DIR node.
    pub fn node_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &NodeEntry)> + '_ {
        self.nodes.iter().map(|(node_id, entry)| (*node_id, entry))
    }

    /// Iterate type attachments keyed by DIR symbol.
    pub fn symbol_entries(&self) -> impl Iterator<Item = (GlobalSymbolId, &SymbolEntry)> + '_ {
        self.iter_symbol_entries()
    }

    /// Insert a new extension.
    pub fn insert_extension(&mut self, extension: Extension) -> LocalExtensionId {
        let extension_id = LocalExtensionId::new(self.extension_count());
        let extension_symbol = extension.symbol;

        // index by target symbol for member lookup
        let target = extension.target;
        self.extensions_by_target_symbol
            .entry(target)
            .or_default()
            .push(extension_id);

        // attach to the declaring symbol
        self.symbol_entry_mut(extension_symbol).extension_id = Some(extension_id);

        self.extensions.allocate(extension);

        extension_id
    }

    /// Get an extension by its id.
    pub fn get_extension(&self, extension_id: LocalExtensionId) -> &Extension {
        self.get_local_extension(extension_id).unwrap_or_else(|| {
            panic!("DIR extension {extension_id:?} is not allocated in this segment")
        })
    }

    /// Get an extension id by its symbol.
    pub fn symbol_extension_id(
        &self,
        extension_symbol: GlobalSymbolId,
    ) -> Option<LocalExtensionId> {
        self.symbol_entry(extension_symbol)
            .and_then(|entry| entry.extension_id)
    }

    /// Get a mutable extension by its id.
    pub fn get_extension_mut(&mut self, extension_id: LocalExtensionId) -> &mut Extension {
        assert!(
            self.contains_extension_id(extension_id),
            "DIR extension {extension_id:?} is not mutable in this segment"
        );

        let slot = extension_id.0 - self.first_extension_id;

        self.extensions.get_mut(slot)
    }

    /// Get all extensions targeting a specific type symbol.
    pub fn target_extensions(
        &self,
        target_symbol: GlobalSymbolId,
    ) -> Option<&Vec<LocalExtensionId>> {
        self.extensions_by_target_symbol.get(&target_symbol)
    }

    /// Iterate over all extensions.
    pub fn iter_extensions(&self) -> impl Iterator<Item = (LocalExtensionId, &Extension)> + '_ {
        (self.first_extension_id..self.extension_count()).map(|index| {
            let extension_id = LocalExtensionId::new(index);
            (extension_id, self.get_extension(extension_id))
        })
    }

    /// Set the declared type for a node.
    pub fn set_declared_type(&mut self, node_id: GlobalNodeIdAny, ty: LocalTypeId) {
        self.node_entry_mut(node_id).declared = Some(ty);
    }

    /// Get the declared type id for a node.
    pub fn get_declared_type_id(&self, node_id: GlobalNodeIdAny) -> Option<LocalTypeId> {
        self.node_entry(node_id).and_then(|entry| entry.declared)
    }

    /// Set the inferred type for a node.
    pub fn set_inferred_type(&mut self, node_id: GlobalNodeIdAny, ty: LocalTypeId) {
        self.node_entry_mut(node_id).inferred = Some(ty);
    }

    /// Get the inferred type id for a node.
    pub fn get_inferred_type_id(&self, node_id: GlobalNodeIdAny) -> Option<LocalTypeId> {
        self.node_entry(node_id).and_then(|entry| entry.inferred)
    }

    /// Get the declared or inferred type id for a node.
    pub fn get_declared_or_inferred_type_id(
        &self,
        node_id: GlobalNodeIdAny,
    ) -> Option<LocalTypeId> {
        let entry = self.node_entry(node_id)?;

        entry.declared.or(entry.inferred)
    }

    /// Set the member receiver type for a node.
    pub fn set_member_receiver_type(&mut self, node_id: GlobalNodeIdAny, ty: LocalTypeId) {
        self.node_entry_mut(node_id).receiver = Some(ty);
    }

    /// Get the member receiver type id for a node.
    pub fn member_receiver_type_id(&self, node_id: GlobalNodeIdAny) -> Option<LocalTypeId> {
        self.node_entry(node_id).and_then(|entry| entry.receiver)
    }

    /// Set the contextual object type for a node.
    pub fn set_contextual_object_type(&mut self, node_id: GlobalNodeIdAny, ty: LocalTypeId) {
        self.node_entry_mut(node_id).contextual = Some(ty);
    }

    /// Get the contextual object type id for a node.
    pub fn contextual_object_type_id(&self, node_id: GlobalNodeIdAny) -> Option<LocalTypeId> {
        self.node_entry(node_id).and_then(|entry| entry.contextual)
    }

    /// Set the signature type for a node.
    pub fn set_signature_type(&mut self, node_id: GlobalNodeIdAny, ty: LocalTypeId) {
        self.node_entry_mut(node_id).signature = Some(ty);
    }

    /// Get the signature type id for a node.
    pub fn signature_type_id(&self, node_id: GlobalNodeIdAny) -> Option<LocalTypeId> {
        self.node_entry(node_id).and_then(|entry| entry.signature)
    }

    /// Set the addressability for a node.
    pub fn set_addressability(&mut self, node_id: GlobalNodeIdAny, addressability: Addressability) {
        self.node_entry_mut(node_id).addressability = Some(addressability);
    }

    /// Get the addressability for a node.
    pub fn get_addressability(&self, node_id: GlobalNodeIdAny) -> Option<Addressability> {
        self.node_entry(node_id)
            .and_then(|entry| entry.addressability)
    }

    /// Copy node-owned relations from one node to another.
    pub fn copy_node_relations(&mut self, source: GlobalNodeIdAny, target: GlobalNodeIdAny) {
        if let Some(entry) = self.node_entry(source).cloned() {
            self.nodes.insert(target, entry);
        }
    }

    /// Set the instance type for a symbol.
    pub fn set_instance_type(&mut self, symbol_id: GlobalSymbolId, ty: LocalTypeId) {
        self.symbol_entry_mut(symbol_id).instance_type = Some(ty);
    }

    /// Get the instance type id for a symbol.
    pub fn get_instance_type_id(&self, symbol_id: GlobalSymbolId) -> Option<LocalTypeId> {
        self.symbol_entry(symbol_id)
            .and_then(|entry| entry.instance_type)
    }

    /// Find the symbol that owns an instance type id.
    pub fn symbol_for_instance_type(
        &self,
        instance_type_id: LocalTypeId,
    ) -> Option<GlobalSymbolId> {
        self.iter_symbol_entries().find_map(|(symbol, entry)| {
            (entry.instance_type == Some(instance_type_id)).then_some(symbol)
        })
    }

    /// Set the value type for a symbol.
    pub fn set_value_type(&mut self, symbol_id: GlobalSymbolId, ty: LocalTypeId) {
        self.symbol_entry_mut(symbol_id).value_type = Some(ty);
    }

    /// Get the value type id for a symbol.
    pub fn get_value_type_id(&self, symbol_id: GlobalSymbolId) -> Option<LocalTypeId> {
        self.symbol_entry(symbol_id)
            .and_then(|entry| entry.value_type)
    }

    /// Set the declared target type id for an alias symbol.
    pub fn set_alias_target_type_id(&mut self, symbol_id: GlobalSymbolId, ty: LocalTypeId) {
        self.symbol_entry_mut(symbol_id).alias_target_type = Some(ty);
    }

    /// Get the declared target type id for an alias symbol.
    pub fn get_alias_target_type_id(&self, symbol_id: GlobalSymbolId) -> Option<LocalTypeId> {
        self.symbol_entry(symbol_id)
            .and_then(|entry| entry.alias_target_type)
    }

    /// Set the backing type for an enum symbol.
    pub fn set_enum_backing_type(&mut self, symbol_id: GlobalSymbolId, backing: EnumBackingType) {
        self.symbol_entry_mut(symbol_id).enum_backing = Some(backing);
    }

    /// Get the backing type for an enum symbol.
    pub fn get_enum_backing_type(&self, symbol_id: GlobalSymbolId) -> Option<EnumBackingType> {
        self.symbol_entry(symbol_id)
            .and_then(|entry| entry.enum_backing)
    }

    /// Set the resolved enum field value for a symbol.
    pub fn set_enum_field_value(&mut self, symbol_id: GlobalSymbolId, value: EnumFieldValue) {
        self.symbol_entry_mut(symbol_id).enum_field_value = Some(value);
    }

    /// Get the resolved enum field value for a symbol.
    pub fn get_enum_field_value(&self, symbol_id: GlobalSymbolId) -> Option<EnumFieldValue> {
        self.symbol_entry(symbol_id)
            .and_then(|entry| entry.enum_field_value)
    }

    /// Get the type id for a symbol through its declaration form.
    pub fn symbol_type_id(
        &self,
        symbols: &BindingTable<'_>,
        symbol_id: GlobalSymbolId,
    ) -> Option<LocalTypeId> {
        let symbol = symbols.get_symbol(symbol_id.local_id);

        match symbol.form {
            SymbolForm::TypeAlias => self.get_alias_target_type_id(symbol_id),
            SymbolForm::Newtype => self
                .get_instance_type_id(symbol_id)
                .or_else(|| self.get_alias_target_type_id(symbol_id)),
            _ => self
                .get_value_type_id(symbol_id)
                .or_else(|| self.get_instance_type_id(symbol_id)),
        }
    }

    /// Get a type by its id.
    pub fn get_type(&self, type_id: LocalTypeId) -> &Type {
        self.get_type_maybe(type_id)
            .unwrap_or_else(|| panic!("DIR type {type_id:?} is not allocated in this segment"))
    }

    /// Get a type by its id when present.
    pub fn get_type_maybe(&self, type_id: LocalTypeId) -> Option<&Type> {
        self.contains_type_id(type_id)
            .then(|| self.types.get(type_id.0 - self.first_type_id))
    }

    /// Strip outer form types to reach the payload type id.
    pub fn unwrap_form_payload_type_id(&self, type_id: LocalTypeId) -> LocalTypeId {
        let mut current = type_id;
        loop {
            match self.get_type(current) {
                Type::Form(form) => current = form.value,
                _ => return current,
            }
        }
    }

    /// Iterate over all type ids.
    pub fn iter_type_ids(&self) -> impl Iterator<Item = LocalTypeId> + '_ {
        let end = self.type_count();

        (self.first_type_id..end).map(LocalTypeId::new)
    }

    /// Get a mutable type by its id.
    pub fn get_type_mut(&mut self, type_id: LocalTypeId) -> &mut Type {
        assert!(
            self.contains_type_id(type_id),
            "DIR type {type_id:?} is not mutable in this segment"
        );

        self.types.get_mut(type_id.0 - self.first_type_id)
    }

    /// Return the source for a type id.
    fn type_source(&self, type_id: LocalTypeId) -> TypeSource {
        if self.contains_type_id(type_id) {
            let slot = type_id.0 - self.first_type_id;
            return *self.sources.get(slot);
        }

        panic!("missing type source for type id {type_id:?}");
    }

    /// Update a type in place.
    pub fn update_type(&mut self, type_id: LocalTypeId, ty: Type) {
        self.assert_type_table_invariants_debug("update_type:start");
        *self.get_type_mut(type_id) = ty;
        self.assert_type_table_invariants_debug("update_type:end");
    }

    /// Get the source id for a type.
    pub fn get_type_source(&self, type_id: LocalTypeId) -> LocalNodeIdAny {
        self.type_source(type_id).source_id
    }

    /// Return the origin for a type.
    pub fn type_origin(&self, type_id: LocalTypeId) -> TypeOrigin {
        self.type_source(type_id).origin
    }

    /// Return true when a type originated from an imported module.
    pub fn is_imported_type(&self, type_id: LocalTypeId) -> bool {
        matches!(self.type_origin(type_id), TypeOrigin::Imported)
    }

    /// Get the number of types in the table.
    pub fn type_count(&self) -> u32 {
        self.first_type_id + self.types.len() as u32
    }

    /// Get the number of extensions in the table.
    pub fn extension_count(&self) -> u32 {
        self.first_extension_id + self.extensions.len() as u32
    }

    /// Return the number of entries in this table.
    pub fn len(&self) -> u32 {
        self.type_count()
    }

    /// Return true when this table has no entries.
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    /// Get an extension owned by this table segment.
    pub(crate) fn get_local_extension(&self, extension_id: LocalExtensionId) -> Option<&Extension> {
        self.contains_extension_id(extension_id).then(|| {
            self.extensions
                .get(extension_id.0 - self.first_extension_id)
        })
    }

    /// Return whether this segment contains the given type id.
    fn contains_type_id(&self, type_id: LocalTypeId) -> bool {
        type_id.0 >= self.first_type_id && type_id.0 < self.type_count()
    }

    /// Return whether this segment contains the given extension id.
    fn contains_extension_id(&self, extension_id: LocalExtensionId) -> bool {
        extension_id.0 >= self.first_extension_id && extension_id.0 < self.extension_count()
    }

    /// Return a node entry when present.
    fn node_entry(&self, node_id: GlobalNodeIdAny) -> Option<&NodeEntry> {
        self.nodes.get(&node_id)
    }

    /// Return a mutable node entry, creating it when needed.
    fn node_entry_mut(&mut self, node_id: GlobalNodeIdAny) -> &mut NodeEntry {
        self.nodes.entry(node_id).or_default()
    }

    /// Return a symbol entry when present.
    fn symbol_entry(&self, symbol_id: GlobalSymbolId) -> Option<&SymbolEntry> {
        self.symbols.get(&symbol_id)
    }

    /// Return a mutable symbol entry, creating it when needed.
    fn symbol_entry_mut(&mut self, symbol_id: GlobalSymbolId) -> &mut SymbolEntry {
        self.symbols.entry(symbol_id).or_default()
    }

    /// Iterate visible symbol entries with local overrides applied.
    fn iter_symbol_entries(&self) -> impl Iterator<Item = (GlobalSymbolId, &SymbolEntry)> + '_ {
        self.symbols
            .iter()
            .map(|(symbol_id, entry)| (*symbol_id, entry))
    }

    /// Assert internal table invariants only in debug builds.
    fn assert_type_table_invariants_debug(&self, _context: &str) {
        #[cfg(debug_assertions)]
        self.assert_type_table_invariants(_context);
    }

    /// Assert internal store invariants.
    #[cfg(debug_assertions)]
    fn assert_type_table_invariants(&self, context: &str) {
        let type_slot_count = self.types.len();
        let source_slot_count = self.sources.len();

        assert_eq!(
            source_slot_count, type_slot_count,
            "type source slot mismatch in {context}: source={source_slot_count}, types={type_slot_count}",
        );
    }
}

/// The argument space accepted by one generic parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenericParameterSpace {
    /// The parameter accepts a type argument.
    Type,
    /// The parameter accepts a value argument.
    Value,
    /// The parameter accepts a lifetime argument.
    Lifetime,
}

/// Type entry for one generic parameter symbol.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GenericParameterEntry {
    /// The argument space accepted by this parameter.
    pub space: GenericParameterSpace,
    /// The constraint type for this parameter.
    pub constraint: Option<LocalTypeId>,
    /// The variance for this parameter.
    pub variance: Option<VarianceModifier>,
}

/// Type attachments recorded for one DIR node.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeEntry {
    /// The declared type.
    pub declared: Option<LocalTypeId>,
    /// The inferred type.
    pub inferred: Option<LocalTypeId>,
    /// The normalized member receiver type.
    pub receiver: Option<LocalTypeId>,
    /// The contextual object type.
    pub contextual: Option<LocalTypeId>,
    /// The signature type.
    pub signature: Option<LocalTypeId>,
    /// The addressability of the node result.
    pub addressability: Option<Addressability>,
}

/// Type attachments recorded for one DIR symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SymbolEntry {
    /// Type metadata when this symbol declares a generic parameter.
    pub generic_parameter: Option<GenericParameterEntry>,
    /// Generic parameter symbols declared by this symbol.
    pub generic_parameter_symbols: Option<Vec<GlobalSymbolId>>,

    /// The instance type for nominal declarations.
    pub instance_type: Option<LocalTypeId>,
    /// The value type for value declarations.
    pub value_type: Option<LocalTypeId>,
    /// The target type for alias declarations.
    pub alias_target_type: Option<LocalTypeId>,
    /// Static constant value for comptime declarations.
    pub static_value: Option<StaticTerm>,

    /// The backing type for enum declarations.
    pub enum_backing: Option<EnumBackingType>,
    /// The resolved enum field value.
    pub enum_field_value: Option<EnumFieldValue>,

    /// The extension declared by this symbol.
    pub extension_id: Option<LocalExtensionId>,
}

/// The origin of one type slot in the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeOrigin {
    /// The type was produced in the local module.
    Local,
    /// The type was imported from another module.
    Imported,
}

/// The source stored for one type id.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TypeSource {
    /// The source node id that produced this type slot.
    pub source_id: LocalNodeIdAny,
    /// The origin of this type slot.
    pub origin: TypeOrigin,
}
