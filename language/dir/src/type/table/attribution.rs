use std::collections::HashSet;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{
    Addressability, EnumBackingType, EnumFieldValue, GlobalNodeIdAny, GlobalSymbolId, LocalTypeId,
    StaticExpression, SymbolTable, Type,
};

use super::TypeTable;

/// Runtime type check strategy for a guard expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeCheckKind {
    /// The runtime check was reduced to a constant.
    Constant(bool),
    /// The runtime check uses a union tag.
    UnionTag,
    /// The runtime check compares type identities.
    TypeDescriptor,
}

/// Type attribution ownership for nodes and symbols.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributionTable {
    /// The declared type by node id.
    pub(crate) declared_type_by_node_id: IndexMap<GlobalNodeIdAny, LocalTypeId>,
    /// The inferred type by node id.
    pub(crate) inferred_type_by_node_id: IndexMap<GlobalNodeIdAny, LocalTypeId>,
    /// The signature type by node id (separate from declared types).
    pub(crate) signature_type_by_node_id: IndexMap<GlobalNodeIdAny, LocalTypeId>,
    /// The addressability by node id.
    pub(crate) addressability_by_node_id: IndexMap<GlobalNodeIdAny, Addressability>,
    /// The runtime check kind for guard expressions.
    #[serde(skip)]
    pub(crate) runtime_check_kind_by_node_id: IndexMap<GlobalNodeIdAny, RuntimeCheckKind>,
    /// The instance type by symbol id (for type declarations: the shape of instances).
    pub(crate) instance_type_by_symbol_id: IndexMap<GlobalSymbolId, LocalTypeId>,
    /// The value type by symbol id (the type when used as a value).
    pub(crate) value_type_by_symbol_id: IndexMap<GlobalSymbolId, LocalTypeId>,
    /// Declare-published static constant values by symbol id.
    pub(crate) published_static_constant_value_by_symbol_id:
        IndexMap<GlobalSymbolId, StaticExpression>,
    /// The target type id for alias symbols (the declared alias value type).
    pub(crate) alias_target_type_by_symbol_id: IndexMap<GlobalSymbolId, LocalTypeId>,
    /// The backing type of enum symbols.
    pub(crate) enum_backing_type_by_symbol_id: IndexMap<GlobalSymbolId, EnumBackingType>,
    /// The resolved enum field values by enum field symbol.
    pub(crate) enum_field_value_by_symbol_id: IndexMap<GlobalSymbolId, EnumFieldValue>,
    /// Symbols that still violate associated requirement implementation contracts.
    #[serde(default)]
    pub(crate) symbols_with_unimplemented_associated_requirements: HashSet<GlobalSymbolId>,
    /// Associated comptime member symbols whose declared value depends on projection syntax.
    #[serde(default)]
    pub(crate) symbols_with_associated_comptime_projection_dependencies: HashSet<GlobalSymbolId>,
}

impl AttributionTable {
    /// Create an empty type attribution table.
    pub fn new() -> Self {
        Self {
            declared_type_by_node_id: IndexMap::new(),
            inferred_type_by_node_id: IndexMap::new(),
            signature_type_by_node_id: IndexMap::new(),
            addressability_by_node_id: IndexMap::new(),
            runtime_check_kind_by_node_id: IndexMap::new(),
            instance_type_by_symbol_id: IndexMap::new(),
            value_type_by_symbol_id: IndexMap::new(),
            published_static_constant_value_by_symbol_id: IndexMap::new(),
            alias_target_type_by_symbol_id: IndexMap::new(),
            enum_backing_type_by_symbol_id: IndexMap::new(),
            enum_field_value_by_symbol_id: IndexMap::new(),
            symbols_with_unimplemented_associated_requirements: HashSet::new(),
            symbols_with_associated_comptime_projection_dependencies: HashSet::new(),
        }
    }
}

impl Default for AttributionTable {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeTable {
    /// Record a runtime check kind for a guard expression.
    pub fn set_runtime_check_kind(&mut self, node_id: GlobalNodeIdAny, kind: RuntimeCheckKind) {
        self.attribution
            .runtime_check_kind_by_node_id
            .insert(node_id, kind);
    }

    /// Get the runtime check kind for a guard expression.
    pub fn get_runtime_check_kind(&self, node_id: GlobalNodeIdAny) -> Option<RuntimeCheckKind> {
        self.attribution
            .runtime_check_kind_by_node_id
            .get(&node_id)
            .copied()
    }

    /// Set the declared type for a node (from type annotation).
    pub fn set_declared_type(&mut self, node_id: GlobalNodeIdAny, ty: LocalTypeId) {
        self.attribution
            .declared_type_by_node_id
            .insert(node_id, ty);
    }

    /// Get the declared type for a node.
    pub fn get_declared_type(&self, node_id: GlobalNodeIdAny) -> Option<&Type> {
        self.attribution
            .declared_type_by_node_id
            .get(&node_id)
            .map(|ty| self.types.get(ty.0))
    }

    /// Get the declared type id for a node.
    pub fn get_declared_type_id(&self, node_id: GlobalNodeIdAny) -> Option<LocalTypeId> {
        self.attribution
            .declared_type_by_node_id
            .get(&node_id)
            .copied()
    }

    /// Iterate over declared type ids keyed by node id.
    pub fn iter_declared_type_ids(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, LocalTypeId)> + '_ {
        self.attribution
            .declared_type_by_node_id
            .iter()
            .map(|(node_id, type_id)| (*node_id, *type_id))
    }

    /// Set the inferred type for a node.
    pub fn set_inferred_type(&mut self, node_id: GlobalNodeIdAny, ty: LocalTypeId) {
        self.attribution
            .inferred_type_by_node_id
            .insert(node_id, ty);
    }

    /// Clear all cached inferred types.
    pub fn clear_inferred_types(&mut self) {
        self.attribution.inferred_type_by_node_id.clear();
    }

    /// Clear cached expression addressability metadata.
    pub fn clear_addressability(&mut self) {
        self.attribution.addressability_by_node_id.clear();
    }

    /// Get the inferred type for a node.
    pub fn get_inferred_type(&self, node_id: GlobalNodeIdAny) -> Option<&Type> {
        self.attribution
            .inferred_type_by_node_id
            .get(&node_id)
            .map(|ty| self.types.get(ty.0))
    }

    /// Get the inferred type id for a node.
    pub fn get_inferred_type_id(&self, node_id: GlobalNodeIdAny) -> Option<LocalTypeId> {
        self.attribution
            .inferred_type_by_node_id
            .get(&node_id)
            .copied()
    }

    /// Iterate over inferred type ids keyed by node id.
    pub fn iter_inferred_type_ids(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, LocalTypeId)> + '_ {
        self.attribution
            .inferred_type_by_node_id
            .iter()
            .map(|(node_id, type_id)| (*node_id, *type_id))
    }

    /// Get declared or fallback to inferred type for a node.
    pub fn get_declared_or_inferred_type(&self, node_id: GlobalNodeIdAny) -> Option<&Type> {
        self.get_declared_type(node_id)
            .or_else(|| self.get_inferred_type(node_id))
    }

    /// Get declared or fallback to inferred type id for a node.
    pub fn get_declared_or_inferred_type_id(
        &self,
        node_id: GlobalNodeIdAny,
    ) -> Option<LocalTypeId> {
        self.get_declared_type_id(node_id)
            .or_else(|| self.get_inferred_type_id(node_id))
    }

    /// Set the signature type for a declaration or member node.
    pub fn set_signature_type_for_node(&mut self, node_id: GlobalNodeIdAny, ty: LocalTypeId) {
        self.attribution
            .signature_type_by_node_id
            .insert(node_id, ty);
    }

    /// Get the signature type id for a declaration or member node.
    pub fn get_signature_type_for_node(&self, node_id: GlobalNodeIdAny) -> Option<LocalTypeId> {
        self.attribution
            .signature_type_by_node_id
            .get(&node_id)
            .copied()
    }

    /// Iterate over signature type ids keyed by node id.
    pub fn iter_signature_type_ids(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, LocalTypeId)> + '_ {
        self.attribution
            .signature_type_by_node_id
            .iter()
            .map(|(node_id, type_id)| (*node_id, *type_id))
    }

    /// Set the addressability for a node.
    pub fn set_addressability_for_node(
        &mut self,
        node_id: GlobalNodeIdAny,
        addressability: Addressability,
    ) {
        self.attribution
            .addressability_by_node_id
            .insert(node_id, addressability);
    }

    /// Get the addressability for a node.
    pub fn get_addressability_for_node(&self, node_id: GlobalNodeIdAny) -> Option<Addressability> {
        self.attribution
            .addressability_by_node_id
            .get(&node_id)
            .copied()
    }

    /// Copy node-local analysis data from a source node to a target node.
    /// Copies declared types, inferred types, signature types, addressability, instances, and resolutions.
    pub fn copy_node_analysis(&mut self, source: GlobalNodeIdAny, target: GlobalNodeIdAny) {
        // declared type
        if let Some(declared_type) = self
            .attribution
            .declared_type_by_node_id
            .get(&source)
            .copied()
        {
            self.attribution
                .declared_type_by_node_id
                .insert(target, declared_type);
        }

        // inferred type
        if let Some(inferred_type) = self
            .attribution
            .inferred_type_by_node_id
            .get(&source)
            .copied()
        {
            self.attribution
                .inferred_type_by_node_id
                .insert(target, inferred_type);
        }

        // signature type
        if let Some(signature_type) = self
            .attribution
            .signature_type_by_node_id
            .get(&source)
            .copied()
        {
            self.attribution
                .signature_type_by_node_id
                .insert(target, signature_type);
        }

        // addressability
        if let Some(addressability) = self
            .attribution
            .addressability_by_node_id
            .get(&source)
            .copied()
        {
            self.attribution
                .addressability_by_node_id
                .insert(target, addressability);
        }

        // instance
        if let Some(instance_id) = self.instance.instance_by_node_id.get(&source).copied() {
            self.instance
                .instance_by_node_id
                .insert(target, instance_id);
        }

        // resolution
        if let Some(resolution_id) = self.resolution.resolution_by_node_id.get(&source).copied() {
            self.resolution
                .resolution_by_node_id
                .insert(target, resolution_id);
        }
    }

    /// Set the instance type for a symbol (what type instances of this type have).
    pub fn set_instance_type(&mut self, symbol_id: GlobalSymbolId, ty: LocalTypeId) {
        self.attribution
            .instance_type_by_symbol_id
            .insert(symbol_id, ty);
        self.bump_symbol_version(symbol_id);
    }

    /// Get the instance type for a symbol.
    pub fn get_instance_type(&self, symbol_id: GlobalSymbolId) -> Option<&Type> {
        self.attribution
            .instance_type_by_symbol_id
            .get(&symbol_id)
            .map(|ty| self.types.get(ty.0))
    }

    /// Get the instance type id for a symbol.
    pub fn get_instance_type_id(&self, symbol_id: GlobalSymbolId) -> Option<LocalTypeId> {
        self.attribution
            .instance_type_by_symbol_id
            .get(&symbol_id)
            .copied()
    }

    /// Find the symbol that owns an instance type id.
    pub fn symbol_for_instance_type(
        &self,
        instance_type_id: LocalTypeId,
    ) -> Option<GlobalSymbolId> {
        self.attribution
            .instance_type_by_symbol_id
            .iter()
            .find_map(|(symbol, ty_id)| (*ty_id == instance_type_id).then_some(*symbol))
    }

    /// Set the value type for a symbol (what type this symbol has when used as a value).
    pub fn set_value_type(&mut self, symbol_id: GlobalSymbolId, ty: LocalTypeId) {
        self.attribution
            .value_type_by_symbol_id
            .insert(symbol_id, ty);
        self.bump_symbol_version(symbol_id);
    }

    /// Get the value type for a symbol.
    pub fn get_value_type(&self, symbol_id: GlobalSymbolId) -> Option<&Type> {
        self.attribution
            .value_type_by_symbol_id
            .get(&symbol_id)
            .map(|ty| self.types.get(ty.0))
    }

    /// Get the value type id for a symbol.
    pub fn get_value_type_id(&self, symbol_id: GlobalSymbolId) -> Option<LocalTypeId> {
        self.attribution
            .value_type_by_symbol_id
            .get(&symbol_id)
            .copied()
    }

    /// Store one artifact static constant value for one symbol.
    pub fn set_artifact_static_constant_value(
        &mut self,
        symbol_id: GlobalSymbolId,
        value: StaticExpression,
    ) {
        self.attribution
            .published_static_constant_value_by_symbol_id
            .insert(symbol_id, value);
    }

    /// Query one artifact static constant value for one symbol.
    pub fn query_artifact_static_constant_value(
        &self,
        symbol_id: GlobalSymbolId,
    ) -> Option<StaticExpression> {
        self.attribution
            .published_static_constant_value_by_symbol_id
            .get(&symbol_id)
            .cloned()
    }

    /// Mark one symbol as having unresolved associated implementation requirements.
    pub fn mark_symbol_with_unimplemented_associated_requirements(
        &mut self,
        symbol_id: GlobalSymbolId,
    ) {
        self.attribution
            .symbols_with_unimplemented_associated_requirements
            .insert(symbol_id);
        self.bump_symbol_version(symbol_id);
    }

    /// Return true when one symbol has unresolved associated implementation requirements.
    pub fn symbol_has_unimplemented_associated_requirements(
        &self,
        symbol_id: GlobalSymbolId,
    ) -> bool {
        self.attribution
            .symbols_with_unimplemented_associated_requirements
            .contains(&symbol_id)
    }

    /// Mark one associated comptime member symbol as projection-dependent in declared static evaluation.
    pub fn mark_symbol_with_associated_comptime_projection_dependencies(
        &mut self,
        symbol_id: GlobalSymbolId,
    ) {
        self.attribution
            .symbols_with_associated_comptime_projection_dependencies
            .insert(symbol_id);
        self.bump_symbol_version(symbol_id);
    }

    /// Return true when one associated comptime member symbol depends on projection syntax.
    pub fn symbol_has_associated_comptime_projection_dependencies(
        &self,
        symbol_id: GlobalSymbolId,
    ) -> bool {
        self.attribution
            .symbols_with_associated_comptime_projection_dependencies
            .contains(&symbol_id)
    }

    /// Get the declared or inferred type id for a symbol in this module.
    pub fn get_type_id_for_symbol(
        &self,
        symbols: &SymbolTable,
        symbol_id: GlobalSymbolId,
    ) -> Option<LocalTypeId> {
        // prefer cached value types
        if let Some(value_type_id) = self.get_value_type_id(symbol_id) {
            return Some(value_type_id);
        }

        // skip non local symbols
        if symbol_id.module_id != self.module_id {
            return None;
        }

        // fall back to declared or inferred declaration types
        let symbol = symbols.get_symbol(symbol_id.local_id);
        let primary_declaration = symbol.primary_declaration?;
        self.get_declared_or_inferred_type_id(primary_declaration)
    }

    /// Set the declared target type id for an alias symbol.
    pub fn set_alias_target_type_id(&mut self, symbol_id: GlobalSymbolId, ty: LocalTypeId) {
        self.attribution
            .alias_target_type_by_symbol_id
            .insert(symbol_id, ty);
        self.bump_symbol_version(symbol_id);
    }

    /// Get the declared target type id for an alias symbol.
    pub fn get_alias_target_type_id(&self, symbol_id: GlobalSymbolId) -> Option<LocalTypeId> {
        self.attribution
            .alias_target_type_by_symbol_id
            .get(&symbol_id)
            .copied()
    }

    /// Set the enum backing type for a symbol.
    pub fn set_enum_backing_type(
        &mut self,
        symbol_id: GlobalSymbolId,
        backing_type: EnumBackingType,
    ) {
        self.attribution
            .enum_backing_type_by_symbol_id
            .insert(symbol_id, backing_type);
    }

    /// Get the enum backing type for a symbol.
    pub fn get_enum_backing_type(&self, symbol_id: GlobalSymbolId) -> Option<EnumBackingType> {
        self.attribution
            .enum_backing_type_by_symbol_id
            .get(&symbol_id)
            .copied()
    }

    /// Set the enum field value for a symbol.
    pub fn set_enum_field_value(&mut self, symbol_id: GlobalSymbolId, value: EnumFieldValue) {
        self.attribution
            .enum_field_value_by_symbol_id
            .insert(symbol_id, value);
    }

    /// Get the enum field value for a symbol.
    pub fn get_enum_field_value(&self, symbol_id: GlobalSymbolId) -> Option<EnumFieldValue> {
        self.attribution
            .enum_field_value_by_symbol_id
            .get(&symbol_id)
            .copied()
    }
}
