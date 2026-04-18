use std::sync::Arc;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::WorkerId;
use crate::runtime::observe::ObservationSubscriptionId;
use crate::runtime::trace::TraceCursor;
use crate::runtime::world::{Revision, RevisionState, RuntimeId, World, WorldImage, WorldSnapshot};

use super::Control;
use super::handle::{ControlHandleId, ControlKind, WorldLabels};

/// One live world-handle entry.
#[derive(Debug)]
pub(crate) struct WorldEntry {
    /// The live world object.
    pub world: Box<World>,
    /// Stored world labels.
    pub labels: WorldLabels,
}

/// One live runtime-handle entry.
#[derive(Debug, Clone)]
pub(crate) struct RuntimeEntry {
    /// The owning world handle.
    pub world_handle_id: ControlHandleId,
    /// The runtime identifier inside that world.
    pub runtime_id: RuntimeId,
}

/// One live worker-handle entry.
#[derive(Debug, Clone)]
pub(crate) struct WorkerEntry {
    /// The owning world handle.
    pub world_handle_id: ControlHandleId,
    /// The owning runtime identifier inside that world.
    pub runtime_id: RuntimeId,
    /// The worker identifier inside that world.
    pub worker_id: WorkerId,
}

/// One live observation-handle entry.
#[derive(Debug, Clone)]
pub(crate) struct ObservationEntry {
    /// The owning world handle.
    pub world_handle_id: ControlHandleId,
    /// The observation subscription id inside that world.
    pub subscription_id: ObservationSubscriptionId,
}

/// One live trace-cursor entry.
#[derive(Debug, Clone)]
pub(crate) struct TraceCursorEntry {
    /// The owning world handle.
    pub world_handle_id: ControlHandleId,
    /// The trace cursor itself.
    pub cursor: Arc<TraceCursor>,
}

/// One pinned world-view entry.
#[derive(Debug, Clone)]
pub(crate) struct WorldViewEntry {
    /// The owning world handle.
    pub world_handle_id: ControlHandleId,
    /// The stored world labels.
    pub labels: WorldLabels,
    /// The pinned revision handle.
    pub revision_handle: Revision,
    /// The pinned revision metadata.
    pub revision: RevisionState,
    /// The pinned world image.
    pub image: Arc<WorldImage>,
}

/// One stored snapshot entry.
#[derive(Debug, Clone)]
pub(crate) struct SnapshotEntry {
    /// The owning world handle.
    pub world_handle_id: ControlHandleId,
    /// The requested snapshot format.
    pub format: super::ControlSnapshotFormat,
    /// The stored serialized snapshot.
    pub snapshot: WorldSnapshot,
    /// The encoded snapshot bytes.
    pub bytes: Arc<[u8]>,
}

/// One owner-thread control object stored under one external handle.
#[derive(Debug)]
pub(super) enum ControlObject {
    /// One live world handle.
    World(WorldEntry),
    /// One live runtime handle.
    Runtime(RuntimeEntry),
    /// One live worker handle.
    Worker(WorkerEntry),
    /// One live observation handle.
    Observation(ObservationEntry),
    /// One live trace-cursor handle.
    TraceCursor(TraceCursorEntry),
    /// One pinned world-view handle.
    WorldView(WorldViewEntry),
    /// One stored snapshot handle.
    Snapshot(Box<SnapshotEntry>),
}

impl ControlObject {
    /// Return one borrowed world entry when this is a world object.
    fn as_world(&self) -> Option<&WorldEntry> {
        match self {
            ControlObject::World(entry) => Some(entry),
            _ => None,
        }
    }

    /// Return one mutably borrowed world entry when this is a world object.
    fn as_world_mut(&mut self) -> Option<&mut WorldEntry> {
        match self {
            ControlObject::World(entry) => Some(entry),
            _ => None,
        }
    }

    /// Return one borrowed runtime entry when this is a runtime object.
    fn as_runtime(&self) -> Option<&RuntimeEntry> {
        match self {
            ControlObject::Runtime(entry) => Some(entry),
            _ => None,
        }
    }

    /// Return one borrowed worker entry when this is an worker object.
    fn as_worker(&self) -> Option<&WorkerEntry> {
        match self {
            ControlObject::Worker(entry) => Some(entry),
            _ => None,
        }
    }

    /// Return one borrowed observation entry when this is an observation object.
    fn as_observation(&self) -> Option<&ObservationEntry> {
        match self {
            ControlObject::Observation(entry) => Some(entry),
            _ => None,
        }
    }

    /// Return one borrowed trace-cursor entry when this is a trace cursor object.
    fn as_trace_cursor(&self) -> Option<&TraceCursorEntry> {
        match self {
            ControlObject::TraceCursor(entry) => Some(entry),
            _ => None,
        }
    }

    /// Return one borrowed world-view entry when this is a world-view object.
    fn as_world_view(&self) -> Option<&WorldViewEntry> {
        match self {
            ControlObject::WorldView(entry) => Some(entry),
            _ => None,
        }
    }

    /// Return one borrowed snapshot entry when this is a snapshot object.
    fn as_snapshot(&self) -> Option<&SnapshotEntry> {
        match self {
            ControlObject::Snapshot(entry) => Some(entry.as_ref()),
            _ => None,
        }
    }

    /// Return the owning world handle for one attached control object.
    fn world_handle_id(&self) -> Option<ControlHandleId> {
        match self {
            ControlObject::World(_) => None,
            ControlObject::Runtime(entry) => Some(entry.world_handle_id),
            ControlObject::Worker(entry) => Some(entry.world_handle_id),
            ControlObject::Observation(entry) => Some(entry.world_handle_id),
            ControlObject::TraceCursor(entry) => Some(entry.world_handle_id),
            ControlObject::WorldView(entry) => Some(entry.world_handle_id),
            ControlObject::Snapshot(entry) => Some(entry.world_handle_id),
        }
    }
}

impl Control {
    /// Insert one control object under one handle id.
    pub(super) fn insert_object(&mut self, handle_id: ControlHandleId, object: ControlObject) {
        self.objects.insert(handle_id, object);
    }

    /// Resolve one world entry.
    pub(super) fn get_world_entry(&self, handle_id: ControlHandleId) -> RuntimeResult<&WorldEntry> {
        let object = self.objects.get(&handle_id).ok_or_else(|| {
            RuntimeError::ControlHandleNotFound {
                handle_id: handle_id.get(),
                kind: ControlKind::World.name().to_string(),
            }
            .boxed()
        })?;

        object.as_world().ok_or_else(|| {
            RuntimeError::Internal {
                message: format!(
                    "control object kind mismatch for {} handle {}",
                    ControlKind::World.name(),
                    handle_id.get()
                ),
            }
            .boxed()
        })
    }

    /// Resolve one mutable world entry.
    pub(super) fn get_world_entry_mut(
        &mut self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<&mut WorldEntry> {
        let object = self.objects.get_mut(&handle_id).ok_or_else(|| {
            RuntimeError::ControlHandleNotFound {
                handle_id: handle_id.get(),
                kind: ControlKind::World.name().to_string(),
            }
            .boxed()
        })?;

        object.as_world_mut().ok_or_else(|| {
            RuntimeError::Internal {
                message: format!(
                    "control object kind mismatch for {} handle {}",
                    ControlKind::World.name(),
                    handle_id.get()
                ),
            }
            .boxed()
        })
    }

    /// Remove one world entry.
    pub(super) fn take_world_entry(
        &mut self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<WorldEntry> {
        match self.objects.remove(&handle_id) {
            Some(ControlObject::World(entry)) => Ok(entry),
            Some(_) => Err(RuntimeError::Internal {
                message: format!(
                    "control object kind mismatch for {} handle {}",
                    ControlKind::World.name(),
                    handle_id.get()
                ),
            }
            .boxed()),
            None => Err(RuntimeError::ControlHandleNotFound {
                handle_id: handle_id.get(),
                kind: ControlKind::World.name().to_string(),
            }
            .boxed()),
        }
    }

    /// Resolve one runtime entry.
    pub(super) fn get_runtime_entry(
        &self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<&RuntimeEntry> {
        let object = self.objects.get(&handle_id).ok_or_else(|| {
            RuntimeError::ControlHandleNotFound {
                handle_id: handle_id.get(),
                kind: ControlKind::Runtime.name().to_string(),
            }
            .boxed()
        })?;

        object.as_runtime().ok_or_else(|| {
            RuntimeError::Internal {
                message: format!(
                    "control object kind mismatch for {} handle {}",
                    ControlKind::Runtime.name(),
                    handle_id.get()
                ),
            }
            .boxed()
        })
    }

    /// Remove one runtime entry.
    pub(super) fn take_runtime_entry(
        &mut self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<RuntimeEntry> {
        match self.objects.remove(&handle_id) {
            Some(ControlObject::Runtime(entry)) => Ok(entry),
            Some(_) => Err(RuntimeError::Internal {
                message: format!(
                    "control object kind mismatch for {} handle {}",
                    ControlKind::Runtime.name(),
                    handle_id.get()
                ),
            }
            .boxed()),
            None => Err(RuntimeError::ControlHandleNotFound {
                handle_id: handle_id.get(),
                kind: ControlKind::Runtime.name().to_string(),
            }
            .boxed()),
        }
    }

    /// Resolve one worker entry.
    pub(super) fn get_worker_entry(
        &self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<&WorkerEntry> {
        let object = self.objects.get(&handle_id).ok_or_else(|| {
            RuntimeError::ControlHandleNotFound {
                handle_id: handle_id.get(),
                kind: ControlKind::Worker.name().to_string(),
            }
            .boxed()
        })?;

        object.as_worker().ok_or_else(|| {
            RuntimeError::Internal {
                message: format!(
                    "control object kind mismatch for {} handle {}",
                    ControlKind::Worker.name(),
                    handle_id.get()
                ),
            }
            .boxed()
        })
    }

    /// Remove one worker entry.
    pub(super) fn take_worker_entry(
        &mut self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<WorkerEntry> {
        match self.objects.remove(&handle_id) {
            Some(ControlObject::Worker(entry)) => Ok(entry),
            Some(_) => Err(RuntimeError::Internal {
                message: format!(
                    "control object kind mismatch for {} handle {}",
                    ControlKind::Worker.name(),
                    handle_id.get()
                ),
            }
            .boxed()),
            None => Err(RuntimeError::ControlHandleNotFound {
                handle_id: handle_id.get(),
                kind: ControlKind::Worker.name().to_string(),
            }
            .boxed()),
        }
    }

    /// Resolve one observation entry.
    pub(super) fn get_observation_entry(
        &self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<&ObservationEntry> {
        let object = self.objects.get(&handle_id).ok_or_else(|| {
            RuntimeError::ControlHandleNotFound {
                handle_id: handle_id.get(),
                kind: ControlKind::Observation.name().to_string(),
            }
            .boxed()
        })?;

        object.as_observation().ok_or_else(|| {
            RuntimeError::Internal {
                message: format!(
                    "control object kind mismatch for {} handle {}",
                    ControlKind::Observation.name(),
                    handle_id.get()
                ),
            }
            .boxed()
        })
    }

    /// Remove one observation entry.
    pub(super) fn take_observation_entry(
        &mut self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<ObservationEntry> {
        match self.objects.remove(&handle_id) {
            Some(ControlObject::Observation(entry)) => Ok(entry),
            Some(_) => Err(RuntimeError::Internal {
                message: format!(
                    "control object kind mismatch for {} handle {}",
                    ControlKind::Observation.name(),
                    handle_id.get()
                ),
            }
            .boxed()),
            None => Err(RuntimeError::ControlHandleNotFound {
                handle_id: handle_id.get(),
                kind: ControlKind::Observation.name().to_string(),
            }
            .boxed()),
        }
    }

    /// Resolve one trace-cursor entry.
    pub(super) fn get_trace_cursor_entry(
        &self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<&TraceCursorEntry> {
        let object = self.objects.get(&handle_id).ok_or_else(|| {
            RuntimeError::ControlHandleNotFound {
                handle_id: handle_id.get(),
                kind: ControlKind::TraceCursor.name().to_string(),
            }
            .boxed()
        })?;

        object.as_trace_cursor().ok_or_else(|| {
            RuntimeError::Internal {
                message: format!(
                    "control object kind mismatch for {} handle {}",
                    ControlKind::TraceCursor.name(),
                    handle_id.get()
                ),
            }
            .boxed()
        })
    }

    /// Remove one trace-cursor entry.
    pub(super) fn take_trace_cursor_entry(
        &mut self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<TraceCursorEntry> {
        match self.objects.remove(&handle_id) {
            Some(ControlObject::TraceCursor(entry)) => Ok(entry),
            Some(_) => Err(RuntimeError::Internal {
                message: format!(
                    "control object kind mismatch for {} handle {}",
                    ControlKind::TraceCursor.name(),
                    handle_id.get()
                ),
            }
            .boxed()),
            None => Err(RuntimeError::ControlHandleNotFound {
                handle_id: handle_id.get(),
                kind: ControlKind::TraceCursor.name().to_string(),
            }
            .boxed()),
        }
    }

    /// Resolve one world-view entry.
    pub(super) fn get_world_view_entry(
        &self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<&WorldViewEntry> {
        let object = self.objects.get(&handle_id).ok_or_else(|| {
            RuntimeError::ControlHandleNotFound {
                handle_id: handle_id.get(),
                kind: ControlKind::WorldView.name().to_string(),
            }
            .boxed()
        })?;

        object.as_world_view().ok_or_else(|| {
            RuntimeError::Internal {
                message: format!(
                    "control object kind mismatch for {} handle {}",
                    ControlKind::WorldView.name(),
                    handle_id.get()
                ),
            }
            .boxed()
        })
    }

    /// Remove one world-view entry.
    pub(super) fn take_world_view_entry(
        &mut self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<WorldViewEntry> {
        match self.objects.remove(&handle_id) {
            Some(ControlObject::WorldView(entry)) => Ok(entry),
            Some(_) => Err(RuntimeError::Internal {
                message: format!(
                    "control object kind mismatch for {} handle {}",
                    ControlKind::WorldView.name(),
                    handle_id.get()
                ),
            }
            .boxed()),
            None => Err(RuntimeError::ControlHandleNotFound {
                handle_id: handle_id.get(),
                kind: ControlKind::WorldView.name().to_string(),
            }
            .boxed()),
        }
    }

    /// Resolve one snapshot entry.
    pub(super) fn get_snapshot_entry(
        &self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<&SnapshotEntry> {
        let object = self.objects.get(&handle_id).ok_or_else(|| {
            RuntimeError::ControlHandleNotFound {
                handle_id: handle_id.get(),
                kind: ControlKind::Snapshot.name().to_string(),
            }
            .boxed()
        })?;

        object.as_snapshot().ok_or_else(|| {
            RuntimeError::Internal {
                message: format!(
                    "control object kind mismatch for {} handle {}",
                    ControlKind::Snapshot.name(),
                    handle_id.get()
                ),
            }
            .boxed()
        })
    }

    /// Return attached handles for one world.
    pub(super) fn handles_for_world(
        &self,
        world_handle_id: ControlHandleId,
    ) -> Vec<ControlHandleId> {
        self.objects
            .iter()
            .filter_map(|(handle_id, object)| {
                object
                    .world_handle_id()
                    .filter(|owner_handle_id| *owner_handle_id == world_handle_id)
                    .map(|_| *handle_id)
            })
            .collect()
    }

    /// Remove one attached control object without caring about its specific kind.
    pub(super) fn remove_object(&mut self, handle_id: ControlHandleId) {
        self.objects.remove(&handle_id);
    }
}
