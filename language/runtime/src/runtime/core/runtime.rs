use std::collections::BTreeMap;
use std::sync::Arc;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformContext;
use crate::runtime::engine::{Engine, EngineOutput};
use crate::runtime::poller::HostPoller;
use crate::runtime::world::World;
use destack_heap as heap;
use destack_workspace::RuntimeOptions;

use super::poller::poller_for_options;
use super::{Agent, AgentId};

/// Runtime container that owns one or more agents in one shared world.
pub struct Runtime {
    /// Stable runtime name for selector matching.
    name: String,
    /// Runtime labels for selector matching.
    labels: BTreeMap<String, String>,
    /// Platform context shared by newly spawned agents.
    platform: PlatformContext,
    /// Runtime options used for agent creation.
    options: RuntimeOptions,
    /// Shared world attached to every agent in this runtime.
    world: Arc<World>,
    /// Shared platform poller for external events.
    poller: Option<Box<dyn HostPoller>>,
    /// All active agents keyed by identifier.
    agents: BTreeMap<AgentId, Box<Agent>>,
    /// Default agent used by convenience accessors.
    primary_agent_id: AgentId,
}

impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("name", &self.name)
            .field("labels", &self.labels)
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
        world: Arc<World>,
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
        self.world.as_ref()
    }

    /// Return the stable runtime name.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Return runtime labels.
    pub fn labels(&self) -> &BTreeMap<String, String> {
        &self.labels
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
        self.agents.get(&agent_id).map(Box::as_ref)
    }

    /// Return one mutable agent by id.
    pub fn agent_mut(&mut self, agent_id: AgentId) -> Option<&mut Agent> {
        self.agents.get_mut(&agent_id).map(Box::as_mut)
    }

    /// Spawn one additional agent in the shared runtime world.
    pub fn spawn_agent(&mut self) -> RuntimeResult<AgentId> {
        self.spawn_agent_with_options(&self.options.clone())
    }

    /// Spawn one additional agent with explicit options in the shared runtime world.
    pub fn spawn_agent_with_options(&mut self, options: &RuntimeOptions) -> RuntimeResult<AgentId> {
        // force runtime identity to stay shared across all agents in this runtime
        let mut options = options.clone();
        options.name = Some(self.name.clone());
        options.labels = self.labels.clone();

        // create one new agent attached to the runtime world
        let agent =
            Agent::from_options_in_world(self.platform.clone(), &options, self.world.clone())?;

        Ok(self.insert_agent(agent))
    }

    /// Attach a shared platform poller for all agents in this runtime.
    pub fn set_poller(&mut self, poller: Box<dyn HostPoller>) {
        self.poller = Some(poller);
    }

    /// Run one entrypoint through the default runtime agent event loop.
    pub fn run_entrypoint<E: Engine>(
        &mut self,
        engine: &mut E,
        entry: &E::Entry,
        args: &[heap::Value],
    ) -> RuntimeResult<EngineOutput> {
        self.run_entrypoint_for_agent(self.primary_agent_id, engine, entry, args)
    }

    /// Run one entrypoint through one explicit runtime agent event loop.
    pub fn run_entrypoint_for_agent<E: Engine>(
        &mut self,
        agent_id: AgentId,
        engine: &mut E,
        entry: &E::Entry,
        args: &[heap::Value],
    ) -> RuntimeResult<EngineOutput> {
        let (agent, poller) = self.agent_and_poller_mut(agent_id)?;
        agent.run_entrypoint_with_poller(engine, entry, args, poller)
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

    /// Create one runtime from one already-constructed primary agent.
    fn new(platform: PlatformContext, options: &RuntimeOptions, primary_agent: Agent) -> Self {
        // seed runtime identity from runtime options
        let world = primary_agent.world.clone();
        let name = options
            .name
            .clone()
            .unwrap_or_else(|| "runtime".to_string());
        let labels = options.labels.clone();
        let primary_agent = Box::new(primary_agent);
        let primary_agent_id = primary_agent.id;
        let mut agents = BTreeMap::new();
        agents.insert(primary_agent_id, primary_agent);

        // store runtime state
        Self {
            name,
            labels,
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
        let agent = Box::new(agent);
        let agent_id = agent.id;

        // insert or replace one slot keyed by id
        let _ = self.agents.insert(agent_id, agent);

        agent_id
    }

    /// Return mutable references to one agent and the shared runtime poller.
    fn agent_and_poller_mut(
        &mut self,
        agent_id: AgentId,
    ) -> RuntimeResult<(&mut Agent, &mut Option<Box<dyn HostPoller>>)> {
        // resolve the agent before returning shared runtime references
        let poller = &mut self.poller;
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

        Ok((agent, poller))
    }
}
