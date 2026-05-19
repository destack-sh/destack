use std::collections::BTreeMap;
use std::sync::Arc;

use destack_core::{CaptureMode, fnv1a_128};
use postcard::to_allocvec;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::ResourceId;
use crate::host::resource::ResourceRebinders;
use crate::runtime::random::RandomImage;
use crate::runtime::time::ClockImage;
use crate::runtime::{Runtime, RuntimeImage, WorkerId, WorkerImage};
use crate::simulation::Simulation;
use crate::world::policy::Policy;
use crate::world::scenario::Scenario;
use crate::world::topology::{Edge, Entity, RuntimeId, Topology};

use super::{Resource, World};

/// World image payload for one materialized world restore point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldImage {
    /// The next runtime id to allocate after restore.
    pub(crate) next_runtime_id: u64,
    /// The next worker id to allocate after restore.
    pub(crate) next_worker_id: u64,
    /// The next scenario id to allocate after restore.
    pub(crate) next_scenario_id: u64,
    /// Captured dynamic policy state.
    pub(crate) policy: Policy,
    /// Captured dynamic scenario state.
    pub(crate) scenarios: Vec<Scenario>,
    /// Captured topology metadata graph.
    pub(crate) topology: Topology,
    /// Captured world resources.
    pub(crate) resources: BTreeMap<ResourceId, Resource>,
    /// Captured simulation state.
    pub(crate) simulation: Simulation,
    /// Captured world clock state.
    pub(crate) clock: ClockImage,
    /// Captured world random state.
    pub(crate) random: RandomImage,
    /// Captured runtime metadata keyed by runtime id.
    pub(crate) runtimes: BTreeMap<RuntimeId, Arc<RuntimeImage>>,
    /// Captured worker metadata keyed by worker id.
    pub(crate) workers: BTreeMap<WorkerId, Arc<WorkerImage>>,
}

impl WorldImage {
    /// Return the captured world policy specification.
    pub fn policy(&self) -> &Policy {
        &self.policy
    }

    /// Return the captured runtimes keyed by runtime id.
    pub fn runtimes(&self) -> &BTreeMap<RuntimeId, Arc<RuntimeImage>> {
        &self.runtimes
    }

    /// Return the captured workers keyed by worker id.
    pub fn workers(&self) -> &BTreeMap<WorkerId, Arc<WorkerImage>> {
        &self.workers
    }

    /// Return the captured world resources keyed by resource id.
    pub fn resources(&self) -> &BTreeMap<ResourceId, Resource> {
        &self.resources
    }

    /// Return the number of captured runtimes.
    pub fn runtime_count(&self) -> usize {
        self.runtimes.len()
    }

    /// Return the number of captured workers.
    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    /// Return the number of captured logical resources.
    pub fn resource_count(&self) -> usize {
        self.resources.len()
    }

    /// Return the number of captured topology entities.
    pub fn entity_count(&self) -> usize {
        self.topology.entities().len()
    }

    /// Return the number of captured topology edges.
    pub fn edge_count(&self) -> usize {
        self.topology.edges().len()
    }

    /// Report whether one runtime exists in this image.
    pub fn has_runtime(&self, runtime_id: RuntimeId) -> bool {
        self.runtimes.contains_key(&runtime_id)
    }

    /// Report whether one worker exists in this image.
    pub fn has_worker(&self, worker_id: WorkerId) -> bool {
        self.workers.contains_key(&worker_id)
    }

    /// Report whether one resource exists in this image.
    pub fn has_resource(&self, resource_id: ResourceId) -> bool {
        self.resources.contains_key(&resource_id)
    }

    /// Report whether one topology entity exists in this image.
    pub fn has_entity(&self, entity_id: &str) -> bool {
        self.topology.entities().contains_key(entity_id)
    }

    /// Report whether one topology edge exists in this image.
    pub fn has_edge(&self, edge_id: &str) -> bool {
        self.topology.edges().contains_key(edge_id)
    }

    /// Return one runtime image by id.
    pub fn runtime(&self, runtime_id: RuntimeId) -> RuntimeResult<&RuntimeImage> {
        self.runtimes
            .get(&runtime_id)
            .map(Arc::as_ref)
            .ok_or_else(|| {
                RuntimeError::RuntimeNotFound {
                    runtime_id: runtime_id.0,
                }
                .boxed()
            })
    }

    /// Return labels for one runtime image.
    pub fn runtime_labels(
        &self,
        runtime_id: RuntimeId,
    ) -> RuntimeResult<&BTreeMap<String, String>> {
        let entity_id = runtime_id.entity_id();
        let entity = self.topology.entities().get(entity_id.as_str()).ok_or(
            RuntimeError::RuntimeNotFound {
                runtime_id: runtime_id.0,
            },
        )?;

        Ok(&entity.labels)
    }

    /// Return the topology name for one runtime image.
    pub fn runtime_name(&self, runtime_id: RuntimeId) -> RuntimeResult<&str> {
        let entity = self
            .topology
            .entities()
            .get(&runtime_id.entity_id())
            .ok_or(RuntimeError::RuntimeNotFound {
                runtime_id: runtime_id.0,
            })?;

        Ok(entity.name.as_str())
    }

    /// Return one worker image by id.
    pub fn worker(&self, worker_id: WorkerId) -> RuntimeResult<&WorkerImage> {
        self.workers
            .get(&worker_id)
            .map(Arc::as_ref)
            .ok_or_else(|| {
                RuntimeError::WorkerNotFound {
                    worker_id: worker_id.0,
                }
                .boxed()
            })
    }

    /// Return labels for one worker image.
    pub fn worker_labels(&self, worker_id: WorkerId) -> RuntimeResult<&BTreeMap<String, String>> {
        let entity_id = worker_id.entity_id();
        let entity = self.topology.entities().get(entity_id.as_str()).ok_or(
            RuntimeError::WorkerNotFound {
                worker_id: worker_id.0,
            },
        )?;

        Ok(&entity.labels)
    }

    /// Return the topology name for one worker image.
    pub fn worker_name(&self, worker_id: WorkerId) -> RuntimeResult<&str> {
        let entity = self.topology.entities().get(&worker_id.entity_id()).ok_or(
            RuntimeError::WorkerNotFound {
                worker_id: worker_id.0,
            },
        )?;

        Ok(entity.name.as_str())
    }

    /// Return whether one captured runtime owns one captured worker.
    pub fn runtime_owns_worker(&self, runtime_id: RuntimeId, worker_id: WorkerId) -> bool {
        self.topology.runtime_owns_worker(runtime_id, worker_id)
    }

    /// Return the owning runtime for one captured worker.
    pub fn worker_runtime_id(&self, worker_id: WorkerId) -> RuntimeResult<RuntimeId> {
        self.runtimes
            .keys()
            .copied()
            .find(|runtime_id| self.runtime_owns_worker(*runtime_id, worker_id))
            .ok_or_else(|| {
                RuntimeError::WorkerNotFound {
                    worker_id: worker_id.0,
                }
                .boxed()
            })
    }

    /// Return one logical world resource by id.
    pub fn resource(&self, resource_id: ResourceId) -> Option<&Resource> {
        self.resources.get(&resource_id)
    }

    /// Return one topology entity by id.
    pub fn entity(&self, entity_id: &str) -> Option<&Entity> {
        self.topology.entities().get(entity_id)
    }

    /// Return one topology edge by id.
    pub fn edge(&self, edge_id: &str) -> Option<&Edge> {
        self.topology.edges().get(edge_id)
    }
}

impl World {
    /// Capture one materialized world image while the world is under exclusive access.
    pub(crate) fn capture_image(&mut self, mode: CaptureMode) -> RuntimeResult<WorldImage> {
        self.quiesce_shared_gc();

        let image = (|| {
            let mut runtime_images = BTreeMap::new();
            let mut worker_images = BTreeMap::new();

            for runtime in self.runtimes.values_mut() {
                let (runtime_image, runtime_workers) = runtime.capture_image(mode)?;
                runtime_images.insert(runtime.runtime_id(), runtime_image);
                worker_images.extend(runtime_workers);
            }

            Ok(WorldImage {
                next_runtime_id: self.state.next_runtime_id,
                next_worker_id: self.state.next_worker_id,
                next_scenario_id: self.state.next_scenario_id,
                policy: self.state.policy.clone(),
                scenarios: self.state.scenarios.clone(),
                topology: self.state.topology.clone(),
                resources: self.state.resources.clone(),
                simulation: self.state.simulation.clone(),
                clock: self.state.clock.snapshot(),
                random: self.state.random.snapshot(),
                runtimes: runtime_images,
                workers: worker_images,
            })
        })();

        self.resume_shared_gc();

        image
    }

    /// Restore one materialized image into this world while the world is quiesced.
    pub(crate) fn restore_image(
        &mut self,
        image: &WorldImage,
        rebind_context: Option<&ResourceRebinders>,
    ) -> RuntimeResult<()> {
        self.quiesce_shared_gc();

        let result = (|| {
            self.state.next_runtime_id = image.next_runtime_id;
            self.state.next_worker_id = image.next_worker_id;
            self.state.next_scenario_id = image.next_scenario_id;
            self.state.policy = image.policy.clone();
            self.state.scenarios = image.scenarios.clone();
            self.state.topology = image.topology.clone();
            self.state.resources = image.resources.clone();
            self.state.simulation = image.simulation.clone();

            self.state.clock.restore_snapshot(&image.clock);
            self.state.random.restore_snapshot(&image.random)?;
            self.state.observations.reset();

            let mut restored_runtimes = BTreeMap::new();
            for (runtime_id, runtime_image) in &image.runtimes {
                let runtime_worker_images = image
                    .workers
                    .iter()
                    .filter(|(worker_id, _)| image.runtime_owns_worker(*runtime_id, **worker_id))
                    .map(|(worker_id, worker_image)| (*worker_id, worker_image.clone()))
                    .collect::<BTreeMap<_, _>>();
                let (allocator, collector) = {
                    let history = self.history.read();
                    (history.allocator(), history.collector())
                };

                let runtime = Runtime::from_image(
                    &mut self.state,
                    allocator,
                    collector,
                    *runtime_id,
                    runtime_image.as_ref(),
                    &runtime_worker_images,
                    rebind_context,
                )?;
                restored_runtimes.insert(*runtime_id, Box::new(runtime));
            }

            self.runtimes = restored_runtimes;

            Ok(())
        })();

        self.resume_shared_gc();

        result
    }

    /// Return the encoded size and hash for one image.
    pub(crate) fn image_size_and_hash(image: &WorldImage) -> RuntimeResult<(u64, u128)> {
        let bytes = to_allocvec(image).map_err(|_| {
            RuntimeError::InconsistentImage {
                detail: "failed to encode world image".to_string(),
            }
            .boxed()
        })?;
        let size_bytes = bytes.len() as u64;
        let hash = fnv1a_128(&bytes);

        Ok((size_bytes, hash))
    }
}
