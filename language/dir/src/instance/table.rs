use destack_source::Arena;

use crate::{Instance, LocalInstanceId, ModuleId};

/// A InstanceTable is a side table for instancing. NOT THREAD-SAFE.
#[derive(Debug, Clone)]
pub struct InstanceTable {
    /// The module id of the instance table.
    pub module_id: ModuleId,

    /// The next instance id to allocate.
    pub(crate) next_instance_id: u32,
    /// The instances.
    pub(crate) instances: Arena<Instance>,
}

impl InstanceTable {
    /// Create a new InstanceTable.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            next_instance_id: 0,
            instances: Arena::new(),
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
}
