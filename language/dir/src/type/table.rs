use indexmap::IndexMap;

use crate::{
    Arena, GlobalNodeIdAny, GlobalSymbolId, Instance, LocalInstanceId, LocalNodeId, LocalNodeIdAny,
    LocalResolutionId, LocalTypeId, ModuleId, Node, Resolution, Type,
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
    /// The declared type by node id (type annotations live on nodes).
    pub(crate) declared_type_by_node_id: IndexMap<GlobalNodeIdAny, LocalTypeId>,
    /// The inferred type by node id (expression types at specific locations).
    pub(crate) inferred_type_by_node_id: IndexMap<GlobalNodeIdAny, LocalTypeId>,
    /// The inferred type by symbol id (convenient lookup for "what type is this binding?").
    pub(crate) inferred_type_by_symbol_id: IndexMap<GlobalSymbolId, LocalTypeId>,

    // instances
    /// The next instance id to allocate.
    pub(crate) next_instance_id: u32,
    /// The instances.
    pub(crate) instances: Arena<Instance>,
    /// The instance used by node ids.
    pub(crate) instance_by_node_id: IndexMap<GlobalNodeIdAny, LocalInstanceId>,

    // resolutions
    /// The next resolution id to allocate.
    pub(crate) next_resolution_id: u32,
    /// The resolutions.
    pub(crate) resolutions: Arena<Resolution>,
    /// The resolution used by node ids.
    pub(crate) resolution_by_node_id: IndexMap<GlobalNodeIdAny, LocalResolutionId>,
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
            declared_type_by_node_id: IndexMap::new(),
            inferred_type_by_node_id: IndexMap::new(),
            inferred_type_by_symbol_id: IndexMap::new(),
            // instances
            next_instance_id: 0,
            instances: Arena::new(),
            instance_by_node_id: IndexMap::new(),
            // resolutions
            next_resolution_id: 0,
            resolutions: Arena::new(),
            resolution_by_node_id: IndexMap::new(),
        }
    }

    /// Insert a type derived from some source node.
    pub fn insert_type<T: Node>(&mut self, ty: Type, node_id: LocalNodeId<T>) -> LocalTypeId {
        let type_id = LocalTypeId::new(self.next_type_id);
        self.next_type_id += 1;
        self.types.allocate(ty);
        self.source_id_by_type_id.push(node_id.into_any());
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
    pub fn set_inferred_type_for_node(&mut self, node_id: GlobalNodeIdAny, ty: LocalTypeId) {
        self.inferred_type_by_node_id.insert(node_id, ty);
    }

    /// Get the inferred type for a node.
    pub fn get_inferred_type_for_node(&self, node_id: GlobalNodeIdAny) -> Option<&Type> {
        self.inferred_type_by_node_id
            .get(&node_id)
            .map(|ty| self.types.get(ty.0))
    }

    /// Get the inferred type id for a node.
    pub fn get_inferred_type_id_for_node(&self, node_id: GlobalNodeIdAny) -> Option<LocalTypeId> {
        self.inferred_type_by_node_id.get(&node_id).copied()
    }

    /// Set the inferred type for a symbol (convenient binding type lookup).
    pub fn set_inferred_type_for_symbol(&mut self, symbol_id: GlobalSymbolId, ty: LocalTypeId) {
        self.inferred_type_by_symbol_id.insert(symbol_id, ty);
    }

    /// Get the inferred type for a symbol.
    pub fn get_inferred_type_for_symbol(&self, symbol_id: GlobalSymbolId) -> Option<&Type> {
        self.inferred_type_by_symbol_id
            .get(&symbol_id)
            .map(|ty| self.types.get(ty.0))
    }

    /// Get the inferred type id for a symbol.
    pub fn get_inferred_type_id_for_symbol(
        &self,
        symbol_id: GlobalSymbolId,
    ) -> Option<LocalTypeId> {
        self.inferred_type_by_symbol_id.get(&symbol_id).copied()
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
}
