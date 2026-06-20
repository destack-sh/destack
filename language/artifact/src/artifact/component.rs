use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use destack_source::{ComponentId, ModuleId, ProfileId};

use crate::{ArtifactProjectionFingerprint, ComponentGraphProjection};

/// Strongly connected component partition of one profile's module graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentGraph {
    /// The profile this partition belongs to.
    pub profile: ProfileId,
    /// Imported modules per module, deduplicated in import order.
    pub imports: IndexMap<ModuleId, Arc<[ModuleId]>>,
    /// Owning component per module.
    pub component_of: IndexMap<ModuleId, ComponentId>,
    /// Member modules per component, sorted, with the entry first.
    pub members: IndexMap<ComponentId, Vec<ModuleId>>,
    /// External components each component depends on.
    pub dependencies: IndexMap<ComponentId, Vec<ComponentId>>,
}

impl ComponentGraph {
    /// Return the modules imported by one module.
    pub fn imports(&self, module: ModuleId) -> &[ModuleId] {
        self.imports.get(&module).map_or(&[], Arc::as_ref)
    }

    /// Return the component containing one module.
    pub fn component(&self, module: ModuleId) -> Option<ComponentId> {
        self.component_of.get(&module).copied()
    }

    /// Return the component and entry module containing one module.
    pub fn component_entry(&self, module: ModuleId) -> Option<(ComponentId, ModuleId)> {
        let component = self.component(module)?;
        let entry = self.entry(component)?;

        Some((component, entry))
    }

    /// Return the member modules of one component.
    pub fn members(&self, component: ComponentId) -> &[ModuleId] {
        self.members.get(&component).map_or(&[], Vec::as_slice)
    }

    /// Return the entry module of one component.
    pub fn entry(&self, component: ComponentId) -> Option<ModuleId> {
        self.members(component).first().copied()
    }

    /// Return the external components one component depends on.
    pub fn dependencies(&self, component: ComponentId) -> &[ComponentId] {
        self.dependencies.get(&component).map_or(&[], Vec::as_slice)
    }

    /// Return the stable fingerprint of one projected component graph value.
    pub fn projection_fingerprint(
        &self,
        projection: ComponentGraphProjection,
    ) -> ArtifactProjectionFingerprint {
        match projection {
            ComponentGraphProjection::ComponentOf(module) => {
                ArtifactProjectionFingerprint::new(&self.component(module))
            }
            ComponentGraphProjection::ComponentEntryOf(module) => {
                ArtifactProjectionFingerprint::new(&self.component_entry(module))
            }
            ComponentGraphProjection::Members(component) => {
                ArtifactProjectionFingerprint::new(&self.members(component))
            }
            ComponentGraphProjection::Dependencies(component) => {
                ArtifactProjectionFingerprint::new(&self.dependencies(component))
            }
        }
    }
}
