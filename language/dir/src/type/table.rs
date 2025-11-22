use indexmap::IndexMap;

use crate::{
    Arena, GlobalNodeIdAny, LocalNodeId, LocalNodeIdAny, LocalTypeId, ModuleId, Node, Type,
};

/// A TypeTable is a side table for a node. NOT THREAD-SAFE.
#[derive(Debug, Clone)]
pub struct TypeTable {
    /// The module id of the type table.
    pub module_id: ModuleId,

    /// The next type id to allocate.
    pub(crate) next_type_id: u32,
    /// The types.
    pub(crate) types: Arena<Type>,

    /// The source ids of all types. Index is the type id.
    pub(crate) source_id_by_type_id: Vec<LocalNodeIdAny>,
    /// The declared type by node id.
    pub(crate) declared_type_by_node_id: IndexMap<GlobalNodeIdAny, LocalTypeId>,
    /// The inferred type by node id.
    pub(crate) inferred_type_by_node_id: IndexMap<GlobalNodeIdAny, LocalTypeId>,
}

impl TypeTable {
    /// Create a new TypeTable.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            next_type_id: 0,
            types: Arena::new(),
            source_id_by_type_id: Vec::new(),
            declared_type_by_node_id: IndexMap::new(),
            inferred_type_by_node_id: IndexMap::new(),
        }
    }

    /// Insert a type derived from some source node.
    pub fn insert_from<T: Node>(&mut self, ty: Type, node_id: LocalNodeId<T>) -> LocalTypeId {
        let type_id = LocalTypeId::new(self.next_type_id);
        self.next_type_id += 1;
        self.types.allocate(ty);
        self.source_id_by_type_id.push(node_id.into_any());
        type_id
    }

    /// Insert a type derived from some source node.
    pub fn insert_from_any<T: Node>(&mut self, ty: Type, node_id: LocalNodeIdAny) -> LocalTypeId {
        let type_id = LocalTypeId::new(self.next_type_id);
        self.next_type_id += 1;
        self.types.allocate(ty);
        self.source_id_by_type_id.push(node_id);
        type_id
    }

    /// Get a type by its id.
    pub fn get(&self, type_id: LocalTypeId) -> &Type {
        self.types.get(type_id.0)
    }

    /// Get a mutable type by its id.
    pub fn get_mut(&mut self, type_id: LocalTypeId) -> &mut Type {
        self.types.get_mut(type_id.0)
    }

    /// Get the source id for a type.
    pub fn get_source(&self, type_id: LocalTypeId) -> LocalNodeIdAny {
        self.source_id_by_type_id[type_id.0 as usize]
    }

    /// Set the declared type for a node.
    pub fn set_declared_type(&mut self, node_id: GlobalNodeIdAny, ty: LocalTypeId) {
        self.declared_type_by_node_id.insert(node_id, ty);
    }

    /// Set the inferred type for a node.
    pub fn set_inferred_type(&mut self, node_id: GlobalNodeIdAny, ty: LocalTypeId) {
        self.inferred_type_by_node_id.insert(node_id, ty);
    }

    /// Get the declared type for a node.
    pub fn get_declared_type(&self, node_id: GlobalNodeIdAny) -> Option<&Type> {
        self.declared_type_by_node_id
            .get(&node_id)
            .map(|ty| self.types.get(ty.0))
    }

    /// Get the inferred type for a node.
    pub fn get_inferred_type(&self, node_id: GlobalNodeIdAny) -> Option<&Type> {
        self.inferred_type_by_node_id
            .get(&node_id)
            .map(|ty| self.types.get(ty.0))
    }
}
