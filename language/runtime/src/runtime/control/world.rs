use std::collections::BTreeMap;

use crate::diagnostic::RuntimeResult;
use crate::runtime::{RuntimeId, WorkerId, World};

use super::Control;
use super::handle::{ControlEntry, ControlHandleId, ControlKind, WorldLabels};
use super::object::{ControlObject, RuntimeEntry, WorkerEntry, WorldEntry};

impl Control {
    /// Register one live world and return its external control handle.
    pub(crate) fn register_world(
        &mut self,
        world: World,
        labels: BTreeMap<String, String>,
    ) -> ControlHandleId {
        let handle_id = self.allocate_handle_id();

        // process-global world handle
        self.handles.insert(
            handle_id,
            ControlEntry {
                kind: ControlKind::World,
            },
        );

        // world storage
        self.insert_object(
            handle_id,
            ControlObject::World(WorldEntry {
                world: Box::new(world),
                labels: WorldLabels { labels },
            }),
        );

        handle_id
    }

    /// Close one live world and every attached control object.
    pub(crate) fn close_world(&mut self, handle_id: ControlHandleId) -> RuntimeResult<()> {
        self.require_kind(handle_id, ControlKind::World)?;

        // detach local objects
        let _world_entry = self.take_world_entry(handle_id)?;
        let attached_handle_ids = self.handles_for_world(handle_id);

        for attached_handle_id in &attached_handle_ids {
            self.remove_object(*attached_handle_id);
        }

        for attached_handle_id in attached_handle_ids {
            self.unregister_handle(attached_handle_id);
        }

        self.unregister_handle(handle_id);

        Ok(())
    }

    /// Borrow one live world immutably.
    pub(crate) fn world(&self, handle_id: ControlHandleId) -> RuntimeResult<&World> {
        self.require_kind(handle_id, ControlKind::World)?;
        let entry = self.get_world_entry(handle_id)?;

        Ok(entry.world.as_ref())
    }

    /// Borrow one live world mutably.
    pub(crate) fn world_mut(&mut self, handle_id: ControlHandleId) -> RuntimeResult<&mut World> {
        self.require_kind(handle_id, ControlKind::World)?;
        let entry = self.get_world_entry_mut(handle_id)?;

        Ok(entry.world.as_mut())
    }

    /// Resolve stored labels for one live world handle.
    pub(crate) fn world_labels(
        &self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<BTreeMap<String, String>> {
        self.require_kind(handle_id, ControlKind::World)?;
        let entry = self.get_world_entry(handle_id)?;

        Ok(entry.labels.labels.clone())
    }

    /// Register one runtime handle and return its external control handle.
    pub(crate) fn register_runtime(
        &mut self,
        world_handle_id: ControlHandleId,
        runtime_id: RuntimeId,
    ) -> ControlHandleId {
        let handle_id = self.allocate_handle_id();

        // process-global runtime handle
        self.handles.insert(
            handle_id,
            ControlEntry {
                kind: ControlKind::Runtime,
            },
        );

        // runtime storage
        self.insert_object(
            handle_id,
            ControlObject::Runtime(RuntimeEntry {
                world_handle_id,
                runtime_id,
            }),
        );

        handle_id
    }

    /// Close one runtime handle and return its stored entry.
    pub(crate) fn close_runtime(
        &mut self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<RuntimeEntry> {
        self.require_kind(handle_id, ControlKind::Runtime)?;

        let entry = self.take_runtime_entry(handle_id)?;

        self.unregister_handle(handle_id);

        Ok(entry)
    }

    /// Resolve one runtime handle into its stored entry.
    pub(crate) fn runtime_entry(&self, handle_id: ControlHandleId) -> RuntimeResult<RuntimeEntry> {
        self.require_kind(handle_id, ControlKind::Runtime)?;

        let entry = self.get_runtime_entry(handle_id)?;

        Ok(entry.clone())
    }

    /// Register one worker handle and return its external control handle.
    pub(crate) fn register_worker(
        &mut self,
        world_handle_id: ControlHandleId,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
    ) -> ControlHandleId {
        let handle_id = self.allocate_handle_id();

        // process-global worker handle
        self.handles.insert(
            handle_id,
            ControlEntry {
                kind: ControlKind::Worker,
            },
        );

        // worker storage
        self.insert_object(
            handle_id,
            ControlObject::Worker(WorkerEntry {
                world_handle_id,
                runtime_id,
                worker_id,
            }),
        );

        handle_id
    }

    /// Close one worker handle and return its stored entry.
    pub(crate) fn close_worker(
        &mut self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<WorkerEntry> {
        self.require_kind(handle_id, ControlKind::Worker)?;

        let entry = self.take_worker_entry(handle_id)?;

        self.unregister_handle(handle_id);

        Ok(entry)
    }

    /// Resolve one worker handle into its stored entry.
    pub(crate) fn worker_entry(&self, handle_id: ControlHandleId) -> RuntimeResult<WorkerEntry> {
        self.require_kind(handle_id, ControlKind::Worker)?;

        let entry = self.get_worker_entry(handle_id)?;

        Ok(entry.clone())
    }

    /// Remove worker handles for one world and one set of worker ids.
    pub(crate) fn close_worker_handles(
        &mut self,
        world_handle_id: ControlHandleId,
        worker_ids: &[WorkerId],
    ) -> RuntimeResult<()> {
        let handle_ids = self
            .objects
            .iter()
            .filter_map(|(handle_id, object)| match object {
                ControlObject::Worker(entry)
                    if entry.world_handle_id == world_handle_id
                        && worker_ids.contains(&entry.worker_id) =>
                {
                    Some(*handle_id)
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        for handle_id in &handle_ids {
            let _entry = self.take_worker_entry(*handle_id)?;
        }

        for handle_id in handle_ids {
            self.unregister_handle(handle_id);
        }

        Ok(())
    }
}
