use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tspp_program as program;
use tspp_repository::ExecutionMode;

use crate::debugger::{
    Breakpoint, MemoryFilter, PointFilter, Probe, ProbeAction, ProbeFilter, ProbeId, Watchpoint,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::ResourceId;
use crate::runtime::{RuntimeId, RuntimeImage};
use crate::worker::{WorkerId, WorkerImage};
use crate::world::observation::Observation;
use crate::world::policy::{Policy, Rule, RuleId};

use super::{
    Edge, EdgeDefinition, EdgeId, EdgeKind, Entity, EntityDefinition, EntityId, EntityKind,
    RestoreContext, World,
};

/// One world mutation recorded in authoritative trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum Mutation {
    // runtime
    /// Add one runtime to the world.
    SpawnRuntime {
        /// The created runtime identifier.
        runtime_id: RuntimeId,
        /// The created runtime topology entity.
        runtime_entity: Entity,
        /// Captured runtime image.
        runtime: Arc<RuntimeImage>,
        /// Captured workers keyed by worker identifier.
        workers: BTreeMap<WorkerId, SpawnedWorker>,
    },
    /// One runtime removal mutation.
    RemoveRuntime {
        /// Runtime identifier to remove.
        runtime_id: RuntimeId,
    },
    /// Add one worker to an existing runtime.
    SpawnWorker {
        /// The owning runtime identifier.
        runtime_id: RuntimeId,
        /// The created worker identifier.
        worker_id: WorkerId,
        /// The created worker topology entity.
        worker_entity: Entity,
        /// Captured worker image.
        worker: Arc<WorkerImage>,
    },
    /// Remove one worker from the world.
    RemoveWorker {
        /// Worker identifier to remove.
        worker_id: WorkerId,
    },

    // resources
    /// Add one logical world resource.
    AddResource {
        /// Resource identifier to add.
        resource_id: ResourceId,
        /// Topology entity for this resource.
        entity: Entity,
    },
    /// Remove one logical world resource.
    RemoveResource {
        /// Resource identifier to remove.
        resource_id: ResourceId,
    },

    // policy
    /// Replace the active world policy.
    SetPolicy {
        /// Full policy replacement.
        policy: Policy,
    },
    /// Add one rule into the active world policy.
    AddRule {
        /// Rule to add.
        rule: Rule,
    },
    /// Remove one rule from the active world policy.
    RemoveRule {
        /// Stable rule identifier.
        rule_id: RuleId,
    },
    /// Enable one rule in the active world policy.
    EnableRule {
        /// Stable rule identifier.
        rule_id: RuleId,
    },
    /// Disable one rule in the active world policy.
    DisableRule {
        /// Stable rule identifier.
        rule_id: RuleId,
    },
    /// Replace one rule in the active world policy.
    ReplaceRule {
        /// Stable rule identifier to replace.
        rule_id: RuleId,
        /// Replacement rule.
        rule: Rule,
    },

    // debugger
    /// Add one debugger breakpoint.
    AddBreakpoint {
        /// Breakpoint to add.
        breakpoint: Breakpoint,
    },
    /// Update one debugger breakpoint.
    UpdateBreakpoint {
        /// Breakpoint to update.
        breakpoint: Breakpoint,
    },
    /// Remove one debugger breakpoint.
    RemoveBreakpoint {
        /// Breakpoint identifier to remove.
        breakpoint_id: program::BreakpointId,
    },
    /// Add one debugger watchpoint.
    AddWatchpoint {
        /// Watchpoint to add.
        watchpoint: Watchpoint,
    },
    /// Update one debugger watchpoint.
    UpdateWatchpoint {
        /// Watchpoint to update.
        watchpoint: Watchpoint,
    },
    /// Remove one debugger watchpoint.
    RemoveWatchpoint {
        /// Watchpoint identifier to remove.
        watchpoint_id: program::WatchpointId,
    },
    /// Add one debugger probe.
    AddProbe {
        /// Probe to add.
        probe: Probe,
    },
    /// Update one debugger probe.
    UpdateProbe {
        /// Probe to update.
        probe: Probe,
    },
    /// Remove one debugger probe.
    RemoveProbe {
        /// Probe identifier to remove.
        probe_id: ProbeId,
    },

    // topology
    /// Define one entity kind in world topology.
    DefineEntityKind {
        /// Entity kind definition.
        kind: EntityDefinition,
    },
    /// Undefine one entity kind in world topology.
    UndefineEntityKind {
        /// Entity kind identifier.
        kind: EntityKind,
    },
    /// Define one edge kind in world topology.
    DefineEdgeKind {
        /// Edge kind definition.
        kind: EdgeDefinition,
    },
    /// Undefine one edge kind in world topology.
    UndefineEdgeKind {
        /// Edge kind identifier.
        kind: EdgeKind,
    },
    /// Upsert one entity in world topology.
    UpsertEntity {
        /// Entity to upsert.
        entity: Entity,
    },
    /// Remove one entity from world topology.
    RemoveEntity {
        /// Stable entity identifier.
        entity_id: EntityId,
    },
    /// Upsert one edge in world topology.
    UpsertEdge {
        /// Edge to upsert.
        edge: Edge,
    },
    /// Remove one edge from world topology.
    RemoveEdge {
        /// Stable edge identifier.
        edge_id: EdgeId,
    },
}

/// One worker image paired with its spawn identity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpawnedWorker {
    /// The created worker topology entity.
    pub entity: Entity,
    /// The captured worker image.
    pub image: Arc<WorkerImage>,
}

impl PartialEq for Mutation {
    /// Compare mutations for replay validation.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            // runtime
            (
                Self::SpawnRuntime {
                    runtime_id,
                    runtime_entity,
                    runtime,
                    workers,
                },
                Self::SpawnRuntime {
                    runtime_id: other_runtime_id,
                    runtime_entity: other_runtime_entity,
                    runtime: other_runtime,
                    workers: other_workers,
                },
            ) => {
                runtime_id == other_runtime_id
                    && runtime_entity == other_runtime_entity
                    && runtime == other_runtime
                    && workers == other_workers
            }
            (Self::RemoveRuntime { runtime_id }, Self::RemoveRuntime { runtime_id: other }) => {
                runtime_id == other
            }
            (
                Self::SpawnWorker {
                    runtime_id,
                    worker_id,
                    worker_entity,
                    worker,
                },
                Self::SpawnWorker {
                    runtime_id: other_runtime_id,
                    worker_id: other_worker_id,
                    worker_entity: other_worker_entity,
                    worker: other_worker,
                },
            ) => {
                runtime_id == other_runtime_id
                    && worker_id == other_worker_id
                    && worker_entity == other_worker_entity
                    && worker == other_worker
            }
            (Self::RemoveWorker { worker_id }, Self::RemoveWorker { worker_id: other }) => {
                worker_id == other
            }

            // resources
            (
                Self::AddResource {
                    resource_id,
                    entity,
                },
                Self::AddResource {
                    resource_id: other_resource_id,
                    entity: other_entity,
                },
            ) => resource_id == other_resource_id && entity == other_entity,
            (Self::RemoveResource { resource_id }, Self::RemoveResource { resource_id: other }) => {
                resource_id == other
            }

            // policy
            (Self::SetPolicy { policy }, Self::SetPolicy { policy: other }) => policy == other,
            (Self::AddRule { rule }, Self::AddRule { rule: other }) => rule == other,
            (Self::RemoveRule { rule_id }, Self::RemoveRule { rule_id: other }) => rule_id == other,
            (Self::EnableRule { rule_id }, Self::EnableRule { rule_id: other }) => rule_id == other,
            (Self::DisableRule { rule_id }, Self::DisableRule { rule_id: other }) => {
                rule_id == other
            }
            (
                Self::ReplaceRule { rule_id, rule },
                Self::ReplaceRule {
                    rule_id: other_rule_id,
                    rule: other_rule,
                },
            ) => rule_id == other_rule_id && rule == other_rule,

            // debugger
            (Self::AddBreakpoint { breakpoint }, Self::AddBreakpoint { breakpoint: other }) => {
                breakpoint == other
            }
            (
                Self::UpdateBreakpoint { breakpoint },
                Self::UpdateBreakpoint { breakpoint: other },
            ) => breakpoint == other,
            (
                Self::RemoveBreakpoint { breakpoint_id },
                Self::RemoveBreakpoint {
                    breakpoint_id: other,
                },
            ) => breakpoint_id == other,
            (Self::AddWatchpoint { watchpoint }, Self::AddWatchpoint { watchpoint: other }) => {
                watchpoint == other
            }
            (
                Self::UpdateWatchpoint { watchpoint },
                Self::UpdateWatchpoint { watchpoint: other },
            ) => watchpoint == other,
            (
                Self::RemoveWatchpoint { watchpoint_id },
                Self::RemoveWatchpoint {
                    watchpoint_id: other,
                },
            ) => watchpoint_id == other,
            (Self::AddProbe { probe }, Self::AddProbe { probe: other }) => probe == other,
            (Self::UpdateProbe { probe }, Self::UpdateProbe { probe: other }) => probe == other,
            (Self::RemoveProbe { probe_id }, Self::RemoveProbe { probe_id: other }) => {
                probe_id == other
            }
            // topology
            (Self::DefineEntityKind { kind }, Self::DefineEntityKind { kind: other }) => {
                kind == other
            }
            (Self::UndefineEntityKind { kind }, Self::UndefineEntityKind { kind: other }) => {
                kind == other
            }
            (Self::DefineEdgeKind { kind }, Self::DefineEdgeKind { kind: other }) => kind == other,
            (Self::UndefineEdgeKind { kind }, Self::UndefineEdgeKind { kind: other }) => {
                kind == other
            }
            (Self::UpsertEntity { entity }, Self::UpsertEntity { entity: other }) => {
                entity == other
            }
            (Self::RemoveEntity { entity_id }, Self::RemoveEntity { entity_id: other }) => {
                entity_id == other
            }
            (Self::UpsertEdge { edge }, Self::UpsertEdge { edge: other }) => edge == other,
            (Self::RemoveEdge { edge_id }, Self::RemoveEdge { edge_id: other }) => edge_id == other,
            _ => false,
        }
    }
}

impl Mutation {
    /// Return the stable mutation name.
    pub fn name(&self) -> &'static str {
        match self {
            // runtime
            Self::SpawnRuntime { .. } => "runtime.instance.spawned",
            Self::RemoveRuntime { .. } => "runtime.instance.remove",
            Self::SpawnWorker { .. } => "runtime.worker.spawned",
            Self::RemoveWorker { .. } => "runtime.worker.remove",

            // resources
            Self::AddResource { .. } => "runtime.resource.add",
            Self::RemoveResource { .. } => "runtime.resource.remove",

            // policy
            Self::SetPolicy { .. } => "runtime.policy.set",
            Self::AddRule { .. } => "runtime.policy.rule.add",
            Self::RemoveRule { .. } => "runtime.policy.rule.remove",
            Self::EnableRule { .. } => "runtime.policy.rule.enable",
            Self::DisableRule { .. } => "runtime.policy.rule.disable",
            Self::ReplaceRule { .. } => "runtime.policy.rule.replace",

            // debugger
            Self::AddBreakpoint { .. } => "runtime.debug.breakpoint.add",
            Self::UpdateBreakpoint { .. } => "runtime.debug.breakpoint.update",
            Self::RemoveBreakpoint { .. } => "runtime.debug.breakpoint.remove",
            Self::AddWatchpoint { .. } => "runtime.debug.watchpoint.add",
            Self::UpdateWatchpoint { .. } => "runtime.debug.watchpoint.update",
            Self::RemoveWatchpoint { .. } => "runtime.debug.watchpoint.remove",
            Self::AddProbe { .. } => "runtime.debug.probe.add",
            Self::UpdateProbe { .. } => "runtime.debug.probe.update",
            Self::RemoveProbe { .. } => "runtime.debug.probe.remove",

            // topology
            Self::DefineEntityKind { .. } => "runtime.topology.entity.kind.define",
            Self::UndefineEntityKind { .. } => "runtime.topology.entity.kind.undefine",
            Self::DefineEdgeKind { .. } => "runtime.topology.edge.kind.define",
            Self::UndefineEdgeKind { .. } => "runtime.topology.edge.kind.undefine",
            Self::UpsertEntity { .. } => "runtime.topology.entity.upsert",
            Self::RemoveEntity { .. } => "runtime.topology.entity.remove",
            Self::UpsertEdge { .. } => "runtime.topology.edge.upsert",
            Self::RemoveEdge { .. } => "runtime.topology.edge.remove",
        }
    }

    /// Return the observation emitted after this mutation applies.
    pub fn observation(&self) -> Observation {
        match self {
            // runtime
            Self::SpawnRuntime {
                runtime_id,
                workers,
                ..
            } => Observation::RuntimeSpawned {
                runtime_id: *runtime_id,
                worker_count: workers.len(),
            },
            Self::RemoveRuntime { runtime_id } => Observation::RuntimeRemoved {
                runtime_id: *runtime_id,
            },
            Self::SpawnWorker {
                runtime_id,
                worker_id,
                ..
            } => Observation::WorkerSpawned {
                runtime_id: *runtime_id,
                worker_id: *worker_id,
            },
            Self::RemoveWorker { worker_id } => Observation::WorkerRemoved {
                worker_id: *worker_id,
            },

            // resources
            Self::AddResource { resource_id, .. } => Observation::ResourceAttached {
                worker_id: resource_id.worker_id,
                resource_id: *resource_id,
            },
            Self::RemoveResource { resource_id } => Observation::ResourceDetached {
                worker_id: resource_id.worker_id,
                resource_id: *resource_id,
            },

            // policy
            Self::SetPolicy { .. } => Observation::PolicyReplaced,
            Self::AddRule { rule } => Observation::RuleAdded {
                rule_id: rule.id.clone(),
            },
            Self::RemoveRule { rule_id } => Observation::RuleRemoved {
                rule_id: rule_id.clone(),
            },
            Self::EnableRule { rule_id } => Observation::RuleEnabled {
                rule_id: rule_id.clone(),
            },
            Self::DisableRule { rule_id } => Observation::RuleDisabled {
                rule_id: rule_id.clone(),
            },
            Self::ReplaceRule { rule_id, .. } => Observation::RuleReplaced {
                rule_id: rule_id.clone(),
            },

            // debugger
            Self::AddBreakpoint { breakpoint } => Observation::BreakpointAdded {
                breakpoint_id: breakpoint.id,
            },
            Self::UpdateBreakpoint { breakpoint } => Observation::BreakpointUpdated {
                breakpoint_id: breakpoint.id,
            },
            Self::RemoveBreakpoint { breakpoint_id } => Observation::BreakpointRemoved {
                breakpoint_id: *breakpoint_id,
            },
            Self::AddWatchpoint { watchpoint } => Observation::WatchpointAdded {
                watchpoint_id: watchpoint.id,
            },
            Self::UpdateWatchpoint { watchpoint } => Observation::WatchpointUpdated {
                watchpoint_id: watchpoint.id,
            },
            Self::RemoveWatchpoint { watchpoint_id } => Observation::WatchpointRemoved {
                watchpoint_id: *watchpoint_id,
            },
            Self::AddProbe { probe } => Observation::ProbeAdded { probe_id: probe.id },
            Self::UpdateProbe { probe } => Observation::ProbeUpdated { probe_id: probe.id },
            Self::RemoveProbe { probe_id } => Observation::ProbeRemoved {
                probe_id: *probe_id,
            },

            // topology
            Self::DefineEntityKind { kind } => Observation::EntityKindDefined {
                kind: kind.kind.clone(),
            },
            Self::UndefineEntityKind { kind } => {
                Observation::EntityKindRemoved { kind: kind.clone() }
            }
            Self::DefineEdgeKind { kind } => Observation::EdgeKindDefined {
                kind: kind.kind.clone(),
            },
            Self::UndefineEdgeKind { kind } => Observation::EdgeKindRemoved { kind: kind.clone() },
            Self::UpsertEntity { entity } => Observation::EntityUpserted {
                entity_id: entity.id.clone(),
            },
            Self::RemoveEntity { entity_id } => Observation::EntityRemoved {
                entity_id: entity_id.clone(),
            },
            Self::UpsertEdge { edge } => Observation::EdgeUpserted {
                edge_id: edge.id.clone(),
            },
            Self::RemoveEdge { edge_id } => Observation::EdgeRemoved {
                edge_id: edge_id.clone(),
            },
        }
    }
}

impl World {
    /// Apply one resolved world mutation without touching authoritative trace.
    pub(crate) fn apply_mutation(&mut self, mutation: Mutation) -> RuntimeResult<()> {
        self.apply_mutation_with_restore(mutation, RestoreContext::empty())
    }

    /// Apply one resolved world mutation with explicit restore context.
    pub(crate) fn apply_mutation_with_restore(
        &mut self,
        mutation: Mutation,
        restore: RestoreContext<'_>,
    ) -> RuntimeResult<()> {
        match mutation {
            // runtime lifecycle
            Mutation::SpawnRuntime {
                runtime_id,
                runtime_entity,
                runtime,
                workers,
            } => {
                self.restore_runtime_image(
                    runtime_id,
                    runtime_entity,
                    &runtime,
                    &workers,
                    restore,
                )?;
            }
            Mutation::RemoveRuntime { runtime_id } => {
                let _ = self.remove_stored_runtime(runtime_id)?;
            }

            // structural mutations
            Mutation::SpawnWorker {
                runtime_id,
                worker_id,
                worker_entity,
                worker,
            } => {
                self.restore_worker_image(runtime_id, worker_id, worker_entity, &worker, restore)?;
            }
            Mutation::RemoveWorker { worker_id } => {
                // live runtime worker
                if let Some(runtime_id) = self.runtime_id_for_worker(worker_id) {
                    let runtime = self.runtime_mut(runtime_id)?;

                    runtime.remove_worker(worker_id)?;
                    runtime.shared_collection.remove_worker(
                        &runtime.shared_heap,
                        &runtime.program,
                        worker_id,
                    );
                }
                // detached worker image
                else if self.state.topology.worker_subject(worker_id).is_none() {
                    return Err(RuntimeError::worker_not_found(worker_id.0).boxed());
                }

                self.remove_worker_topology(worker_id);
            }
            Mutation::AddResource {
                resource_id,
                entity,
            } => {
                self.state.attach_resource(resource_id, entity)?;
            }
            Mutation::RemoveResource { resource_id } => {
                self.state.detach_resource(resource_id);
            }
            Mutation::SetPolicy { policy } => {
                self.state.policy.replace(policy)?;
            }
            Mutation::AddRule { rule } => {
                self.state.policy.add_rule(rule)?;
            }
            Mutation::RemoveRule { rule_id } => {
                self.state.policy.remove_rule(&rule_id)?;
            }
            Mutation::EnableRule { rule_id } => {
                self.state.policy.enable_rule(&rule_id)?;
            }
            Mutation::DisableRule { rule_id } => {
                self.state.policy.disable_rule(&rule_id)?;
            }
            Mutation::ReplaceRule { rule_id, rule } => {
                self.state.policy.replace_rule(&rule_id, rule)?;
            }
            Mutation::AddBreakpoint { breakpoint } => {
                self.state.debugger.add_breakpoint(breakpoint);
            }
            Mutation::UpdateBreakpoint { breakpoint } => {
                if !self.state.debugger.update_breakpoint(breakpoint) {
                    return Err(Self::missing_debugger_entry("breakpoint"));
                }
            }
            Mutation::RemoveBreakpoint { breakpoint_id } => {
                if !self.state.debugger.remove_breakpoint(breakpoint_id) {
                    return Err(Self::missing_debugger_entry("breakpoint"));
                }
            }
            Mutation::AddWatchpoint { watchpoint } => {
                self.state.debugger.add_watchpoint(watchpoint);
            }
            Mutation::UpdateWatchpoint { watchpoint } => {
                if !self.state.debugger.update_watchpoint(watchpoint) {
                    return Err(Self::missing_debugger_entry("watchpoint"));
                }
            }
            Mutation::RemoveWatchpoint { watchpoint_id } => {
                if !self.state.debugger.remove_watchpoint(watchpoint_id) {
                    return Err(Self::missing_debugger_entry("watchpoint"));
                }
            }
            Mutation::AddProbe { probe } => {
                self.state.debugger.add_probe(probe);
            }
            Mutation::UpdateProbe { probe } => {
                if !self.state.debugger.update_probe(probe) {
                    return Err(Self::missing_debugger_entry("probe"));
                }
            }
            Mutation::RemoveProbe { probe_id } => {
                if !self.state.debugger.remove_probe(probe_id) {
                    return Err(Self::missing_debugger_entry("probe"));
                }
            }
            Mutation::DefineEntityKind { kind } => {
                self.state
                    .topology
                    .define_entity_kind(kind)
                    .map_err(Self::internal_error)?;
            }
            Mutation::UndefineEntityKind { kind } => {
                self.state
                    .topology
                    .undefine_entity_kind(&kind)
                    .map_err(Self::internal_error)?;
            }
            Mutation::DefineEdgeKind { kind } => {
                self.state
                    .topology
                    .define_edge_kind(kind)
                    .map_err(Self::internal_error)?;
            }
            Mutation::UndefineEdgeKind { kind } => {
                self.state
                    .topology
                    .undefine_edge_kind(&kind)
                    .map_err(Self::internal_error)?;
            }
            Mutation::UpsertEntity { entity } => {
                self.state
                    .topology
                    .upsert_entity(entity)
                    .map_err(Self::internal_error)?;
            }
            Mutation::RemoveEntity { entity_id } => {
                self.state.topology.remove_entity(entity_id.as_str());
            }
            Mutation::UpsertEdge { edge } => {
                self.state
                    .topology
                    .upsert_edge(edge)
                    .map_err(Self::internal_error)?;
            }
            Mutation::RemoveEdge { edge_id } => {
                self.state.topology.remove_edge(edge_id.as_str());
            }
        }

        Ok(())
    }

    /// Apply one world mutation through the authoritative input boundary.
    pub(crate) fn mutate(&mut self, mutation: Mutation) -> RuntimeResult<()> {
        // resolve the requested mutation first
        let mode = self.state.trace.mode();
        let mutation = self.resolve_mutation(mutation)?;

        // apply the mutation before appending it to trace
        self.apply_mutation(mutation.clone())?;

        // record the successful state transition in the visible timeline
        self.state.advance_moment()?;
        self.state.observe(mutation.observation())?;

        // append the authoritative input only after the mutation succeeds
        if mode == ExecutionMode::Record {
            self.record_mutation(mutation)?;
        }

        Ok(())
    }

    /// Remove one worker from the world.
    pub fn remove_worker(&mut self, worker_id: WorkerId) -> RuntimeResult<()> {
        self.mutate(Mutation::RemoveWorker { worker_id })
    }

    /// Add one world resource.
    pub fn add_resource(&mut self, resource_id: ResourceId, entity: Entity) -> RuntimeResult<()> {
        self.mutate(Mutation::AddResource {
            resource_id,
            entity,
        })
    }

    /// Remove one world resource.
    pub fn remove_resource(&mut self, resource_id: ResourceId) -> RuntimeResult<()> {
        self.mutate(Mutation::RemoveResource { resource_id })
    }

    /// Replace the active policy.
    pub fn set_policy(&mut self, policy: Policy) -> RuntimeResult<()> {
        self.mutate(Mutation::SetPolicy { policy })
    }

    /// Add one rule into the active policy.
    pub fn add_rule(&mut self, rule: Rule) -> RuntimeResult<()> {
        self.mutate(Mutation::AddRule { rule })
    }

    /// Add many rules into the active policy in declaration order.
    pub fn add_rules(&mut self, rules: impl IntoIterator<Item = Rule>) -> RuntimeResult<()> {
        for rule in rules {
            self.add_rule(rule)?;
        }

        Ok(())
    }

    /// Remove one rule from the active policy.
    pub fn remove_rule(&mut self, rule_id: RuleId) -> RuntimeResult<()> {
        self.mutate(Mutation::RemoveRule { rule_id })
    }

    /// Enable one rule in the active policy.
    pub fn enable_rule(&mut self, rule_id: RuleId) -> RuntimeResult<()> {
        self.mutate(Mutation::EnableRule { rule_id })
    }

    /// Disable one rule in the active policy.
    pub fn disable_rule(&mut self, rule_id: RuleId) -> RuntimeResult<()> {
        self.mutate(Mutation::DisableRule { rule_id })
    }

    /// Replace one active rule.
    pub fn replace_rule(&mut self, rule_id: RuleId, rule: Rule) -> RuntimeResult<()> {
        self.mutate(Mutation::ReplaceRule { rule_id, rule })
    }

    /// Add one debugger breakpoint.
    pub fn add_breakpoint(&mut self, filter: PointFilter) -> RuntimeResult<program::BreakpointId> {
        let breakpoint_id = self.state.debugger.allocate_breakpoint_id();
        let breakpoint = Breakpoint::new(breakpoint_id, filter);

        self.mutate(Mutation::AddBreakpoint { breakpoint })?;

        Ok(breakpoint_id)
    }

    /// Update one debugger breakpoint.
    pub fn update_breakpoint(&mut self, breakpoint: Breakpoint) -> RuntimeResult<()> {
        self.mutate(Mutation::UpdateBreakpoint { breakpoint })
    }

    /// Remove one debugger breakpoint.
    pub fn remove_breakpoint(&mut self, breakpoint_id: program::BreakpointId) -> RuntimeResult<()> {
        self.mutate(Mutation::RemoveBreakpoint { breakpoint_id })
    }

    /// Add one debugger watchpoint.
    pub fn add_watchpoint(&mut self, filter: MemoryFilter) -> RuntimeResult<program::WatchpointId> {
        let watchpoint_id = self.state.debugger.allocate_watchpoint_id();
        let watchpoint = Watchpoint::new(watchpoint_id, filter);

        self.mutate(Mutation::AddWatchpoint { watchpoint })?;

        Ok(watchpoint_id)
    }

    /// Update one debugger watchpoint.
    pub fn update_watchpoint(&mut self, watchpoint: Watchpoint) -> RuntimeResult<()> {
        self.mutate(Mutation::UpdateWatchpoint { watchpoint })
    }

    /// Remove one debugger watchpoint.
    pub fn remove_watchpoint(&mut self, watchpoint_id: program::WatchpointId) -> RuntimeResult<()> {
        self.mutate(Mutation::RemoveWatchpoint { watchpoint_id })
    }

    /// Add one debugger probe.
    pub fn add_probe(
        &mut self,
        filter: ProbeFilter,
        action: ProbeAction,
    ) -> RuntimeResult<ProbeId> {
        let probe_id = self.state.debugger.allocate_probe_id();
        let probe = Probe::new(probe_id, filter, action);

        self.mutate(Mutation::AddProbe { probe })?;

        Ok(probe_id)
    }

    /// Update one debugger probe.
    pub fn update_probe(&mut self, probe: Probe) -> RuntimeResult<()> {
        self.mutate(Mutation::UpdateProbe { probe })
    }

    /// Remove one debugger probe.
    pub fn remove_probe(&mut self, probe_id: ProbeId) -> RuntimeResult<()> {
        self.mutate(Mutation::RemoveProbe { probe_id })
    }

    /// Define one world entity kind.
    pub fn define_entity_kind(&mut self, kind: EntityDefinition) -> RuntimeResult<()> {
        self.mutate(Mutation::DefineEntityKind { kind })
    }

    /// Undefine one world entity kind.
    pub fn undefine_entity_kind(&mut self, kind: EntityKind) -> RuntimeResult<()> {
        self.mutate(Mutation::UndefineEntityKind { kind })
    }

    /// Define one world edge kind.
    pub fn define_edge_kind(&mut self, kind: EdgeDefinition) -> RuntimeResult<()> {
        self.mutate(Mutation::DefineEdgeKind { kind })
    }

    /// Undefine one world edge kind.
    pub fn undefine_edge_kind(&mut self, kind: EdgeKind) -> RuntimeResult<()> {
        self.mutate(Mutation::UndefineEdgeKind { kind })
    }

    /// Upsert one world entity.
    pub fn upsert_entity(&mut self, entity: Entity) -> RuntimeResult<()> {
        self.mutate(Mutation::UpsertEntity { entity })
    }

    /// Remove one world entity.
    pub fn remove_entity(&mut self, entity_id: EntityId) -> RuntimeResult<()> {
        self.mutate(Mutation::RemoveEntity { entity_id })
    }

    /// Upsert one world edge.
    pub fn upsert_edge(&mut self, edge: Edge) -> RuntimeResult<()> {
        self.mutate(Mutation::UpsertEdge { edge })
    }

    /// Remove one world edge.
    pub fn remove_edge(&mut self, edge_id: EdgeId) -> RuntimeResult<()> {
        self.mutate(Mutation::RemoveEdge { edge_id })
    }

    /// Build one internal runtime error.
    fn internal_error(message: impl fmt::Display) -> Box<RuntimeError> {
        RuntimeError::Internal {
            message: message.to_string(),
        }
        .boxed()
    }

    /// Build one missing debugger configuration error.
    fn missing_debugger_entry(kind: &str) -> Box<RuntimeError> {
        Self::internal_error(format!("{kind} not found"))
    }

    /// Return the runtime that owns one live worker.
    fn runtime_id_for_worker(&self, worker_id: WorkerId) -> Option<RuntimeId> {
        self.runtimes.iter().find_map(|(runtime_id, runtime)| {
            runtime.worker(worker_id).is_some().then_some(*runtime_id)
        })
    }

    /// Remove one worker topology entity.
    pub(super) fn remove_worker_topology(&mut self, worker_id: WorkerId) {
        self.state.topology.remove_worker(worker_id);
    }
}
