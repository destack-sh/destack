use destack_source::Arena;
use indexmap::IndexMap;

use crate::{GlobalNodeIdAny, Instance, LocalInstanceId, LocalResolutionId, ModuleId, Resolution};

/// A InstanceTable is a side table for instancing and resolution. NOT THREAD-SAFE.
#[derive(Debug, Clone)]
pub struct InstanceTable {
    /// The module id of the instance table.
    pub module_id: ModuleId,

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

impl InstanceTable {
    /// Create a new InstanceTable.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
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
