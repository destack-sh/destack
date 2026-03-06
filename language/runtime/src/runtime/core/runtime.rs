use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::Host;
use crate::runtime::engine::{Engine, EngineOutput, Entry};
use crate::runtime::poller::HostPoller;
use crate::runtime::scheduler::Timer;
use crate::runtime::time::WorldInstant;
use crate::runtime::world::{Ingress, RuntimeId, Wake, World};
use crate::runtime::{DropCounts, DropReason};
use destack_heap as heap;
use destack_workspace::RuntimeOptions;
use std::collections::BTreeMap;
use std::sync::Arc;

use super::poller::poller_for_options;
use super::{Agent, AgentId};

/// Runtime container that owns one or more agents in one shared world.
pub struct Runtime {
    /// Runtime identifier in world topology.
    id: RuntimeId,
    /// Runtime name used for identity selection and diagnostics.
    name: String,
    /// Immutable process arguments shared by newly spawned agents.
    platform_args: Arc<[String]>,
    /// Runtime options used for agent creation.
    options: RuntimeOptions,
    /// Shared host integration for all agents in this runtime.
    host: Host,
    /// Shared platform poller for external events.
    poller: Option<Box<dyn HostPoller>>,
    /// Execution engine owned by this runtime.
    engine: Box<dyn Engine>,
    /// Drop accounting at the runtime coordination boundary.
    drop_counts: DropCounts,
    /// All active agents keyed by identifier.
    agents: BTreeMap<AgentId, Box<Agent>>,
    /// Default agent used by convenience accessors.
    primary_agent_id: AgentId,
}

/// Result of one runtime scheduler tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickOutcome {
    /// One deterministic unit of work or ingress handling completed.
    Progressed,
    /// Virtual time advanced to the next deadline.
    AdvancedTime,
    /// No runnable work or future deadlines remained.
    Idle,
}

impl TickOutcome {
    /// Return whether this tick made deterministic progress.
    pub const fn progressed(self) -> bool {
        !matches!(self, Self::Idle)
    }
}

impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("runtime_id", &self.id)
            .field("name", &self.name)
            .field("platform_args", &self.platform_args)
            .field("options", &self.options)
            .field("host", &self.host)
            .field("agents", &self.agents)
            .field("primary_agent_id", &self.primary_agent_id)
            .field("poller", &"<shared platform poller>")
            .finish()
    }
}

impl Runtime {
    /// Create a runtime with one primary agent in one explicit shared world.
    pub(crate) fn from_options_in_world(
        platform_args: impl Into<Arc<[String]>>,
        options: &RuntimeOptions,
        world: &World,
        engine: Box<dyn Engine>,
    ) -> RuntimeResult<Self> {
        let platform_args = platform_args.into();
        let primary_agent = Agent::new_in_world(platform_args.clone(), options, world)?;
        let mut runtime = Self::new(platform_args, options, primary_agent, engine)?;
        if let Some(poller) = poller_for_options(options)? {
            runtime.set_poller(poller);
        }

        Ok(runtime)
    }

    /// Return the shared host integration for this runtime.
    pub fn host(&self) -> &Host {
        &self.host
    }

    /// Return the callback runtime id for native host callback routing.
    pub fn host_callback_runtime_id(&self) -> Option<u64> {
        self.host.callback_runtime_id()
    }

    /// Return the current primary agent id.
    pub fn primary_agent_id(&self) -> AgentId {
        self.primary_agent_id
    }

    /// Return the world topology runtime id.
    pub fn runtime_id(&self) -> RuntimeId {
        self.id
    }

    /// Return this runtime name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return drop accounting observed by this runtime coordinator.
    pub const fn drop_counts(&self) -> DropCounts {
        self.drop_counts
    }

    /// Set one explicit primary agent.
    pub fn set_primary_agent(&mut self, agent_id: AgentId) -> RuntimeResult<()> {
        if self.agents.contains_key(&agent_id) {
            self.primary_agent_id = agent_id;
            return Ok(());
        }

        Err(RuntimeError::Internal {
            message: format!("runtime agent {} does not exist", agent_id.0),
        }
        .boxed())
    }

    /// Return all active agent ids.
    pub fn agent_ids(&self) -> Vec<AgentId> {
        self.agents.keys().copied().collect()
    }

    /// Return one immutable agent by id.
    pub fn agent(&self, agent_id: AgentId) -> Option<&Agent> {
        self.agents.get(&agent_id).map(Box::as_ref)
    }

    /// Return one mutable agent by id.
    pub fn agent_mut(&mut self, agent_id: AgentId) -> Option<&mut Agent> {
        self.agents.get_mut(&agent_id).map(Box::as_mut)
    }

    /// Spawn one additional agent in the shared runtime world.
    pub fn spawn_agent(&mut self, world: &World) -> RuntimeResult<AgentId> {
        self.spawn_agent_with_options(world, &self.options.clone())
    }

    /// Spawn one additional agent with explicit options in the shared runtime world.
    pub fn spawn_agent_with_options(
        &mut self,
        world: &World,
        options: &RuntimeOptions,
    ) -> RuntimeResult<AgentId> {
        // force runtime identity to stay shared across all agents in this runtime
        let mut options = options.clone();
        options.name = self.options.name.clone();
        options.labels = self.options.labels.clone();
        self.align_spawn_options_with_runtime(&mut options);

        // create one new agent attached to the runtime world
        let agent = Agent::new_in_runtime(self.platform_args.clone(), &options, world, self.id)?;

        Ok(self.insert_agent(agent))
    }

    /// Attach a shared platform poller for all agents in this runtime.
    pub fn set_poller(&mut self, poller: Box<dyn HostPoller>) {
        self.poller = Some(poller);
    }

    /// Run one entrypoint through the default runtime agent event loop.
    pub(crate) fn run_entrypoint(
        &mut self,
        world: &World,
        entry: &Entry,
        args: &[heap::Value],
    ) -> RuntimeResult<EngineOutput> {
        self.run_entrypoint_for_agent(world, self.primary_agent_id, entry, args)
    }

    /// Run one entrypoint through one explicit runtime agent event loop.
    pub(crate) fn run_entrypoint_for_agent(
        &mut self,
        world: &World,
        agent_id: AgentId,
        entry: &Entry,
        args: &[heap::Value],
    ) -> RuntimeResult<EngineOutput> {
        let host = &self.host;
        let poller = &mut self.poller;
        let engine = self.engine.as_mut();
        let agent = self
            .agents
            .get_mut(&agent_id)
            .map(Box::as_mut)
            .ok_or_else(|| {
                RuntimeError::Internal {
                    message: format!("runtime agent {} does not exist", agent_id.0),
                }
                .boxed()
            })?;
        agent.run_entrypoint_with_host_and_poller(world, host, engine, entry, args, poller)
    }

    /// Execute one runtime tick across all agents without advancing world time.
    pub(crate) fn tick(&mut self, world: &World) -> RuntimeResult<TickOutcome> {
        // poll and handle runtime ingress first
        let ingress_handled = self.poll_ingress(world)?;

        // run one local agent tick in stable id order
        let agent_ids = self.agent_ids();
        let host = &self.host;
        let engine = self.engine.as_mut();
        let agents = &mut self.agents;
        for agent_id in agent_ids {
            let agent = agents.get_mut(&agent_id).map(Box::as_mut).ok_or_else(|| {
                RuntimeError::Internal {
                    message: format!("runtime agent {} does not exist", agent_id.0),
                }
                .boxed()
            })?;
            if agent.tick(world, host, engine)? {
                return Ok(TickOutcome::Progressed);
            }
        }

        // ingress handling counts as runnable scheduler progress
        if ingress_handled {
            return Ok(TickOutcome::Progressed);
        }

        Ok(TickOutcome::Idle)
    }

    /// Remove one agent from this runtime and return its boxed handle.
    pub fn remove_agent(&mut self, agent_id: AgentId) -> RuntimeResult<Box<Agent>> {
        // remove the target agent from the registry
        let removed_agent = self.agents.remove(&agent_id).ok_or_else(|| {
            RuntimeError::Internal {
                message: format!("runtime agent {} does not exist", agent_id.0),
            }
            .boxed()
        })?;

        // reject removing the last remaining agent
        if self.agents.is_empty() {
            self.agents.insert(agent_id, removed_agent);
            return Err(RuntimeError::Internal {
                message: "runtime must keep at least one agent".to_string(),
            }
            .boxed());
        }

        // reject implicit primary fallback to keep ownership explicit
        if self.primary_agent_id == agent_id {
            self.agents.insert(agent_id, removed_agent);
            return Err(RuntimeError::Internal {
                message: "cannot remove primary agent: set a new primary agent first".to_string(),
            }
            .boxed());
        }

        Ok(removed_agent)
    }

    /// Create one runtime from one already-constructed primary agent.
    fn new(
        platform_args: Arc<[String]>,
        options: &RuntimeOptions,
        primary_agent: Agent,
        engine: Box<dyn Engine>,
    ) -> RuntimeResult<Self> {
        // seed runtime identity from runtime options
        let host = Host::from_runtime_options(options);
        let primary_agent = Box::new(primary_agent);
        let primary_agent_id = primary_agent.id;
        let runtime_id = primary_agent.runtime_id;
        let runtime_name = options
            .name
            .clone()
            .unwrap_or_else(|| "runtime".to_string());
        let mut agents = BTreeMap::new();
        agents.insert(primary_agent_id, primary_agent);

        // store runtime state
        Ok(Self {
            id: runtime_id,
            name: runtime_name,
            platform_args,
            options: options.clone(),
            host,
            poller: None,
            engine,
            drop_counts: DropCounts::default(),
            agents,
            primary_agent_id,
        })
    }

    /// Insert one agent and return its id.
    fn insert_agent(&mut self, agent: Agent) -> AgentId {
        // derive one stable id from the underlying runtime context
        let agent = Box::new(agent);
        let agent_id = agent.id;

        // insert or replace one slot keyed by id
        let _ = self.agents.insert(agent_id, agent);

        agent_id
    }

    /// Return the next virtual deadline across all agents and simulation.
    pub(crate) fn next_deadline(&self, world: &World) -> Option<WorldInstant> {
        // current virtual timestamps: monotonic deadlines are projected onto wall time
        let wall_now = world.wall();
        let mono_now = world.mono();

        world.next_deadline(
            self.agents
                .values()
                .map(|agent| agent.event_loop.next_deadline(wall_now, mono_now)),
        )
    }

    /// Drain due agent timers after the world advances time.
    pub(crate) fn collect_due_timers(
        &mut self,
        world: &World,
    ) -> RuntimeResult<Vec<(RuntimeId, AgentId, Timer)>> {
        let wall_now = world.wall();
        let mono_now = world.mono();
        let mut agent_timers = Vec::new();

        for agent in self.agents.values_mut() {
            let ready_timers = agent.event_loop.poll_timers(wall_now, mono_now)?;
            for timer in ready_timers {
                agent_timers.push((self.id, agent.id, timer));
            }
        }

        Ok(agent_timers)
    }

    /// Poll runtime-owned ingress sources and deliver arrivals to agent event loops.
    fn poll_ingress(&mut self, world: &World) -> RuntimeResult<bool> {
        let mut ingress = Vec::new();

        // host semantic ingress
        let poll_result = self.host.poll_events(Some(0))?;
        if poll_result.dropped_event_count > 0 {
            self.drop_counts
                .record(DropReason::QueuePressure, poll_result.dropped_event_count);
        }
        for event in poll_result.events {
            ingress.push(Ingress::Host {
                runtime_id: self.id,
                event,
            });
        }

        // poller ingress
        if let Some(poller) = self.poller.as_mut() {
            let events = poller.poll(Some(0))?;
            for event in events {
                ingress.push(Ingress::Poller {
                    runtime_id: self.id,
                    event,
                });
            }
        }

        self.deliver_ingress(world, ingress)
    }

    /// Deliver coordinator-owned ingress into agent event loops.
    fn deliver_ingress(&mut self, world: &World, ingress: Vec<Ingress>) -> RuntimeResult<bool> {
        let mut handled_any = false;

        for item in ingress {
            match item {
                Ingress::Host { runtime_id, event } => {
                    if runtime_id != self.id {
                        continue;
                    }

                    let targets = if let Some(kind) = event.kind() {
                        self.agent_ids()
                            .into_iter()
                            .filter(|agent_id| {
                                self.agent(*agent_id)
                                    .map(|agent| agent.watches_host_event(kind))
                                    .unwrap_or(false)
                            })
                            .collect::<Vec<_>>()
                    } else {
                        Vec::new()
                    };

                    // explicit unmatched ingress
                    if targets.is_empty() {
                        self.drop_counts.record(DropReason::UnmatchedIngress, 1);
                        handled_any = true;
                        continue;
                    }

                    // matched host ingress
                    for agent_id in targets {
                        let agent = self.agent_mut(agent_id).ok_or_else(|| {
                            RuntimeError::Internal {
                                message: format!("runtime agent {} does not exist", agent_id.0),
                            }
                            .boxed()
                        })?;
                        agent.event_loop.enqueue_host_events(vec![event.clone()]);
                        agent.hooks.on_ingress_enqueue(world);
                        handled_any = true;
                    }
                }
                Ingress::Poller { runtime_id, event } => {
                    if runtime_id != self.id {
                        continue;
                    }

                    let targets = self
                        .agent_ids()
                        .into_iter()
                        .filter(|agent_id| {
                            self.agent(*agent_id)
                                .map(|agent| agent.watches_event(event.token))
                                .unwrap_or(false)
                        })
                        .collect::<Vec<_>>();

                    // explicit unmatched ingress
                    if targets.is_empty() {
                        self.drop_counts.record(DropReason::UnmatchedIngress, 1);
                        handled_any = true;
                        continue;
                    }

                    // matched poller ingress
                    for agent_id in targets {
                        let agent = self.agent_mut(agent_id).ok_or_else(|| {
                            RuntimeError::Internal {
                                message: format!("runtime agent {} does not exist", agent_id.0),
                            }
                            .boxed()
                        })?;
                        agent.event_loop.enqueue_events(vec![event]);
                        agent.hooks.on_ingress_enqueue(world);
                        handled_any = true;
                    }
                }
            }
        }

        Ok(handled_any)
    }

    /// Deliver one batch of due agent-timer wakes.
    pub(crate) fn deliver_wakes(&mut self, world: &World, wakes: Vec<Wake>) -> RuntimeResult<()> {
        for wake in wakes {
            match wake {
                Wake::AgentTimer {
                    runtime_id,
                    agent_id,
                    timer,
                } => {
                    if runtime_id != self.id {
                        continue;
                    }

                    let agent = self.agent_mut(agent_id).ok_or_else(|| {
                        RuntimeError::Internal {
                            message: format!("runtime agent {} does not exist", agent_id.0),
                        }
                        .boxed()
                    })?;
                    agent.deliver_timer_wake(world, timer)?;
                }
            }
        }

        Ok(())
    }

    /// Align world-scoped options for agents spawned in one existing runtime.
    fn align_spawn_options_with_runtime(&self, options: &mut RuntimeOptions) {
        // world scoped settings: all agents in one runtime share one world
        options.execution = self.options.execution;
        options.world = self.options.world;
        options.access = self.options.access;
        options.replay = self.options.replay.clone();
        options.time = self.options.time.clone();
        options.random = self.options.random.clone();
        options.rules = self.options.rules.clone();
    }

    /// Return one stored engine with one explicit concrete type.
    #[cfg(test)]
    pub(crate) fn engine<T: Engine>(&self) -> Option<&T> {
        let engine = self.engine.as_ref() as &dyn std::any::Any;
        engine.downcast_ref::<T>()
    }
}
