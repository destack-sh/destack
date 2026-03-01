use std::collections::BTreeMap;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::Host;
use crate::platform::PlatformContext;
use crate::runtime::bindings::BindingRegistry;
use crate::runtime::engine::{Engine, RuntimeOutput, RuntimeValue};
use crate::runtime::poller::HostPoller;
use crate::runtime::scheduler::TaskId;
use crate::runtime::world::World;
use destack_workspace::RuntimeOptions;

use super::Agent;
use super::poller::poller_for_options;

/// Stable identifier for one runtime-managed agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AgentId(pub u64);

/// Runtime container that owns one or more agents in one shared world.
pub struct Runtime {
    /// Platform context shared by newly spawned agents.
    platform: PlatformContext,
    /// Runtime options used for agent creation.
    options: RuntimeOptions,
    /// Shared world attached to every agent in this runtime.
    world: World,
    /// Shared platform poller for external events.
    poller: Option<Box<dyn HostPoller>>,
    /// All active agents keyed by identifier.
    agents: BTreeMap<AgentId, Agent>,
    /// Default agent used by convenience accessors.
    primary_agent_id: AgentId,
}

impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("platform", &self.platform)
            .field("options", &self.options)
            .field("world", &self.world)
            .field("agents", &self.agents)
            .field("primary_agent_id", &self.primary_agent_id)
            .field("poller", &"<shared platform poller>")
            .finish()
    }
}

impl Runtime {
    /// Create a runtime with one primary agent from explicit options.
    pub fn from_options(
        platform: PlatformContext,
        options: &RuntimeOptions,
    ) -> RuntimeResult<Self> {
        let primary_agent = Agent::from_options(platform.clone(), options)?;
        let mut runtime = Self::new(platform, options, primary_agent);
        if let Some(poller) = poller_for_options(options)? {
            runtime.set_poller(poller);
        }

        Ok(runtime)
    }

    /// Create a runtime with one primary agent in one explicit shared world.
    pub fn from_options_in_world(
        platform: PlatformContext,
        options: &RuntimeOptions,
        world: World,
    ) -> RuntimeResult<Self> {
        let primary_agent = Agent::from_options_in_world(platform.clone(), options, world)?;
        let mut runtime = Self::new(platform, options, primary_agent);
        if let Some(poller) = poller_for_options(options)? {
            runtime.set_poller(poller);
        }

        Ok(runtime)
    }

    /// Return the shared world for this runtime.
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Return the current primary agent id.
    pub fn primary_agent_id(&self) -> AgentId {
        self.primary_agent_id
    }

    /// Return all active agent ids.
    pub fn agent_ids(&self) -> Vec<AgentId> {
        self.agents.keys().copied().collect()
    }

    /// Return one immutable agent by id.
    pub fn agent(&self, agent_id: AgentId) -> Option<&Agent> {
        self.agents.get(&agent_id)
    }

    /// Return one mutable agent by id.
    pub fn agent_mut(&mut self, agent_id: AgentId) -> Option<&mut Agent> {
        self.agents.get_mut(&agent_id)
    }

    /// Return one immutable reference to the primary agent.
    pub fn primary_agent(&self) -> &Agent {
        self.agents.get(&self.primary_agent_id).unwrap_or_else(|| {
            panic!(
                "runtime primary agent {} is missing",
                self.primary_agent_id.0
            )
        })
    }

    /// Return one mutable reference to the primary agent.
    pub fn primary_agent_mut(&mut self) -> &mut Agent {
        self.agents
            .get_mut(&self.primary_agent_id)
            .unwrap_or_else(|| {
                panic!(
                    "runtime primary agent {} is missing",
                    self.primary_agent_id.0
                )
            })
    }

    /// Spawn one additional agent in the shared runtime world.
    pub fn spawn_agent(&mut self) -> RuntimeResult<AgentId> {
        self.spawn_agent_with_options(&self.options.clone())
    }

    /// Spawn one additional agent with explicit options in the shared runtime world.
    pub fn spawn_agent_with_options(&mut self, options: &RuntimeOptions) -> RuntimeResult<AgentId> {
        // create one new agent attached to the runtime world
        let agent =
            Agent::from_options_in_world(self.platform.clone(), options, self.world.clone())?;

        Ok(self.insert_agent(agent))
    }

    /// Attach a shared platform poller for all agents in this runtime.
    pub fn set_poller(&mut self, poller: Box<dyn HostPoller>) {
        self.poller = Some(poller);
    }

    /// Borrow host integration from the primary agent.
    pub fn host(&self) -> &Host {
        self.primary_agent().host()
    }

    /// Borrow binding registry from the primary agent.
    pub fn bindings(&self) -> &BindingRegistry {
        &self.primary_agent().bindings
    }

    /// Borrow mutable binding registry from the primary agent.
    pub fn bindings_mut(&mut self) -> &mut BindingRegistry {
        &mut self.primary_agent_mut().bindings
    }

    /// Return dropped dispatch-event count from the primary agent.
    pub fn dropped_dispatch_events(&self) -> u64 {
        self.primary_agent().dropped_dispatch_events()
    }

    /// Return dropped host-queue event count from the primary agent.
    pub fn dropped_host_queue_events(&self) -> u64 {
        self.primary_agent().dropped_host_queue_events()
    }

    /// Return dropped unwatched dispatch-event count from the primary agent.
    pub fn dropped_unwatched_dispatch_events(&self) -> u64 {
        self.primary_agent().dropped_unwatched_dispatch_events()
    }

    /// Return the callback agent id used by native host callback routing.
    pub fn host_callback_agent_id(&self) -> Option<u64> {
        self.primary_agent().host_callback_agent_id()
    }

    /// Run one entrypoint through the primary agent event loop.
    pub fn run_entrypoint<E: Engine<Output = RuntimeOutput, Value = RuntimeValue>>(
        &mut self,
        engine: &mut E,
        entry: &E::Entry,
        args: &[RuntimeValue],
    ) -> RuntimeResult<RuntimeOutput> {
        let (agent, poller) = self.primary_agent_and_poller_mut();
        agent.run_entrypoint_with_poller(engine, entry, args, poller)
    }

    /// Tick the primary agent once.
    pub fn tick_once<E: Engine<Output = RuntimeOutput, Value = RuntimeValue>>(
        &mut self,
        engine: &mut E,
    ) -> RuntimeResult<bool> {
        let (agent, poller) = self.primary_agent_and_poller_mut();
        agent.tick_once_with_poller(engine, poller)
    }

    /// Run the primary agent until one task completes.
    pub fn run_loop_until_task_complete<E: Engine<Output = RuntimeOutput, Value = RuntimeValue>>(
        &mut self,
        engine: &mut E,
        target_task: TaskId,
    ) -> RuntimeResult<RuntimeOutput> {
        let (agent, poller) = self.primary_agent_and_poller_mut();
        agent.run_loop_until_task_complete_with_poller(engine, target_task, poller)
    }

    /// Remove one agent from this runtime.
    pub fn remove_agent(&mut self, agent_id: AgentId) -> RuntimeResult<Agent> {
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

        // rebind primary agent if the removed agent was primary
        if self.primary_agent_id == agent_id {
            let next_primary_agent_id = self
                .agents
                .keys()
                .copied()
                .next()
                .unwrap_or_else(|| panic!("runtime agent registry became empty unexpectedly"));
            self.primary_agent_id = next_primary_agent_id;
        }

        Ok(removed_agent)
    }

    // constructors

    /// Create one runtime from one already-constructed primary agent.
    fn new(platform: PlatformContext, options: &RuntimeOptions, primary_agent: Agent) -> Self {
        // seed runtime identity from the primary agent
        let world = primary_agent.world().clone();
        let primary_agent_id = AgentId(primary_agent.state.agent_id);
        let mut agents = BTreeMap::new();
        agents.insert(primary_agent_id, primary_agent);

        // store runtime state
        Self {
            platform,
            options: options.clone(),
            world,
            poller: None,
            agents,
            primary_agent_id,
        }
    }

    /// Insert one agent and return its id.
    fn insert_agent(&mut self, agent: Agent) -> AgentId {
        // derive one stable id from the underlying runtime context
        let agent_id = AgentId(agent.state.agent_id);

        // insert or replace one slot keyed by id
        let _ = self.agents.insert(agent_id, agent);

        agent_id
    }

    /// Return mutable references to the primary agent and shared runtime poller.
    fn primary_agent_and_poller_mut(&mut self) -> (&mut Agent, &mut Option<Box<dyn HostPoller>>) {
        let primary_agent_id = self.primary_agent_id;
        let poller = &mut self.poller;
        let agent = self
            .agents
            .get_mut(&primary_agent_id)
            .unwrap_or_else(|| panic!("runtime primary agent {} is missing", primary_agent_id.0));

        (agent, poller)
    }
}
