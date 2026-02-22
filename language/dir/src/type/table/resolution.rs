use std::collections::HashMap;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{GlobalNodeIdAny, LocalInstanceId, LocalResolutionId, Resolution, ResolutionCandidate};

use super::TypeTable;

/// Resolution ownership for members and overloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionTable {
    /// The next resolution id to allocate.
    pub(crate) next_resolution_id: u32,
    /// The resolutions.
    pub(crate) resolutions: crate::Arena<Resolution>,
    /// The resolution used by node ids.
    pub(crate) resolution_by_node_id: IndexMap<GlobalNodeIdAny, LocalResolutionId>,
}

impl ResolutionTable {
    /// Create an empty resolution table.
    pub fn new() -> Self {
        Self {
            next_resolution_id: 0,
            resolutions: crate::Arena::new(),
            resolution_by_node_id: IndexMap::new(),
        }
    }
}

/// Remap one resolution candidate instance id when present.
pub(super) fn remap_resolution_candidate_instance(
    candidate: &mut ResolutionCandidate,
    remap: &HashMap<LocalInstanceId, LocalInstanceId>,
) {
    let Some(instance_id) = candidate.instance else {
        return;
    };

    if let Some(remapped) = remap.get(&instance_id) {
        candidate.instance = Some(*remapped);
    }
}

impl TypeTable {
    /// Insert a new resolution.
    pub fn insert_resolution(&mut self, resolution: Resolution) -> LocalResolutionId {
        let resolution_id = LocalResolutionId::new(self.resolution.next_resolution_id);
        self.resolution.next_resolution_id += 1;
        self.resolution.resolutions.allocate(resolution);
        resolution_id
    }

    /// Get a resolution by its id.
    pub fn get_resolution(&self, resolution_id: LocalResolutionId) -> &Resolution {
        self.resolution.resolutions.get(resolution_id.0)
    }

    /// Get a mutable resolution by its id.
    pub fn get_resolution_mut(&mut self, resolution_id: LocalResolutionId) -> &mut Resolution {
        self.resolution.resolutions.get_mut(resolution_id.0)
    }

    /// Set the resolution used by a node id.
    pub fn set_resolution_for_node(
        &mut self,
        node_id: GlobalNodeIdAny,
        resolution_id: LocalResolutionId,
    ) {
        self.resolution
            .resolution_by_node_id
            .insert(node_id, resolution_id);
    }

    /// Get the resolution used by a node id.
    pub fn get_resolution_for_node(&self, node_id: GlobalNodeIdAny) -> Option<LocalResolutionId> {
        self.resolution.resolution_by_node_id.get(&node_id).copied()
    }
}
