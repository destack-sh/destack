use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::diagnostic::{RuntimeError, RuntimeResult};

use super::Control;

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

/// One externally visible control-object kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ControlKind {
    /// One live world handle.
    World,
    /// One live runtime handle.
    Runtime,
    /// One live worker handle.
    Worker,
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
    pub(super) fn name(self) -> &'static str {
        match self {
            ControlKind::World => "world",
            ControlKind::Runtime => "runtime",
            ControlKind::Worker => "worker",
            ControlKind::Observation => "observation",
            ControlKind::Snapshot => "snapshot",
            ControlKind::TraceCursor => "trace cursor",
            ControlKind::WorldView => "world view",
        }
    }
}

/// One process-global control-handle record.
#[derive(Debug, Clone, Copy)]
pub(super) struct ControlEntry {
    /// The control-object kind.
    pub kind: ControlKind,
}

impl Control {
    /// Allocate one fresh control-handle id.
    pub(super) fn allocate_handle_id(&mut self) -> ControlHandleId {
        static NEXT_CONTROL_HANDLE_ID: AtomicU64 = AtomicU64::new(1);
        let handle_id = NEXT_CONTROL_HANDLE_ID.fetch_add(1, Ordering::Relaxed);

        ControlHandleId::new(handle_id)
    }

    /// Require one handle to exist on the owner thread with the expected kind.
    pub(super) fn require_kind(
        &self,
        handle_id: ControlHandleId,
        expected_kind: ControlKind,
    ) -> RuntimeResult<()> {
        let entry = self.handles.get(&handle_id).copied().ok_or_else(|| {
            RuntimeError::ControlHandleNotFound {
                handle_id: handle_id.get(),
                kind: expected_kind.name().to_string(),
            }
            .boxed()
        })?;

        // kind mismatch
        if entry.kind != expected_kind {
            return Err(RuntimeError::ControlHandleKindMismatch {
                handle_id: handle_id.get(),
                expected: expected_kind.name().to_string(),
                actual: entry.kind.name().to_string(),
            }
            .boxed());
        }

        Ok(())
    }

    /// Remove one registered control handle.
    pub(super) fn unregister_handle(&mut self, handle_id: ControlHandleId) {
        self.handles.remove(&handle_id);
    }
}
