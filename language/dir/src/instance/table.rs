use destack_source::Arena;
use indexmap::IndexMap;

use crate::{GlobalNodeIdAny, Instance, LocalInstanceId, ModuleId};

/// A InstanceTable is a side table for instancing. NOT THREAD-SAFE.
#[derive(Debug, Clone)]
pub struct InstanceTable {
    /// The module id of the instance table.
    pub module_id: ModuleId,

    /// The next instance id to allocate.
    pub(crate) next_instance_id: u32,
    /// The instances.
    pub(crate) instances: Arena<Instance>,

    /// The instance used by node ids.
    pub(crate) instance_by_node_id: IndexMap<GlobalNodeIdAny, LocalInstanceId>,
}

impl InstanceTable {
    /// Create a new InstanceTable.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            next_instance_id: 0,
            instances: Arena::new(),
            instance_by_node_id: IndexMap::new(),
        }
    }

    /// Insert a new instance.
    pub fn insert(&mut self, instance: Instance) -> LocalInstanceId {
        let instance_id = LocalInstanceId::new(self.next_instance_id);
        self.next_instance_id += 1;
        self.instances.allocate(instance);
        instance_id
    }

    /// Get an instance by its id.
    pub fn get(&self, instance_id: LocalInstanceId) -> &Instance {
        self.instances.get(instance_id.0)
    }

    /// Get a mutable instance by its id.
    pub fn get_mut(&mut self, instance_id: LocalInstanceId) -> &mut Instance {
        self.instances.get_mut(instance_id.0)
    }

    /// Set the instance used by a node id.
    pub fn instance(&mut self, node_id: GlobalNodeIdAny, instance_id: LocalInstanceId) {
        self.instance_by_node_id.insert(node_id, instance_id);
    }

    /// Get the instance used by a node id.
    pub fn get_instance(&self, node_id: GlobalNodeIdAny) -> Option<LocalInstanceId> {
        self.instance_by_node_id.get(&node_id).cloned()
    }
}
