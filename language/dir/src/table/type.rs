use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{
    Addressability, Arena, ControlResolution, DependencyResolution, EnumBackingType,
    EnumFieldValue, Extension, GlobalNodeIdAny, GlobalSymbolId, Instantiation, IntersectionType,
    Lineage, LiteralType, LocalExtensionId, LocalInstantiationId, LocalLineageId, LocalNodeId,
    LocalNodeIdAny, LocalTypeId, Node, Resolution, StaticExpression, SymbolForm, SymbolTable, Type,
    TypeLiteral, UnionType, VarianceModifier,
};

/// Append-only type slots and relations for one DIR artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeTable {
    /// The module id of the type store.
    pub module_id: ModuleId,
    /// Canonical type entries.
    pub(crate) types: Arena<Type>,

    /// The provenance for each type id.
    pub(crate) provenances: Vec<TypeProvenance>,
    /// Semantic attachments keyed by DIR node.
    pub(crate) nodes: IndexMap<GlobalNodeIdAny, NodeEntry>,
    /// Semantic attachments keyed by DIR symbol.
    pub(crate) symbols: IndexMap<GlobalSymbolId, SymbolEntry>,

    /// Interned generic instantiations.
    pub(crate) instantiations: Arena<Instantiation>,

    /// Nominal lineage records.
    pub(crate) lineages: Arena<Lineage>,

    /// Extension records.
    pub(crate) extensions: Arena<Extension>,
    /// Extension ids by target symbol.
    pub(crate) extensions_by_target_symbol: IndexMap<GlobalSymbolId, Vec<LocalExtensionId>>,
}

impl TypeTable {
    /// Create a new TypeTable.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            types: Arena::new(),
            provenances: Vec::new(),
            nodes: IndexMap::new(),
            symbols: IndexMap::new(),
            instantiations: Arena::new(),
            lineages: Arena::new(),
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

        let type_id = LocalTypeId::new(self.types.len() as u32);

        self.types.allocate(ty);
        self.provenances.push(TypeProvenance { source_id, origin });
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

    /// Get or insert a literal type id.
    pub fn intern_literal_type(
        &mut self,
        source_type_id: LocalTypeId,
        literal: TypeLiteral,
    ) -> LocalTypeId {
        // reuse an existing literal type when available
        for (index, ty) in self.types.iter().enumerate() {
            if let Type::Literal(value) = ty
                && value.value == literal
            {
                return LocalTypeId::new(index as u32);
            }
        }

        self.insert_type_from_type(
            Type::Literal(LiteralType { value: literal }),
            source_type_id,
        )
    }

    /// Intern one union type by deterministic structural scan.
    pub fn intern_union_type(
        &mut self,
        source_type_id: LocalTypeId,
        elements: Vec<LocalTypeId>,
    ) -> LocalTypeId {
        for (index, ty) in self.types.iter().enumerate() {
            if let Type::Union(existing) = ty
                && existing.elements == elements
            {
                return LocalTypeId::new(index as u32);
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
        for (index, ty) in self.types.iter().enumerate() {
            if let Type::Intersection(existing) = ty
                && existing.elements == elements
            {
                return LocalTypeId::new(index as u32);
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

    /// Insert a generic instantiation.
    pub fn insert_instantiation(&mut self, instantiation: Instantiation) -> LocalInstantiationId {
        // reuse existing exact instantiations by deterministic scan
        if let Some(instantiation_id) = self.find_instantiation(&instantiation) {
            return instantiation_id;
        }

        let instantiation_id = LocalInstantiationId::new(self.instantiations.len() as u32);
        self.instantiations.allocate(instantiation);

        instantiation_id
    }

    /// Get an instantiation by id.
    pub fn get_instantiation(&self, instantiation_id: LocalInstantiationId) -> &Instantiation {
        self.instantiations.get(instantiation_id.0)
    }

    /// Iterate committed instantiations with their local ids.
    pub fn iter_instantiations(
        &self,
    ) -> impl Iterator<Item = (LocalInstantiationId, &Instantiation)> + '_ {
        self.instantiations
            .iter()
            .enumerate()
            .map(|(index, instantiation)| (LocalInstantiationId::new(index as u32), instantiation))
    }

    /// Attach an instantiation to a source node.
    pub fn set_node_instantiation(
        &mut self,
        node_id: GlobalNodeIdAny,
        instantiation_id: LocalInstantiationId,
    ) {
        self.node_entry_mut(node_id).instantiation = Some(instantiation_id);
    }

    /// Return the instantiation attached to a source node.
    pub fn node_instantiation_id(&self, node_id: GlobalNodeIdAny) -> Option<LocalInstantiationId> {
        self.node_entry(node_id)
            .and_then(|entry| entry.instantiation)
    }

    /// Find one exact instantiation by shape.
    pub fn find_instantiation(&self, expected: &Instantiation) -> Option<LocalInstantiationId> {
        for (index, instantiation) in self.instantiations.iter().enumerate() {
            if instantiation == expected {
                return Some(LocalInstantiationId::new(index as u32));
            }
        }

        None
    }

    /// Insert a new lineage.
    pub fn insert_lineage(&mut self, lineage: Lineage) -> LocalLineageId {
        let lineage_id = LocalLineageId::new(self.lineages.len() as u32);
        self.lineages.allocate(lineage);

        lineage_id
    }

    /// Get a lineage by its id.
    pub fn get_lineage(&self, lineage_id: LocalLineageId) -> &Lineage {
        self.lineages.get(lineage_id.0)
    }

    /// Get a mutable lineage by its id.
    pub fn get_lineage_mut(&mut self, lineage_id: LocalLineageId) -> &mut Lineage {
        self.lineages.get_mut(lineage_id.0)
    }

    /// Set the lineage for a symbol.
    pub fn set_symbol_lineage(&mut self, symbol_id: GlobalSymbolId, lineage_id: LocalLineageId) {
        self.symbol_entry_mut(symbol_id).lineage = Some(lineage_id);
    }

    /// Get the lineage id for a symbol.
    pub fn symbol_lineage_id(&self, symbol_id: GlobalSymbolId) -> Option<LocalLineageId> {
        self.symbol_entry(symbol_id).and_then(|entry| entry.lineage)
    }

    /// Get the lineage for a symbol directly.
    pub fn symbol_lineage(&self, symbol_id: GlobalSymbolId) -> Option<&Lineage> {
        self.symbol_lineage_id(symbol_id)
            .map(|id| self.lineages.get(id.0))
    }

    /// Iterate over all lineages with their associated symbol ids.
    pub fn iter_lineages(&self) -> impl Iterator<Item = (GlobalSymbolId, &Lineage)> {
        self.symbols.iter().filter_map(|(symbol_id, entry)| {
            entry
                .lineage
                .map(|lineage_id| (*symbol_id, self.lineages.get(lineage_id.0)))
        })
    }

    /// Insert a new extension.
    pub fn insert_extension(&mut self, extension: Extension) -> LocalExtensionId {
        let extension_id = LocalExtensionId::new(self.extensions.len() as u32);
        let extension_symbol = extension.symbol;

        // index by target symbol for member lookup
        let target = extension.target;
        self.extensions_by_target_symbol
            .entry(target)
            .or_default()
            .push(extension_id);

        // attach to the declaring symbol
        self.symbol_entry_mut(extension_symbol).extension = Some(extension_id);

        self.extensions.allocate(extension);

        extension_id
    }

    /// Get an extension by its id.
    pub fn get_extension(&self, extension_id: LocalExtensionId) -> &Extension {
        self.extensions.get(extension_id.0)
    }

    /// Get an extension id by its symbol.
    pub fn symbol_extension_id(
        &self,
        extension_symbol: GlobalSymbolId,
    ) -> Option<LocalExtensionId> {
        self.symbol_entry(extension_symbol)
            .and_then(|entry| entry.extension)
    }

    /// Get a mutable extension by its id.
    pub fn get_extension_mut(&mut self, extension_id: LocalExtensionId) -> &mut Extension {
        self.extensions.get_mut(extension_id.0)
    }

    /// Get all extensions targeting a specific type symbol.
    pub fn target_extensions(
        &self,
        target_symbol: GlobalSymbolId,
    ) -> Option<&Vec<LocalExtensionId>> {
        self.extensions_by_target_symbol.get(&target_symbol)
    }

    /// Iterate over all extensions.
    pub fn iter_extensions(&self) -> impl Iterator<Item = (LocalExtensionId, &Extension)> {
        self.extensions
            .iter()
            .enumerate()
            .map(|(index, extension)| (LocalExtensionId::new(index as u32), extension))
    }

    /// Set the semantic resolution for a node.
    pub fn set_resolution(&mut self, node_id: GlobalNodeIdAny, resolution: Resolution) {
        self.node_entry_mut(node_id).resolution = Some(resolution);
    }

    /// Get the semantic resolution for a node.
    pub fn resolution(&self, node_id: GlobalNodeIdAny) -> Option<&Resolution> {
        self.node_entry(node_id)
            .and_then(|entry| entry.resolution.as_ref())
    }

    /// Set the lexical symbol resolution for a node.
    pub fn set_symbol_resolution(&mut self, node_id: GlobalNodeIdAny, symbol_id: GlobalSymbolId) {
        self.set_resolution(node_id, Resolution::Symbol(symbol_id));
    }

    /// Get the lexical symbol resolution for a node.
    pub fn symbol_resolution(&self, node_id: GlobalNodeIdAny) -> Option<GlobalSymbolId> {
        match self.resolution(node_id) {
            Some(Resolution::Symbol(symbol_id)) => Some(*symbol_id),
            _ => None,
        }
    }

    /// Set the dependency resolution for a node.
    pub fn set_dependency_resolution(
        &mut self,
        node_id: GlobalNodeIdAny,
        resolution: DependencyResolution,
    ) {
        self.set_resolution(node_id, Resolution::Dependency(resolution));
    }

    /// Get the dependency resolution for a node.
    pub fn dependency_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&DependencyResolution> {
        match self.resolution(node_id) {
            Some(Resolution::Dependency(resolution)) => Some(resolution),
            _ => None,
        }
    }

    /// Set the control flow resolution for a node.
    pub fn set_control_resolution(
        &mut self,
        node_id: GlobalNodeIdAny,
        resolution: ControlResolution,
    ) {
        self.set_resolution(node_id, Resolution::Control(resolution));
    }

    /// Get the control flow resolution for a node.
    pub fn control_resolution(&self, node_id: GlobalNodeIdAny) -> Option<ControlResolution> {
        match self.resolution(node_id) {
            Some(Resolution::Control(resolution)) => Some(*resolution),
            _ => None,
        }
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
        if let Some(entry) = self.nodes.get(&source).cloned() {
            self.nodes.insert(target, entry);
        }
    }

    /// Set the instance type for a symbol.
    pub fn set_instance_type(&mut self, symbol_id: GlobalSymbolId, ty: LocalTypeId) {
        self.symbol_entry_mut(symbol_id).instance = Some(ty);
    }

    /// Get the instance type id for a symbol.
    pub fn get_instance_type_id(&self, symbol_id: GlobalSymbolId) -> Option<LocalTypeId> {
        self.symbol_entry(symbol_id)
            .and_then(|entry| entry.instance)
    }

    /// Find the symbol that owns an instance type id.
    pub fn symbol_for_instance_type(
        &self,
        instance_type_id: LocalTypeId,
    ) -> Option<GlobalSymbolId> {
        self.symbols.iter().find_map(|(symbol, entry)| {
            (entry.instance == Some(instance_type_id)).then_some(*symbol)
        })
    }

    /// Set the value type for a symbol.
    pub fn set_value_type(&mut self, symbol_id: GlobalSymbolId, ty: LocalTypeId) {
        self.symbol_entry_mut(symbol_id).value = Some(ty);
    }

    /// Get the value type id for a symbol.
    pub fn get_value_type_id(&self, symbol_id: GlobalSymbolId) -> Option<LocalTypeId> {
        self.symbol_entry(symbol_id).and_then(|entry| entry.value)
    }

    /// Set the declared target type id for an alias symbol.
    pub fn set_alias_target_type_id(&mut self, symbol_id: GlobalSymbolId, ty: LocalTypeId) {
        self.symbol_entry_mut(symbol_id).alias_target = Some(ty);
    }

    /// Get the declared target type id for an alias symbol.
    pub fn get_alias_target_type_id(&self, symbol_id: GlobalSymbolId) -> Option<LocalTypeId> {
        self.symbol_entry(symbol_id)
            .and_then(|entry| entry.alias_target)
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

    /// Get the type id for a symbol through its semantic form.
    pub fn symbol_type_id(
        &self,
        symbols: &SymbolTable,
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
        self.types.get(self.local_slot(type_id))
    }

    /// Get a type by its id when present.
    pub fn get_type_maybe(&self, type_id: LocalTypeId) -> Option<&Type> {
        self.types.get_maybe(self.local_slot(type_id))
    }

    /// Strip value wrapper types to reach the underlying type id.
    pub fn unwrap_value_type_id(&self, type_id: LocalTypeId) -> LocalTypeId {
        let mut current = type_id;
        loop {
            match self.get_type(current) {
                Type::Value(value) => current = value.value,
                _ => return current,
            }
        }
    }

    /// Iterate over all type ids.
    pub fn iter_type_ids(&self) -> impl Iterator<Item = LocalTypeId> + '_ {
        let end = self.type_count();

        (0..end).map(LocalTypeId::new)
    }

    /// Get a mutable type by its id.
    pub fn get_type_mut(&mut self, type_id: LocalTypeId) -> &mut Type {
        self.types.get_mut(self.local_slot(type_id))
    }

    /// Return provenance for a type id.
    fn type_provenance(&self, type_id: LocalTypeId) -> TypeProvenance {
        let slot = self.local_slot(type_id) as usize;

        self.provenances.get(slot).copied().unwrap_or_else(|| {
            panic!("missing type provenance for type id {type_id:?}");
        })
    }

    /// Update a type in place.
    pub fn update_type(&mut self, type_id: LocalTypeId, ty: Type) {
        self.assert_type_table_invariants_debug("update_type:start");
        *self.types.get_mut(self.local_slot(type_id)) = ty;
        self.assert_type_table_invariants_debug("update_type:end");
    }

    /// Get the source id for a type.
    pub fn get_type_source(&self, type_id: LocalTypeId) -> LocalNodeIdAny {
        self.type_provenance(type_id).source_id
    }

    /// Get the provenance for a type.
    pub fn type_origin(&self, type_id: LocalTypeId) -> TypeOrigin {
        self.type_provenance(type_id).origin
    }

    /// Return true when a type originated from an imported module.
    pub fn is_imported_type(&self, type_id: LocalTypeId) -> bool {
        matches!(self.type_origin(type_id), TypeOrigin::Imported)
    }

    /// Get the number of types in the table.
    pub fn type_count(&self) -> u32 {
        self.types.len() as u32
    }

    /// Return the number of entries in this table.
    pub fn len(&self) -> u32 {
        self.types.len() as u32
    }

    /// Return true when this table has no entries.
    pub fn is_empty(&self) -> bool {
        self.types.len() == 0
    }

    /// Convert a type id into a local slot.
    fn local_slot(&self, type_id: LocalTypeId) -> u32 {
        type_id.0
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

    /// Assert internal table invariants only in debug builds.
    fn assert_type_table_invariants_debug(&self, _context: &str) {
        #[cfg(debug_assertions)]
        self.assert_type_table_invariants(_context);
    }

    /// Assert internal store invariants.
    #[cfg(debug_assertions)]
    fn assert_type_table_invariants(&self, context: &str) {
        let type_slot_count = self.types.len();
        let provenance_slot_count = self.provenances.len();

        assert_eq!(
            provenance_slot_count, type_slot_count,
            "type provenance slot mismatch in {context}: provenance={provenance_slot_count}, types={type_slot_count}",
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

/// Semantic table entry for one generic parameter symbol.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GenericParameterEntry {
    /// The argument space accepted by this parameter.
    pub space: GenericParameterSpace,
    /// The constraint type for this parameter.
    pub constraint: Option<LocalTypeId>,
    /// The variance for this parameter.
    pub variance: Option<VarianceModifier>,
}

/// Semantic attachments recorded for one DIR node.
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
    /// The semantic resolution for this node.
    pub resolution: Option<Resolution>,
    /// The generic instantiation attached to this node.
    pub instantiation: Option<LocalInstantiationId>,
}

/// Semantic attachments recorded for one DIR symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SymbolEntry {
    /// Semantic metadata when this symbol declares a generic parameter.
    pub generic_parameter: Option<GenericParameterEntry>,
    /// Generic parameter symbols declared by this symbol.
    pub generic_parameter_symbols: Option<Vec<GlobalSymbolId>>,

    /// The instance type for nominal declarations.
    pub instance: Option<LocalTypeId>,
    /// The value type for value declarations.
    pub value: Option<LocalTypeId>,
    /// The target type for alias declarations.
    pub alias_target: Option<LocalTypeId>,
    /// Static constant value for comptime declarations.
    pub static_value: Option<StaticExpression>,

    /// The backing type for enum declarations.
    pub enum_backing: Option<EnumBackingType>,
    /// The resolved enum field value.
    pub enum_field_value: Option<EnumFieldValue>,

    /// The nominal lineage for this declaration.
    pub lineage: Option<LocalLineageId>,
    /// The extension declared by this symbol.
    pub extension: Option<LocalExtensionId>,
}

/// The provenance of one type slot in the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeOrigin {
    /// The type was produced in the local module.
    Local,
    /// The type was imported from another module.
    Imported,
}

/// The provenance stored for one type id.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TypeProvenance {
    /// The source node id that produced this type slot.
    pub source_id: LocalNodeIdAny,
    /// The provenance of this type slot.
    pub origin: TypeOrigin,
}

#[cfg(test)]
mod tests {
    use destack_source::ModuleId;

    use super::{TypeOrigin, TypeTable};
    use crate::{
        LiteralType, LocalNodeIdAny, NodeType, PrimitiveType, SliceType, Type, TypeLiteral,
    };

    #[test]
    fn test_insert_type_from_any_tracks_local_type_provenance() {
        let mut types = TypeTable::new(ModuleId::EPHEMERAL);
        let source_id = LocalNodeIdAny::new(7, NodeType::Expression);
        let type_id = types.insert_type_from_any(
            Type::Literal(LiteralType {
                value: TypeLiteral::Any,
            }),
            source_id,
        );

        assert_eq!(types.get_type_source(type_id), source_id);
        assert_eq!(types.type_origin(type_id), TypeOrigin::Local);
        assert!(!types.is_imported_type(type_id));
    }

    #[test]
    fn test_insert_imported_type_from_any_tracks_imported_type_provenance() {
        let mut types = TypeTable::new(ModuleId::EPHEMERAL);
        let source_id = LocalNodeIdAny::new(9, NodeType::Expression);
        let type_id = types.insert_imported_type_from_any(
            Type::Literal(LiteralType {
                value: TypeLiteral::Primitive(PrimitiveType::Number),
            }),
            source_id,
        );

        assert_eq!(types.get_type_source(type_id), source_id);
        assert_eq!(types.type_origin(type_id), TypeOrigin::Imported);
        assert!(types.is_imported_type(type_id));
    }

    #[test]
    fn test_insert_type_from_type_preserves_origin_and_source_metadata() {
        let mut types = TypeTable::new(ModuleId::EPHEMERAL);
        let source_id = LocalNodeIdAny::new(13, NodeType::Expression);
        let source_type_id = types.insert_imported_type_from_any(
            Type::Literal(LiteralType {
                value: TypeLiteral::Primitive(PrimitiveType::String),
            }),
            source_id,
        );

        let mapped_type_id = types.insert_type_from_type(
            Type::Slice(SliceType {
                element: Some(source_type_id),
                is_readonly: false,
            }),
            source_type_id,
        );

        assert_eq!(types.get_type_source(mapped_type_id), source_id);
        assert_eq!(types.type_origin(mapped_type_id), TypeOrigin::Imported);
    }

    #[test]
    fn test_update_type_preserves_type_provenance() {
        let mut types = TypeTable::new(ModuleId::EPHEMERAL);
        let source_id = LocalNodeIdAny::new(21, NodeType::Expression);
        let type_id = types.insert_type_from_any(
            Type::Literal(LiteralType {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean),
            }),
            source_id,
        );

        types.update_type(
            type_id,
            Type::Literal(LiteralType {
                value: TypeLiteral::Unknown,
            }),
        );

        assert_eq!(types.get_type_source(type_id), source_id);
        assert_eq!(types.type_origin(type_id), TypeOrigin::Local);
    }
}
