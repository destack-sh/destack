use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, OnceLock};
use std::thread::{self, ThreadId};

use parking_lot::RwLock;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::AgentId;
use crate::runtime::replay::TraceCursor;
use crate::runtime::world::{
    Image, ObservationSubscriptionId, Revision, RevisionId, RuntimeId, Snapshot as WorldSnapshot,
    World,
};

/// Opaque control-handle identifier in the runtime control table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct ControlHandleId(u64);

impl ControlHandleId {
    /// Create one control-handle identifier.
    pub(crate) const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw control-handle value.
    pub(crate) const fn get(self) -> u64 {
        self.0
    }
}

/// Snapshot format stored in one control-table snapshot entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ControlSnapshotFormat {
    /// Fast runtime-specific snapshot encoding.
    Fast,
    /// Portable snapshot encoding.
    Portable,
}

/// Stable labels associated with one externally controlled world handle.
#[derive(Debug, Clone, Default)]
pub(crate) struct WorldLabels {
    /// Stored world labels in stable key order.
    pub labels: BTreeMap<String, String>,
}

/// One live world-handle entry.
#[derive(Debug, Clone)]
pub(crate) struct WorldEntry {
    /// The live world object.
    pub world: Arc<World>,
    /// Stored world labels.
    pub labels: WorldLabels,
}

/// One live runtime-handle entry.
#[derive(Debug, Clone)]
pub(crate) struct RuntimeEntry {
    /// The owning world handle.
    pub world_handle_id: ControlHandleId,
    /// The live world object.
    pub world: Arc<World>,
    /// The runtime identifier inside that world.
    pub runtime_id: RuntimeId,
}

/// One live agent-handle entry.
#[derive(Debug, Clone)]
pub(crate) struct AgentEntry {
    /// The owning world handle.
    pub world_handle_id: ControlHandleId,
    /// The live world object.
    pub world: Arc<World>,
    /// The owning runtime identifier inside that world.
    pub runtime_id: RuntimeId,
    /// The agent identifier inside that world.
    pub agent_id: AgentId,
}

/// One live observation-handle entry.
#[derive(Debug, Clone)]
pub(crate) struct ObservationEntry {
    /// The owning world handle.
    pub world_handle_id: ControlHandleId,
    /// The live world object.
    pub world: Arc<World>,
    /// The observation subscription id inside that world.
    pub subscription_id: ObservationSubscriptionId,
}

/// One live trace-cursor entry.
#[derive(Debug, Clone)]
pub(crate) struct TraceCursorEntry {
    /// The owning world handle.
    pub world_handle_id: ControlHandleId,
    /// The live world object.
    pub world: Arc<World>,
    /// The trace cursor itself.
    pub cursor: Arc<TraceCursor>,
}

/// One pinned world-view entry.
#[derive(Debug, Clone)]
pub(crate) struct WorldViewEntry {
    /// The owning world handle.
    pub world_handle_id: ControlHandleId,
    /// The live world object.
    pub world: Arc<World>,
    /// The stored world labels.
    pub labels: WorldLabels,
    /// The pinned revision metadata.
    pub revision: Revision,
    /// The pinned world image.
    pub image: Arc<Image>,
}

/// One stored snapshot entry.
#[derive(Debug, Clone)]
pub(crate) struct SnapshotEntry {
    /// The owning world handle.
    pub world_handle_id: ControlHandleId,
    /// The requested snapshot format.
    pub format: ControlSnapshotFormat,
    /// The stored serialized snapshot.
    pub snapshot: WorldSnapshot,
    /// The encoded snapshot bytes.
    pub bytes: Arc<[u8]>,
}

/// One externally visible control-object kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControlKind {
    /// One live world handle.
    World,
    /// One live runtime handle.
    Runtime,
    /// One live agent handle.
    Agent,
    /// One live observation handle.
    Observation,
    /// One stored snapshot handle.
    Snapshot,
    /// One live trace-cursor handle.
    TraceCursor,
    /// One pinned world-view handle.
    WorldView,
}

impl ControlKind {
    /// Return the display name for this control-object kind.
    fn name(self) -> &'static str {
        match self {
            ControlKind::World => "world",
            ControlKind::Runtime => "runtime",
            ControlKind::Agent => "agent",
            ControlKind::Observation => "observation",
            ControlKind::Snapshot => "snapshot",
            ControlKind::TraceCursor => "trace cursor",
            ControlKind::WorldView => "world view",
        }
    }
}

/// One process-global control-handle record.
#[derive(Debug, Clone, Copy)]
struct ControlEntry {
    /// The owning thread for this handle.
    owner_thread_id: ThreadId,
    /// The control-object kind.
    kind: ControlKind,
}

/// Process-global metadata for externally visible runtime control objects.
#[derive(Debug, Default)]
pub(crate) struct ControlTable {
    /// The next process-local control handle id.
    next_handle_id: u64,
    /// Registered control handles.
    handles: HashMap<ControlHandleId, ControlEntry>,
}

/// Thread-local live control objects.
#[derive(Debug, Default)]
struct ControlStore {
    /// Stored control objects keyed by their opaque handle id.
    objects: HashMap<ControlHandleId, ControlObject>,
}

/// One owner-thread control object stored under one external handle.
#[derive(Debug, Clone)]
enum ControlObject {
    /// One live world handle.
    World(WorldEntry),
    /// One live runtime handle.
    Runtime(RuntimeEntry),
    /// One live agent handle.
    Agent(AgentEntry),
    /// One live observation handle.
    Observation(ObservationEntry),
    /// One live trace-cursor handle.
    TraceCursor(TraceCursorEntry),
    /// One pinned world-view handle.
    WorldView(WorldViewEntry),
    /// One stored snapshot handle.
    Snapshot(SnapshotEntry),
}

impl ControlObject {
    /// Return the owning world handle for one attached control object.
    fn world_handle_id(&self) -> Option<ControlHandleId> {
        match self {
            ControlObject::World(_) => None,
            ControlObject::Runtime(entry) => Some(entry.world_handle_id),
            ControlObject::Agent(entry) => Some(entry.world_handle_id),
            ControlObject::Observation(entry) => Some(entry.world_handle_id),
            ControlObject::TraceCursor(entry) => Some(entry.world_handle_id),
            ControlObject::WorldView(entry) => Some(entry.world_handle_id),
            ControlObject::Snapshot(entry) => Some(entry.world_handle_id),
        }
    }
}

/// Return one internal control-store mismatch error.
fn control_store_kind_mismatch(
    handle_id: ControlHandleId,
    control_kind: ControlKind,
) -> Box<RuntimeError> {
    RuntimeError::Internal {
        message: format!(
            "control store kind mismatch for {} handle {}",
            control_kind.name(),
            handle_id.get()
        ),
    }
    .boxed()
}

impl ControlStore {
    /// Insert one control object under one handle id.
    fn insert_object(&mut self, handle_id: ControlHandleId, object: ControlObject) {
        self.objects.insert(handle_id, object);
    }

    /// Resolve one world entry.
    fn world_entry(&self, handle_id: ControlHandleId) -> RuntimeResult<&WorldEntry> {
        match self.objects.get(&handle_id) {
            Some(ControlObject::World(entry)) => Ok(entry),
            Some(_) => Err(control_store_kind_mismatch(handle_id, ControlKind::World)),
            None => Err(control_handle_not_found(handle_id, ControlKind::World)),
        }
    }

    /// Remove one world entry.
    fn remove_world_entry(&mut self, handle_id: ControlHandleId) -> RuntimeResult<WorldEntry> {
        match self.objects.remove(&handle_id) {
            Some(ControlObject::World(entry)) => Ok(entry),
            Some(_) => Err(control_store_kind_mismatch(handle_id, ControlKind::World)),
            None => Err(control_handle_not_found(handle_id, ControlKind::World)),
        }
    }

    /// Resolve one runtime entry.
    fn runtime_entry(&self, handle_id: ControlHandleId) -> RuntimeResult<&RuntimeEntry> {
        match self.objects.get(&handle_id) {
            Some(ControlObject::Runtime(entry)) => Ok(entry),
            Some(_) => Err(control_store_kind_mismatch(handle_id, ControlKind::Runtime)),
            None => Err(control_handle_not_found(handle_id, ControlKind::Runtime)),
        }
    }

    /// Remove one runtime entry.
    fn remove_runtime_entry(&mut self, handle_id: ControlHandleId) -> RuntimeResult<RuntimeEntry> {
        match self.objects.remove(&handle_id) {
            Some(ControlObject::Runtime(entry)) => Ok(entry),
            Some(_) => Err(control_store_kind_mismatch(handle_id, ControlKind::Runtime)),
            None => Err(control_handle_not_found(handle_id, ControlKind::Runtime)),
        }
    }

    /// Resolve one agent entry.
    fn agent_entry(&self, handle_id: ControlHandleId) -> RuntimeResult<&AgentEntry> {
        match self.objects.get(&handle_id) {
            Some(ControlObject::Agent(entry)) => Ok(entry),
            Some(_) => Err(control_store_kind_mismatch(handle_id, ControlKind::Agent)),
            None => Err(control_handle_not_found(handle_id, ControlKind::Agent)),
        }
    }

    /// Remove one agent entry.
    fn remove_agent_entry(&mut self, handle_id: ControlHandleId) -> RuntimeResult<AgentEntry> {
        match self.objects.remove(&handle_id) {
            Some(ControlObject::Agent(entry)) => Ok(entry),
            Some(_) => Err(control_store_kind_mismatch(handle_id, ControlKind::Agent)),
            None => Err(control_handle_not_found(handle_id, ControlKind::Agent)),
        }
    }

    /// Resolve one observation entry.
    fn observation_entry(&self, handle_id: ControlHandleId) -> RuntimeResult<&ObservationEntry> {
        match self.objects.get(&handle_id) {
            Some(ControlObject::Observation(entry)) => Ok(entry),
            Some(_) => Err(control_store_kind_mismatch(
                handle_id,
                ControlKind::Observation,
            )),
            None => Err(control_handle_not_found(
                handle_id,
                ControlKind::Observation,
            )),
        }
    }

    /// Remove one observation entry.
    fn remove_observation_entry(
        &mut self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<ObservationEntry> {
        match self.objects.remove(&handle_id) {
            Some(ControlObject::Observation(entry)) => Ok(entry),
            Some(_) => Err(control_store_kind_mismatch(
                handle_id,
                ControlKind::Observation,
            )),
            None => Err(control_handle_not_found(
                handle_id,
                ControlKind::Observation,
            )),
        }
    }

    /// Resolve one trace-cursor entry.
    fn trace_cursor_entry(&self, handle_id: ControlHandleId) -> RuntimeResult<&TraceCursorEntry> {
        match self.objects.get(&handle_id) {
            Some(ControlObject::TraceCursor(entry)) => Ok(entry),
            Some(_) => Err(control_store_kind_mismatch(
                handle_id,
                ControlKind::TraceCursor,
            )),
            None => Err(control_handle_not_found(
                handle_id,
                ControlKind::TraceCursor,
            )),
        }
    }

    /// Remove one trace-cursor entry.
    fn remove_trace_cursor_entry(
        &mut self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<TraceCursorEntry> {
        match self.objects.remove(&handle_id) {
            Some(ControlObject::TraceCursor(entry)) => Ok(entry),
            Some(_) => Err(control_store_kind_mismatch(
                handle_id,
                ControlKind::TraceCursor,
            )),
            None => Err(control_handle_not_found(
                handle_id,
                ControlKind::TraceCursor,
            )),
        }
    }

    /// Resolve one world-view entry.
    fn world_view_entry(&self, handle_id: ControlHandleId) -> RuntimeResult<&WorldViewEntry> {
        match self.objects.get(&handle_id) {
            Some(ControlObject::WorldView(entry)) => Ok(entry),
            Some(_) => Err(control_store_kind_mismatch(
                handle_id,
                ControlKind::WorldView,
            )),
            None => Err(control_handle_not_found(handle_id, ControlKind::WorldView)),
        }
    }

    /// Remove one world-view entry.
    fn remove_world_view_entry(
        &mut self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<WorldViewEntry> {
        match self.objects.remove(&handle_id) {
            Some(ControlObject::WorldView(entry)) => Ok(entry),
            Some(_) => Err(control_store_kind_mismatch(
                handle_id,
                ControlKind::WorldView,
            )),
            None => Err(control_handle_not_found(handle_id, ControlKind::WorldView)),
        }
    }

    /// Resolve one snapshot entry.
    fn snapshot_entry(&self, handle_id: ControlHandleId) -> RuntimeResult<&SnapshotEntry> {
        match self.objects.get(&handle_id) {
            Some(ControlObject::Snapshot(entry)) => Ok(entry),
            Some(_) => Err(control_store_kind_mismatch(
                handle_id,
                ControlKind::Snapshot,
            )),
            None => Err(control_handle_not_found(handle_id, ControlKind::Snapshot)),
        }
    }

    /// Return attached handles for one world.
    fn handles_for_world(&self, world_handle_id: ControlHandleId) -> Vec<ControlHandleId> {
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
    fn remove_object(&mut self, handle_id: ControlHandleId) {
        self.objects.remove(&handle_id);
    }
}

thread_local! {
    /// Thread-local live control objects for the current owner thread.
    static OWNER_CONTROL_STORE: RefCell<ControlStore> =
        RefCell::new(ControlStore::default());
}

/// Return the process-global runtime control table.
pub(crate) fn control_table() -> &'static RwLock<ControlTable> {
    static CONTROL_TABLE: OnceLock<RwLock<ControlTable>> = OnceLock::new();

    CONTROL_TABLE.get_or_init(|| RwLock::new(ControlTable::default()))
}

/// Return the current owner thread id.
fn owner_thread_id() -> ThreadId {
    thread::current().id()
}

/// Return one missing-handle error.
fn control_handle_not_found(handle_id: ControlHandleId, kind: ControlKind) -> Box<RuntimeError> {
    RuntimeError::ControlHandleNotFound {
        handle_id: handle_id.get(),
        kind: kind.name().to_string(),
    }
    .boxed()
}

/// Return one handle-kind mismatch error.
fn control_handle_kind_mismatch(
    handle_id: ControlHandleId,
    expected_kind: ControlKind,
    actual_kind: ControlKind,
) -> Box<RuntimeError> {
    RuntimeError::ControlHandleKindMismatch {
        handle_id: handle_id.get(),
        expected: expected_kind.name().to_string(),
        actual: actual_kind.name().to_string(),
    }
    .boxed()
}

impl ControlTable {
    /// Allocate one fresh control-handle id.
    fn allocate_handle_id(&mut self) -> ControlHandleId {
        let handle_id = self.next_handle_id.max(1);
        self.next_handle_id = handle_id + 1;

        ControlHandleId::new(handle_id)
    }

    /// Run one shared store read.
    fn with_store<T>(&self, f: impl FnOnce(&ControlStore) -> RuntimeResult<T>) -> RuntimeResult<T> {
        OWNER_CONTROL_STORE.with(|control_store| {
            let control_store = control_store.borrow();
            f(&control_store)
        })
    }

    /// Run one exclusive store access.
    fn with_store_mut<T>(
        &self,
        f: impl FnOnce(&mut ControlStore) -> RuntimeResult<T>,
    ) -> RuntimeResult<T> {
        OWNER_CONTROL_STORE.with(|control_store| {
            let mut control_store = control_store.borrow_mut();
            f(&mut control_store)
        })
    }

    /// Run one infallible store mutation.
    fn mutate_store(&self, f: impl FnOnce(&mut ControlStore)) {
        OWNER_CONTROL_STORE.with(|control_store| {
            let mut control_store = control_store.borrow_mut();
            f(&mut control_store);
        });
    }

    /// Require one handle to exist on the owner thread with the expected kind.
    fn require_kind(
        &self,
        handle_id: ControlHandleId,
        expected_kind: ControlKind,
    ) -> RuntimeResult<()> {
        let entry = self
            .handles
            .get(&handle_id)
            .copied()
            .ok_or_else(|| control_handle_not_found(handle_id, expected_kind))?;

        // kind mismatch
        if entry.kind != expected_kind {
            return Err(control_handle_kind_mismatch(
                handle_id,
                expected_kind,
                entry.kind,
            ));
        }

        // owner affinity
        if entry.owner_thread_id != owner_thread_id() {
            return Err(RuntimeError::AffinityViolation {
                name: expected_kind.name().to_string(),
                affinity: "owner".to_string(),
            }
            .boxed());
        }

        Ok(())
    }

    /// Remove one registered control handle.
    fn unregister_handle(&mut self, handle_id: ControlHandleId) {
        self.handles.remove(&handle_id);
    }

    /// Register one live world and return its external control handle.
    pub(crate) fn register_world(
        &mut self,
        world: Arc<World>,
        labels: BTreeMap<String, String>,
    ) -> ControlHandleId {
        let handle_id = self.allocate_handle_id();

        // process-global world handle
        self.handles.insert(
            handle_id,
            ControlEntry {
                owner_thread_id: owner_thread_id(),
                kind: ControlKind::World,
            },
        );

        // thread-local world storage
        self.mutate_store(|control_store| {
            control_store.insert_object(
                handle_id,
                ControlObject::World(WorldEntry {
                    world,
                    labels: WorldLabels { labels },
                }),
            );
        });

        handle_id
    }

    /// Close one live world and every attached control object.
    pub(crate) fn close_world(&mut self, handle_id: ControlHandleId) -> RuntimeResult<()> {
        self.require_kind(handle_id, ControlKind::World)?;

        // detach local objects
        let attached_handle_ids = self.with_store_mut(|control_store| {
            let _world_entry = control_store.remove_world_entry(handle_id)?;
            let attached_handle_ids = control_store.handles_for_world(handle_id);

            for attached_handle_id in &attached_handle_ids {
                control_store.remove_object(*attached_handle_id);
            }

            Ok(attached_handle_ids)
        })?;

        // remove process-global metadata
        for attached_handle_id in attached_handle_ids {
            self.unregister_handle(attached_handle_id);
        }

        self.unregister_handle(handle_id);

        Ok(())
    }

    /// Resolve one live world handle.
    pub(crate) fn world(&self, handle_id: ControlHandleId) -> RuntimeResult<Arc<World>> {
        self.require_kind(handle_id, ControlKind::World)?;

        self.with_store(|control_store| {
            let entry = control_store.world_entry(handle_id)?;
            Ok(entry.world.clone())
        })
    }

    /// Resolve one live world entry.
    pub(crate) fn world_entry(&self, handle_id: ControlHandleId) -> RuntimeResult<WorldEntry> {
        self.require_kind(handle_id, ControlKind::World)?;

        self.with_store(|control_store| {
            let entry = control_store.world_entry(handle_id)?;
            Ok(entry.clone())
        })
    }

    /// Resolve stored labels for one live world handle.
    pub(crate) fn world_labels(
        &self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<BTreeMap<String, String>> {
        self.require_kind(handle_id, ControlKind::World)?;

        self.with_store(|control_store| {
            let entry = control_store.world_entry(handle_id)?;
            Ok(entry.labels.labels.clone())
        })
    }

    /// Register one runtime handle and return its external control handle.
    pub(crate) fn register_runtime(
        &mut self,
        world_handle_id: ControlHandleId,
        world: Arc<World>,
        runtime_id: RuntimeId,
    ) -> ControlHandleId {
        let handle_id = self.allocate_handle_id();

        // process-global runtime handle
        self.handles.insert(
            handle_id,
            ControlEntry {
                owner_thread_id: owner_thread_id(),
                kind: ControlKind::Runtime,
            },
        );

        // thread-local runtime storage
        self.mutate_store(|control_store| {
            control_store.insert_object(
                handle_id,
                ControlObject::Runtime(RuntimeEntry {
                    world_handle_id,
                    world,
                    runtime_id,
                }),
            );
        });

        handle_id
    }

    /// Close one runtime handle and return its stored entry.
    pub(crate) fn close_runtime(
        &mut self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<RuntimeEntry> {
        self.require_kind(handle_id, ControlKind::Runtime)?;

        let entry =
            self.with_store_mut(|control_store| control_store.remove_runtime_entry(handle_id))?;

        self.unregister_handle(handle_id);

        Ok(entry)
    }

    /// Resolve one runtime handle into its live world and runtime id.
    pub(crate) fn runtime(
        &self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<(Arc<World>, RuntimeId)> {
        self.require_kind(handle_id, ControlKind::Runtime)?;

        self.with_store(|control_store| {
            let entry = control_store.runtime_entry(handle_id)?;
            Ok((entry.world.clone(), entry.runtime_id))
        })
    }

    /// Resolve one runtime handle into its stored entry.
    pub(crate) fn runtime_entry(&self, handle_id: ControlHandleId) -> RuntimeResult<RuntimeEntry> {
        self.require_kind(handle_id, ControlKind::Runtime)?;

        self.with_store(|control_store| {
            let entry = control_store.runtime_entry(handle_id)?;
            Ok(entry.clone())
        })
    }

    /// Register one agent handle and return its external control handle.
    pub(crate) fn register_agent(
        &mut self,
        world_handle_id: ControlHandleId,
        world: Arc<World>,
        runtime_id: RuntimeId,
        agent_id: AgentId,
    ) -> ControlHandleId {
        let handle_id = self.allocate_handle_id();

        // process-global agent handle
        self.handles.insert(
            handle_id,
            ControlEntry {
                owner_thread_id: owner_thread_id(),
                kind: ControlKind::Agent,
            },
        );

        // thread-local agent storage
        self.mutate_store(|control_store| {
            control_store.insert_object(
                handle_id,
                ControlObject::Agent(AgentEntry {
                    world_handle_id,
                    world,
                    runtime_id,
                    agent_id,
                }),
            );
        });

        handle_id
    }

    /// Close one agent handle and return its stored entry.
    pub(crate) fn close_agent(&mut self, handle_id: ControlHandleId) -> RuntimeResult<AgentEntry> {
        self.require_kind(handle_id, ControlKind::Agent)?;

        let entry =
            self.with_store_mut(|control_store| control_store.remove_agent_entry(handle_id))?;

        self.unregister_handle(handle_id);

        Ok(entry)
    }

    /// Resolve one agent handle into its live world and agent id.
    pub(crate) fn agent(
        &self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<(Arc<World>, RuntimeId, AgentId)> {
        self.require_kind(handle_id, ControlKind::Agent)?;

        self.with_store(|control_store| {
            let entry = control_store.agent_entry(handle_id)?;
            Ok((entry.world.clone(), entry.runtime_id, entry.agent_id))
        })
    }

    /// Remove agent handles for one world and one set of agent ids.
    pub(crate) fn close_agent_handles(
        &mut self,
        world_handle_id: ControlHandleId,
        agent_ids: &[AgentId],
    ) -> RuntimeResult<()> {
        let handle_ids = self.with_store_mut(|control_store| {
            let handle_ids = control_store
                .objects
                .iter()
                .filter_map(|(handle_id, object)| match object {
                    ControlObject::Agent(entry)
                        if entry.world_handle_id == world_handle_id
                            && agent_ids.contains(&entry.agent_id) =>
                    {
                        Some(*handle_id)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();

            for handle_id in &handle_ids {
                control_store.remove_agent_entry(*handle_id)?;
            }

            Ok(handle_ids)
        })?;

        for handle_id in handle_ids {
            self.unregister_handle(handle_id);
        }

        Ok(())
    }

    /// Register one observation subscription and return its external control handle.
    pub(crate) fn open_observation(
        &mut self,
        world_handle_id: ControlHandleId,
        world: Arc<World>,
        subscription_id: ObservationSubscriptionId,
    ) -> ControlHandleId {
        let handle_id = self.allocate_handle_id();

        // process-global observation handle
        self.handles.insert(
            handle_id,
            ControlEntry {
                owner_thread_id: owner_thread_id(),
                kind: ControlKind::Observation,
            },
        );

        // thread-local observation storage
        self.mutate_store(|control_store| {
            control_store.insert_object(
                handle_id,
                ControlObject::Observation(ObservationEntry {
                    world_handle_id,
                    world,
                    subscription_id,
                }),
            );
        });

        handle_id
    }

    /// Open one observation handle under one live world handle.
    pub(crate) fn open_observation_handle(
        &mut self,
        world_handle_id: ControlHandleId,
        subscription_id: ObservationSubscriptionId,
    ) -> RuntimeResult<ControlHandleId> {
        let world = self.world(world_handle_id)?;

        Ok(self.open_observation(world_handle_id, world, subscription_id))
    }

    /// Resolve one observation handle into its live world and subscription id.
    pub(crate) fn observation(
        &self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<(Arc<World>, ObservationSubscriptionId)> {
        self.require_kind(handle_id, ControlKind::Observation)?;

        self.with_store(|control_store| {
            let entry = control_store.observation_entry(handle_id)?;
            Ok((entry.world.clone(), entry.subscription_id))
        })
    }

    /// Close one observation handle and return its stored entry.
    pub(crate) fn close_observation(
        &mut self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<ObservationEntry> {
        self.require_kind(handle_id, ControlKind::Observation)?;

        let entry =
            self.with_store_mut(|control_store| control_store.remove_observation_entry(handle_id))?;

        self.unregister_handle(handle_id);

        Ok(entry)
    }

    /// Register one stored snapshot and return its external handle.
    pub(crate) fn store_snapshot(
        &mut self,
        world_handle_id: ControlHandleId,
        format: ControlSnapshotFormat,
        snapshot: WorldSnapshot,
        bytes: Arc<[u8]>,
    ) -> ControlHandleId {
        let handle_id = self.allocate_handle_id();

        // process-global snapshot handle
        self.handles.insert(
            handle_id,
            ControlEntry {
                owner_thread_id: owner_thread_id(),
                kind: ControlKind::Snapshot,
            },
        );

        // thread-local snapshot storage
        self.mutate_store(|control_store| {
            control_store.insert_object(
                handle_id,
                ControlObject::Snapshot(SnapshotEntry {
                    world_handle_id,
                    format,
                    snapshot,
                    bytes,
                }),
            );
        });

        handle_id
    }

    /// Store one snapshot handle under one live world handle.
    pub(crate) fn store_snapshot_handle(
        &mut self,
        world_handle_id: ControlHandleId,
        format: ControlSnapshotFormat,
        snapshot: WorldSnapshot,
        bytes: Arc<[u8]>,
    ) -> RuntimeResult<ControlHandleId> {
        self.require_kind(world_handle_id, ControlKind::World)?;

        Ok(self.store_snapshot(world_handle_id, format, snapshot, bytes))
    }

    /// Resolve one stored snapshot entry.
    pub(crate) fn snapshot(&self, handle_id: ControlHandleId) -> RuntimeResult<SnapshotEntry> {
        self.require_kind(handle_id, ControlKind::Snapshot)?;

        self.with_store(|control_store| {
            let entry = control_store.snapshot_entry(handle_id)?;
            Ok(entry.clone())
        })
    }

    /// List stored snapshots for one world in stable id order.
    pub(crate) fn snapshots_for_world(
        &self,
        world_handle_id: ControlHandleId,
        after: Option<ControlHandleId>,
        limit: Option<usize>,
    ) -> Vec<(ControlHandleId, SnapshotEntry)> {
        let after = after.unwrap_or(ControlHandleId::new(0));

        self.with_store(|control_store| {
            let mut snapshots = control_store
                .objects
                .iter()
                .filter_map(|(handle_id, object)| match object {
                    ControlObject::Snapshot(entry)
                        if entry.world_handle_id == world_handle_id && *handle_id > after =>
                    {
                        Some((*handle_id, entry.clone()))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();

            snapshots.sort_by_key(|(handle_id, _)| *handle_id);

            if let Some(limit) = limit {
                snapshots.truncate(limit);
            }

            Ok(snapshots)
        })
        .unwrap_or_default()
    }

    /// Register one trace cursor and return its external control handle.
    pub(crate) fn open_trace_cursor(
        &mut self,
        world_handle_id: ControlHandleId,
        world: Arc<World>,
        cursor: Arc<TraceCursor>,
    ) -> ControlHandleId {
        let handle_id = self.allocate_handle_id();

        // process-global trace-cursor handle
        self.handles.insert(
            handle_id,
            ControlEntry {
                owner_thread_id: owner_thread_id(),
                kind: ControlKind::TraceCursor,
            },
        );

        // thread-local cursor storage
        self.mutate_store(|control_store| {
            control_store.insert_object(
                handle_id,
                ControlObject::TraceCursor(TraceCursorEntry {
                    world_handle_id,
                    world,
                    cursor,
                }),
            );
        });

        handle_id
    }

    /// Open one trace cursor handle under one live world handle.
    pub(crate) fn open_trace_cursor_handle(
        &mut self,
        world_handle_id: ControlHandleId,
        cursor: Arc<TraceCursor>,
    ) -> RuntimeResult<ControlHandleId> {
        let world = self.world(world_handle_id)?;

        Ok(self.open_trace_cursor(world_handle_id, world, cursor))
    }

    /// Resolve one trace cursor handle into its live world and cursor.
    pub(crate) fn trace_cursor(
        &self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<(Arc<World>, Arc<TraceCursor>)> {
        self.require_kind(handle_id, ControlKind::TraceCursor)?;

        self.with_store(|control_store| {
            let entry = control_store.trace_cursor_entry(handle_id)?;
            Ok((entry.world.clone(), entry.cursor.clone()))
        })
    }

    /// Close one trace cursor handle and return its stored entry.
    pub(crate) fn close_trace_cursor(
        &mut self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<TraceCursorEntry> {
        self.require_kind(handle_id, ControlKind::TraceCursor)?;

        let entry = self
            .with_store_mut(|control_store| control_store.remove_trace_cursor_entry(handle_id))?;

        self.unregister_handle(handle_id);

        Ok(entry)
    }

    /// Open one pinned world view and return its external control handle.
    pub(crate) fn open_world_view(
        &mut self,
        world_handle_id: ControlHandleId,
        revision_id: RevisionId,
    ) -> RuntimeResult<ControlHandleId> {
        let world_entry = self.world_entry(world_handle_id)?;
        let backing = world_entry.world.revision_backing(revision_id)?;
        let handle_id = self.allocate_handle_id();

        // process-global world-view handle
        self.handles.insert(
            handle_id,
            ControlEntry {
                owner_thread_id: owner_thread_id(),
                kind: ControlKind::WorldView,
            },
        );

        // thread-local view storage
        self.mutate_store(|control_store| {
            control_store.insert_object(
                handle_id,
                ControlObject::WorldView(WorldViewEntry {
                    world_handle_id,
                    world: world_entry.world,
                    labels: world_entry.labels,
                    revision: backing.revision,
                    image: backing.image,
                }),
            );
        });

        Ok(handle_id)
    }

    /// Resolve one pinned world view entry.
    pub(crate) fn world_view(&self, handle_id: ControlHandleId) -> RuntimeResult<WorldViewEntry> {
        self.require_kind(handle_id, ControlKind::WorldView)?;

        self.with_store(|control_store| {
            let entry = control_store.world_view_entry(handle_id)?;
            Ok(entry.clone())
        })
    }

    /// Close one pinned world view and return its stored entry.
    pub(crate) fn close_world_view(
        &mut self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<WorldViewEntry> {
        self.require_kind(handle_id, ControlKind::WorldView)?;

        let entry =
            self.with_store_mut(|control_store| control_store.remove_world_view_entry(handle_id))?;

        self.unregister_handle(handle_id);

        Ok(entry)
    }
}
