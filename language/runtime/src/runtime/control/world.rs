use std::collections::BTreeMap;

use crate::diagnostic::RuntimeResult;
use crate::runtime::{AgentId, RuntimeId, World};

use super::Control;
use super::handle::{ControlEntry, ControlHandleId, ControlKind, WorldLabels};
use super::object::{AgentEntry, ControlObject, RuntimeEntry, WorldEntry};

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

    /// Register one agent handle and return its external control handle.
    pub(crate) fn register_agent(
        &mut self,
        world_handle_id: ControlHandleId,
        runtime_id: RuntimeId,
        agent_id: AgentId,
    ) -> ControlHandleId {
        let handle_id = self.allocate_handle_id();

        // process-global agent handle
        self.handles.insert(
            handle_id,
            ControlEntry {
                kind: ControlKind::Agent,
            },
        );

        // agent storage
        self.insert_object(
            handle_id,
            ControlObject::Agent(AgentEntry {
                world_handle_id,
                runtime_id,
                agent_id,
            }),
        );

        handle_id
    }

    /// Close one agent handle and return its stored entry.
    pub(crate) fn close_agent(&mut self, handle_id: ControlHandleId) -> RuntimeResult<AgentEntry> {
        self.require_kind(handle_id, ControlKind::Agent)?;

        let entry = self.take_agent_entry(handle_id)?;

        self.unregister_handle(handle_id);

        Ok(entry)
    }

    /// Resolve one agent handle into its stored entry.
    pub(crate) fn agent_entry(&self, handle_id: ControlHandleId) -> RuntimeResult<AgentEntry> {
        self.require_kind(handle_id, ControlKind::Agent)?;

        let entry = self.get_agent_entry(handle_id)?;

        Ok(entry.clone())
    }

    /// Remove agent handles for one world and one set of agent ids.
    pub(crate) fn close_agent_handles(
        &mut self,
        world_handle_id: ControlHandleId,
        agent_ids: &[AgentId],
    ) -> RuntimeResult<()> {
        let handle_ids = self
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
            let _entry = self.take_agent_entry(*handle_id)?;
        }

        for handle_id in handle_ids {
            self.unregister_handle(handle_id);
        }

        Ok(())
    }
}
