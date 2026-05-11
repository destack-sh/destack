use crate::diagnostic::RuntimeResult;
use crate::runtime::trace::{ObservationSubscriptionId, TraceCursor};

use super::Control;
use super::handle::{ControlEntry, ControlHandleId, ControlKind};
use super::object::{ControlObject, ObservationEntry, TraceCursorEntry};

impl Control {
    /// Register one observation subscription and return its external control handle.
    pub(crate) fn open_observation(
        &mut self,
        world_handle_id: ControlHandleId,
        subscription_id: ObservationSubscriptionId,
    ) -> ControlHandleId {
        let handle_id = self.allocate_handle_id();

        // process-global observation handle
        self.handles.insert(
            handle_id,
            ControlEntry {
                kind: ControlKind::Observation,
            },
        );

        // observation storage
        self.insert_object(
            handle_id,
            ControlObject::Observation(ObservationEntry {
                world_handle_id,
                subscription_id,
            }),
        );

        handle_id
    }

    /// Open one observation handle under one live world handle.
    pub(crate) fn open_observation_handle(
        &mut self,
        world_handle_id: ControlHandleId,
        subscription_id: ObservationSubscriptionId,
    ) -> RuntimeResult<ControlHandleId> {
        self.require_kind(world_handle_id, ControlKind::World)?;

        Ok(self.open_observation(world_handle_id, subscription_id))
    }

    /// Resolve one observation handle into its stored entry.
    pub(crate) fn observation_entry(
        &self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<ObservationEntry> {
        self.require_kind(handle_id, ControlKind::Observation)?;

        let entry = self.get_observation_entry(handle_id)?;

        Ok(entry.clone())
    }

    /// Close one observation handle and return its stored entry.
    pub(crate) fn close_observation(
        &mut self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<ObservationEntry> {
        self.require_kind(handle_id, ControlKind::Observation)?;

        let entry = self.take_observation_entry(handle_id)?;

        self.unregister_handle(handle_id);

        Ok(entry)
    }

    /// Register one trace cursor and return its external control handle.
    pub(crate) fn open_trace_cursor(
        &mut self,
        world_handle_id: ControlHandleId,
        cursor: TraceCursor,
    ) -> ControlHandleId {
        let handle_id = self.allocate_handle_id();

        // process-global trace-cursor handle
        self.handles.insert(
            handle_id,
            ControlEntry {
                kind: ControlKind::TraceCursor,
            },
        );

        // cursor storage
        self.insert_object(
            handle_id,
            ControlObject::TraceCursor(TraceCursorEntry {
                world_handle_id,
                cursor,
            }),
        );

        handle_id
    }

    /// Open one trace cursor handle under one live world handle.
    pub(crate) fn open_trace_cursor_handle(
        &mut self,
        world_handle_id: ControlHandleId,
        cursor: TraceCursor,
    ) -> RuntimeResult<ControlHandleId> {
        self.require_kind(world_handle_id, ControlKind::World)?;

        Ok(self.open_trace_cursor(world_handle_id, cursor))
    }

    /// Resolve one trace cursor handle into its stored entry.
    pub(crate) fn trace_cursor_entry(
        &self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<&TraceCursorEntry> {
        self.require_kind(handle_id, ControlKind::TraceCursor)?;

        self.get_trace_cursor_entry(handle_id)
    }

    /// Resolve one trace cursor handle into its stored mutable entry.
    pub(crate) fn trace_cursor_entry_mut(
        &mut self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<&mut TraceCursorEntry> {
        self.require_kind(handle_id, ControlKind::TraceCursor)?;

        self.get_trace_cursor_entry_mut(handle_id)
    }

    /// Close one trace cursor handle and return its stored entry.
    pub(crate) fn close_trace_cursor(
        &mut self,
        handle_id: ControlHandleId,
    ) -> RuntimeResult<TraceCursorEntry> {
        self.require_kind(handle_id, ControlKind::TraceCursor)?;

        let entry = self.take_trace_cursor_entry(handle_id)?;

        self.unregister_handle(handle_id);

        Ok(entry)
    }
}
