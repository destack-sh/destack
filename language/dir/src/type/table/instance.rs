use destack_source::AdaptImage;
use std::collections::HashMap;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{Arena, GlobalNodeIdAny, GlobalSymbolId, Instance, LocalInstanceId, Resolution};

use super::core::{InstanceInternerKey, instance_interner_key};
use super::{TypeTable, resolution};

/// Instance interning and attachment ownership.
#[derive(Debug, Clone, Serialize, Deserialize, AdaptImage)]
pub struct InstanceTable {
    /// The next instance id to allocate.
    pub(crate) next_instance_id: u32,
    /// The instances.
    pub(crate) instances: crate::Arena<Instance>,
    /// The instance used by node ids.
    pub(crate) instance_by_node_id: IndexMap<GlobalNodeIdAny, LocalInstanceId>,
    /// Candidate instance ids indexed by compact interner key.
    pub(super) instance_ids_by_interner_key: HashMap<InstanceInternerKey, Vec<LocalInstanceId>>,
}

impl InstanceTable {
    /// Create an empty instance table.
    pub fn new() -> Self {
        Self {
            next_instance_id: 0,
            instances: crate::Arena::new(),
            instance_by_node_id: IndexMap::new(),
            instance_ids_by_interner_key: HashMap::new(),
        }
    }
}

impl Default for InstanceTable {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeTable {
    /// Insert a new instance.
    pub fn insert_instance(&mut self, instance: Instance) -> LocalInstanceId {
        let interner_key =
            instance_interner_key(instance.symbol_id, instance.generic_arguments.len());

        // reuse existing exact instances within the compact interner bucket
        if let Some(candidates) = self
            .instance
            .instance_ids_by_interner_key
            .get(&interner_key)
        {
            for candidate_id in candidates {
                if self.get_instance(*candidate_id) == &instance {
                    return *candidate_id;
                }
            }
        }

        let instance_id = LocalInstanceId::new(self.instance.next_instance_id);
        self.instance.next_instance_id += 1;
        self.instance.instances.allocate(instance);
        self.instance
            .instance_ids_by_interner_key
            .entry(interner_key)
            .or_default()
            .push(instance_id);
        instance_id
    }

    /// Get an instance by its id.
    pub fn get_instance(&self, instance_id: LocalInstanceId) -> &Instance {
        self.instance.instances.get(instance_id.0)
    }

    /// Return the number of committed instances.
    pub fn instance_count(&self) -> usize {
        self.instance.instances.iter().count()
    }

    /// Iterate committed instances with their local ids.
    pub fn iter_instances(&self) -> impl Iterator<Item = (LocalInstanceId, &Instance)> + '_ {
        self.instance
            .instances
            .iter()
            .enumerate()
            .map(|(index, instance)| (LocalInstanceId::new(index as u32), instance))
    }

    /// Get a mutable instance by its id.
    pub fn get_instance_mut(&mut self, instance_id: LocalInstanceId) -> &mut Instance {
        self.instance.instances.get_mut(instance_id.0)
    }

    /// Set the instance used by a node id.
    pub fn set_instance_for_node(
        &mut self,
        node_id: GlobalNodeIdAny,
        instance_id: LocalInstanceId,
    ) {
        self.instance
            .instance_by_node_id
            .insert(node_id, instance_id);
    }

    /// Get the instance used by a node id.
    pub fn get_instance_for_node(&self, node_id: GlobalNodeIdAny) -> Option<LocalInstanceId> {
        self.instance.instance_by_node_id.get(&node_id).copied()
    }

    /// Return candidate instance ids for one compact interner key.
    pub fn query_instance_interner_candidates(
        &self,
        symbol_id: GlobalSymbolId,
        generic_argument_count: usize,
    ) -> Vec<LocalInstanceId> {
        let key = instance_interner_key(symbol_id, generic_argument_count);
        self.instance
            .instance_ids_by_interner_key
            .get(&key)
            .cloned()
            .unwrap_or_default()
    }

    /// Find one exact instance by full instance shape.
    pub fn find_instance_exact(&self, expected: &Instance) -> Option<LocalInstanceId> {
        for (index, instance) in self.instance.instances.iter().enumerate() {
            if instance == expected {
                return Some(LocalInstanceId::new(index as u32));
            }
        }

        None
    }

    /// Replace all instances and remap node and resolution instance references.
    pub fn replace_instances_with_remap(
        &mut self,
        instances: Vec<Instance>,
        remap: &HashMap<LocalInstanceId, LocalInstanceId>,
    ) {
        // remap instance facts attached to nodes
        for instance_id in self.instance.instance_by_node_id.values_mut() {
            if let Some(remapped) = remap.get(instance_id) {
                *instance_id = *remapped;
            }
        }

        // remap instance facts attached to resolution candidates
        for resolution_id in 0..self.resolution.resolutions.len() {
            let resolution = self.resolution.resolutions.get_mut(resolution_id as u32);
            match resolution {
                Resolution::Static { candidate, .. } => {
                    resolution::remap_resolution_candidate_instance(candidate, remap);
                }
                Resolution::Dynamic { candidates, .. }
                | Resolution::Unresolved { candidates, .. } => {
                    for candidate in candidates {
                        resolution::remap_resolution_candidate_instance(candidate, remap);
                    }
                }
                Resolution::Builtin { .. } => {}
            }
        }

        // rebuild the instance arena in canonical id order
        let mut rebuilt = Arena::with(instances.len());
        for instance in instances {
            rebuilt.allocate(instance);
        }

        self.instance.instances = rebuilt;
        self.instance.next_instance_id = self.instance.instances.len() as u32;
        self.instance.instance_ids_by_interner_key.clear();

        for (index, instance) in self.instance.instances.iter().enumerate() {
            let instance_id = LocalInstanceId::new(index as u32);
            let interner_key =
                instance_interner_key(instance.symbol_id, instance.generic_arguments.len());
            self.instance
                .instance_ids_by_interner_key
                .entry(interner_key)
                .or_default()
                .push(instance_id);
        }
    }
}
