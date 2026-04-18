use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::runtime::world::{Revision, WorldSnapshot};

use super::handle::{ControlEntry, ControlHandleId, ControlKind, WorldLabels};
use super::object::{ControlObject, SnapshotEntry, WorldViewEntry};
use super::{Control, ControlSnapshotFormat};

impl Control {
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
                kind: ControlKind::Snapshot,
            },
        );

        // snapshot storage
        self.insert_object(
            handle_id,
            ControlObject::Snapshot(Box::new(SnapshotEntry {
                world_handle_id,
                format,
                snapshot,
                bytes,
            })),
        );

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
        let entry = self.get_snapshot_entry(handle_id)?;

        Ok(entry.clone())
    }

    /// List stored snapshots for one world in stable id order.
    pub(crate) fn snapshots_for_world(
        &self,
        world_handle_id: ControlHandleId,
        after: Option<ControlHandleId>,
        limit: Option<usize>,
    ) -> Vec<(ControlHandleId, SnapshotEntry)> {
        let after = after.unwrap_or(ControlHandleId::new(0));

        let mut snapshots = self
            .objects
            .iter()
            .filter_map(|(handle_id, object)| match object {
                ControlObject::Snapshot(entry)
                    if entry.world_handle_id == world_handle_id && *handle_id > after =>
                {
                    Some((*handle_id, entry.as_ref().clone()))
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        snapshots.sort_by_key(|(handle_id, _)| *handle_id);

        if let Some(limit) = limit {
            snapshots.truncate(limit);
        }

        snapshots
    }

    /// Open one pinned world view and return its external control handle.
    pub(crate) fn open_world_view(
        &mut self,
        world_handle_id: ControlHandleId,
        revision: Revision,
    ) -> RuntimeResult<ControlHandleId> {
        let labels = self.world_labels(world_handle_id)?;
        let world = self.world(world_handle_id)?;
        let revision_handle = revision;
        let (revision, image, _) = world.revision_data(revision_handle)?;
        let handle_id = self.allocate_handle_id();

        // process-global world-view handle
        self.handles.insert(
            handle_id,
            ControlEntry {
                kind: ControlKind::WorldView,
            },
        );

        // view storage
        self.insert_object(
            handle_id,
            ControlObject::WorldView(WorldViewEntry {
                world_handle_id,
                labels: WorldLabels { labels },
                revision_handle,
                revision,
                image,
            }),
        );

        Ok(handle_id)
    }

    /// Resolve one pinned world view entry.
    pub(crate) fn world_view(&self, handle_id: ControlHandleId) -> RuntimeResult<WorldViewEntry> {
        self.require_kind(handle_id, ControlKind::WorldView)?;

        let entry = self.get_world_view_entry(handle_id)?;

        Ok(entry.clone())
    }

    /// Close one pinned world view and return its stored entry.
    pub(crate) fn close_world_view(
        &mut self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<WorldViewEntry> {
        self.require_kind(handle_id, ControlKind::WorldView)?;

        let entry = self.take_world_view_entry(handle_id)?;

        self.unregister_handle(handle_id);

        Ok(entry)
    }
}
