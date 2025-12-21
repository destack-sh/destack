use indexmap::IndexMap;

use destack_source::ModuleId;

use crate::{
    Arena, Extension, GlobalNodeIdAny, GlobalSymbolId, Instance, Lineage, LocalExtensionId,
    LocalInstanceId, LocalLineageId, LocalNodeId, LocalNodeIdAny, LocalResolutionId, LocalTypeId,
    Node, Resolution, Type,
};

/// TypeTable stores all type-related analysis results for a module. NOT THREAD-SAFE.
#[derive(Debug, Clone)]
pub struct TypeTable {
    /// The module id of the type table.
    pub module_id: ModuleId,

    // types
    /// The next type id to allocate.
    pub(crate) next_type_id: u32,
    /// The types.
    pub(crate) types: Arena<Type>,
    /// The source ids of all types. Index is the type id.
    pub(crate) source_id_by_type_id: Vec<LocalNodeIdAny>,

    // node types
    /// The declared type by node id (type annotations live on nodes).
    pub(crate) declared_type_by_node_id: IndexMap<GlobalNodeIdAny, LocalTypeId>,
    /// The inferred type by node id (expression-like types at specific locations).
    pub(crate) inferred_type_by_node_id: IndexMap<GlobalNodeIdAny, LocalTypeId>,

    // symbol types
    /// The instance type by symbol id (for type declarations: the shape of instances).
    pub(crate) instance_type_by_symbol_id: IndexMap<GlobalSymbolId, LocalTypeId>,
    /// The value type by symbol id (the type when used as a value).
    pub(crate) value_type_by_symbol_id: IndexMap<GlobalSymbolId, LocalTypeId>,

    // instances (statically parameterised types)
    /// The next instance id to allocate.
    pub(crate) next_instance_id: u32,
    /// The instances.
    pub(crate) instances: Arena<Instance>,
    /// The instance used by node ids.
    pub(crate) instance_by_node_id: IndexMap<GlobalNodeIdAny, LocalInstanceId>,

    // resolutions (types of members like functions/methods)
    /// The next resolution id to allocate.
    pub(crate) next_resolution_id: u32,
    /// The resolutions.
    pub(crate) resolutions: Arena<Resolution>,
    /// The resolution used by node ids.
    pub(crate) resolution_by_node_id: IndexMap<GlobalNodeIdAny, LocalResolutionId>,

    // lineages (resolved heritage for nominal types)
    /// The next lineage id to allocate.
    pub(crate) next_lineage_id: u32,
    /// The lineages.
    pub(crate) lineages: Arena<Lineage>,
    /// The lineage by symbol id (for type declarations: their resolved heritage).
    pub(crate) lineage_by_symbol_id: IndexMap<GlobalSymbolId, LocalLineageId>,

    // extensions (methods and implements added to existing types)
    /// The next extension id to allocate.
    pub(crate) next_extension_id: u32,
    /// The extensions.
    pub(crate) extensions: Arena<Extension>,
    /// Extensions by their declaration symbol (for lookup by extension symbol).
    pub(crate) extension_by_symbol: IndexMap<GlobalSymbolId, LocalExtensionId>,
    /// Extensions indexed by target symbol (for member lookup on types).
    pub(crate) extensions_by_target: IndexMap<GlobalSymbolId, Vec<LocalExtensionId>>,
}

impl TypeTable {
    /// Create a new TypeTable.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            // types
            next_type_id: 0,
            types: Arena::new(),
            source_id_by_type_id: Vec::new(),

            // node types
            declared_type_by_node_id: IndexMap::new(),
            inferred_type_by_node_id: IndexMap::new(),
            // symbol types
            instance_type_by_symbol_id: IndexMap::new(),
            value_type_by_symbol_id: IndexMap::new(),
            // instances
            next_instance_id: 0,
            instances: Arena::new(),
            instance_by_node_id: IndexMap::new(),
            // resolutions
            next_resolution_id: 0,
            resolutions: Arena::new(),
            resolution_by_node_id: IndexMap::new(),
            // lineages
            next_lineage_id: 0,
            lineages: Arena::new(),
            lineage_by_symbol_id: IndexMap::new(),
            // extensions
            next_extension_id: 0,
            extensions: Arena::new(),
            extension_by_symbol: IndexMap::new(),
            extensions_by_target: IndexMap::new(),
        }
    }

    /// Insert a type derived from some source node.
    pub fn insert_type_from<T: Node>(&mut self, ty: Type, node_id: LocalNodeId<T>) -> LocalTypeId {
        let type_id = LocalTypeId::new(self.next_type_id);
        self.next_type_id += 1;
        self.types.allocate(ty);
        self.source_id_by_type_id.push(node_id.into_any());
        type_id
    }

    /// Insert a type without a source node (for synthetic types).
    pub fn insert_type(&mut self, ty: Type) -> LocalTypeId {
        use crate::NodeType;
        let type_id = LocalTypeId::new(self.next_type_id);
        self.next_type_id += 1;
        self.types.allocate(ty);
        // use a sentinel value for synthetic types
        self.source_id_by_type_id
            .push(LocalNodeIdAny::new(u32::MAX, NodeType::Expression));
        type_id
    }

    /// Insert a type derived from some source node (any node type).
    pub fn insert_type_from_any(&mut self, ty: Type, node_id: LocalNodeIdAny) -> LocalTypeId {
        let type_id = LocalTypeId::new(self.next_type_id);
        self.next_type_id += 1;
        self.types.allocate(ty);
        self.source_id_by_type_id.push(node_id);
        type_id
    }

    /// Get a type by its id.
    pub fn get_type(&self, type_id: LocalTypeId) -> &Type {
        self.types.get(type_id.0)
    }

    /// Get a mutable type by its id.
    pub fn get_type_mut(&mut self, type_id: LocalTypeId) -> &mut Type {
        self.types.get_mut(type_id.0)
    }

    /// Get the source id for a type.
    pub fn get_type_source(&self, type_id: LocalTypeId) -> LocalNodeIdAny {
        self.source_id_by_type_id[type_id.0 as usize]
    }

    /// Get the number of types in the table.
    pub fn type_count(&self) -> u32 {
        self.next_type_id
    }

    /// Set the declared type for a node (from type annotation).
    pub fn set_declared_type(&mut self, node_id: GlobalNodeIdAny, ty: LocalTypeId) {
        self.declared_type_by_node_id.insert(node_id, ty);
    }

    /// Get the declared type for a node.
    pub fn get_declared_type(&self, node_id: GlobalNodeIdAny) -> Option<&Type> {
        self.declared_type_by_node_id
            .get(&node_id)
            .map(|ty| self.types.get(ty.0))
    }

    /// Get the declared type id for a node.
    pub fn get_declared_type_id(&self, node_id: GlobalNodeIdAny) -> Option<LocalTypeId> {
        self.declared_type_by_node_id.get(&node_id).copied()
    }

    /// Set the inferred type for a node (expression type at this location).
    pub fn set_inferred_type(&mut self, node_id: GlobalNodeIdAny, ty: LocalTypeId) {
        self.inferred_type_by_node_id.insert(node_id, ty);
    }

    /// Get the inferred type for a node.
    pub fn get_inferred_type(&self, node_id: GlobalNodeIdAny) -> Option<&Type> {
        self.inferred_type_by_node_id
            .get(&node_id)
            .map(|ty| self.types.get(ty.0))
    }

    /// Get the inferred type id for a node.
    pub fn get_inferred_type_id(&self, node_id: GlobalNodeIdAny) -> Option<LocalTypeId> {
        self.inferred_type_by_node_id.get(&node_id).copied()
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

    /// Set the instance type for a symbol (what type instances of this type have).
    pub fn set_instance_type(&mut self, symbol_id: GlobalSymbolId, ty: LocalTypeId) {
        self.instance_type_by_symbol_id.insert(symbol_id, ty);
    }

    /// Get the instance type for a symbol.
    pub fn get_instance_type(&self, symbol_id: GlobalSymbolId) -> Option<&Type> {
        self.instance_type_by_symbol_id
            .get(&symbol_id)
            .map(|ty| self.types.get(ty.0))
    }

    /// Get the instance type id for a symbol.
    pub fn get_instance_type_id(&self, symbol_id: GlobalSymbolId) -> Option<LocalTypeId> {
        self.instance_type_by_symbol_id.get(&symbol_id).copied()
    }

    /// Set the value type for a symbol (what type this symbol has when used as a value).
    pub fn set_value_type(&mut self, symbol_id: GlobalSymbolId, ty: LocalTypeId) {
        self.value_type_by_symbol_id.insert(symbol_id, ty);
    }

    /// Get the value type for a symbol.
    pub fn get_value_type(&self, symbol_id: GlobalSymbolId) -> Option<&Type> {
        self.value_type_by_symbol_id
            .get(&symbol_id)
            .map(|ty| self.types.get(ty.0))
    }

    /// Get the value type id for a symbol.
    pub fn get_value_type_id(&self, symbol_id: GlobalSymbolId) -> Option<LocalTypeId> {
        self.value_type_by_symbol_id.get(&symbol_id).copied()
    }

    /// Insert a new instance.
    pub fn insert_instance(&mut self, instance: Instance) -> LocalInstanceId {
        let instance_id = LocalInstanceId::new(self.next_instance_id);
        self.next_instance_id += 1;
        self.instances.allocate(instance);
        instance_id
    }

    /// Get an instance by its id.
    pub fn get_instance(&self, instance_id: LocalInstanceId) -> &Instance {
        self.instances.get(instance_id.0)
    }

    /// Get a mutable instance by its id.
    pub fn get_instance_mut(&mut self, instance_id: LocalInstanceId) -> &mut Instance {
        self.instances.get_mut(instance_id.0)
    }

    /// Set the instance used by a node id.
    pub fn set_instance_for_node(
        &mut self,
        node_id: GlobalNodeIdAny,
        instance_id: LocalInstanceId,
    ) {
        self.instance_by_node_id.insert(node_id, instance_id);
    }

    /// Get the instance used by a node id.
    pub fn get_instance_for_node(&self, node_id: GlobalNodeIdAny) -> Option<LocalInstanceId> {
        self.instance_by_node_id.get(&node_id).copied()
    }

    /// Insert a new resolution.
    pub fn insert_resolution(&mut self, resolution: Resolution) -> LocalResolutionId {
        let resolution_id = LocalResolutionId::new(self.next_resolution_id);
        self.next_resolution_id += 1;
        self.resolutions.allocate(resolution);
        resolution_id
    }

    /// Get a resolution by its id.
    pub fn get_resolution(&self, resolution_id: LocalResolutionId) -> &Resolution {
        self.resolutions.get(resolution_id.0)
    }

    /// Get a mutable resolution by its id.
    pub fn get_resolution_mut(&mut self, resolution_id: LocalResolutionId) -> &mut Resolution {
        self.resolutions.get_mut(resolution_id.0)
    }

    /// Set the resolution used by a node id.
    pub fn set_resolution_for_node(
        &mut self,
        node_id: GlobalNodeIdAny,
        resolution_id: LocalResolutionId,
    ) {
        self.resolution_by_node_id.insert(node_id, resolution_id);
    }

    /// Get the resolution used by a node id.
    pub fn get_resolution_for_node(&self, node_id: GlobalNodeIdAny) -> Option<LocalResolutionId> {
        self.resolution_by_node_id.get(&node_id).copied()
    }

    /// Insert a new lineage.
    pub fn insert_lineage(&mut self, lineage: Lineage) -> LocalLineageId {
        let lineage_id = LocalLineageId::new(self.next_lineage_id);
        self.next_lineage_id += 1;
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

    /// Set the lineage for a symbol (type declaration).
    pub fn set_lineage_for_symbol(
        &mut self,
        symbol_id: GlobalSymbolId,
        lineage_id: LocalLineageId,
    ) {
        self.lineage_by_symbol_id.insert(symbol_id, lineage_id);
    }

    /// Get the lineage id for a symbol.
    pub fn get_lineage_id_for_symbol(&self, symbol_id: GlobalSymbolId) -> Option<LocalLineageId> {
        self.lineage_by_symbol_id.get(&symbol_id).copied()
    }

    /// Get the lineage for a symbol directly.
    pub fn get_lineage_for_symbol(&self, symbol_id: GlobalSymbolId) -> Option<&Lineage> {
        self.lineage_by_symbol_id
            .get(&symbol_id)
            .map(|id| self.lineages.get(id.0))
    }

    /// Insert a new extension.
    pub fn insert_extension(&mut self, extension: Extension) -> LocalExtensionId {
        let extension_id = LocalExtensionId::new(self.next_extension_id);
        self.next_extension_id += 1;

        // index by target symbol for member lookup
        let target = extension.target;
        self.extensions_by_target
            .entry(target)
            .or_default()
            .push(extension_id);

        // index by extension symbol
        self.extension_by_symbol
            .insert(extension.symbol, extension_id);

        self.extensions.allocate(extension);
        extension_id
    }

    /// Get an extension by its id.
    pub fn get_extension(&self, extension_id: LocalExtensionId) -> &Extension {
        self.extensions.get(extension_id.0)
    }

    /// Get a mutable extension by its id.
    pub fn get_extension_mut(&mut self, extension_id: LocalExtensionId) -> &mut Extension {
        self.extensions.get_mut(extension_id.0)
    }

    /// Get all extensions targeting a specific type symbol.
    pub fn get_extensions_for_target(
        &self,
        target_symbol: GlobalSymbolId,
    ) -> Option<&Vec<LocalExtensionId>> {
        self.extensions_by_target.get(&target_symbol)
    }

    /// Iterate over all extensions.
    pub fn iter_extensions(&self) -> impl Iterator<Item = (LocalExtensionId, &Extension)> {
        (0..self.next_extension_id).map(|i| {
            let id = LocalExtensionId::new(i);
            (id, self.extensions.get(i))
        })
    }

    /// Iterate over all lineages with their associated symbol ids.
    pub fn iter_lineages(&self) -> impl Iterator<Item = (GlobalSymbolId, &Lineage)> {
        self.lineage_by_symbol_id
            .iter()
            .map(|(symbol_id, lineage_id)| (*symbol_id, self.lineages.get(lineage_id.0)))
    }
}
