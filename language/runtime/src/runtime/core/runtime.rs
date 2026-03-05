use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::Host;
use crate::runtime::engine::{Engine, EngineOutput};
use crate::runtime::poller::HostPoller;
use crate::runtime::world::{RuntimeId, World};
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
    /// Shared world attached to every agent in this runtime.
    world: Arc<World>,
    /// Shared host integration for all agents in this runtime.
    host: Host,
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
            .field("runtime_id", &self.id)
            .field("name", &self.name)
            .field("platform_args", &self.platform_args)
            .field("options", &self.options)
            .field("world", &self.world)
            .field("host", &self.host)
            .field("agents", &self.agents)
            .field("primary_agent_id", &self.primary_agent_id)
            .field("poller", &"<shared platform poller>")
            .finish()
    }
}

impl Runtime {
    /// Create a runtime with one primary agent from explicit options.
    pub fn from_options(
        platform_args: impl Into<Arc<[String]>>,
        options: &RuntimeOptions,
    ) -> RuntimeResult<Self> {
        let platform_args = platform_args.into();
        let primary_agent = Agent::new(platform_args.clone(), options)?;
        let mut runtime = Self::new(platform_args, options, primary_agent)?;
        if let Some(poller) = poller_for_options(options)? {
            runtime.set_poller(poller);
        }

        Ok(runtime)
    }

    /// Create a runtime with one primary agent in one explicit shared world.
    pub fn from_options_in_world(
        platform_args: impl Into<Arc<[String]>>,
        options: &RuntimeOptions,
        world: impl Into<Arc<World>>,
    ) -> RuntimeResult<Self> {
        let platform_args = platform_args.into();
        let primary_agent = Agent::new_in_world(platform_args.clone(), options, world)?;
        let mut runtime = Self::new(platform_args, options, primary_agent)?;
        if let Some(poller) = poller_for_options(options)? {
            runtime.set_poller(poller);
        }

        Ok(runtime)
    }

    /// Return the shared world for this runtime.
    pub fn world(&self) -> &World {
        self.world.as_ref()
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
    pub fn spawn_agent(&mut self) -> RuntimeResult<AgentId> {
        self.spawn_agent_with_options(&self.options.clone())
    }

    /// Spawn one additional agent with explicit options in the shared runtime world.
    pub fn spawn_agent_with_options(&mut self, options: &RuntimeOptions) -> RuntimeResult<AgentId> {
        // force runtime identity to stay shared across all agents in this runtime
        let mut options = options.clone();
        options.name = self.options.name.clone();
        options.labels = self.options.labels.clone();
        self.align_spawn_options_with_runtime(&mut options);

        // create one new agent attached to the runtime world
        let agent = Agent::new_in_runtime(
            self.platform_args.clone(),
            &options,
            self.world.clone(),
            self.id,
        )?;

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
        let (agent, host, poller) = self.agent_and_poller_mut(agent_id)?;
        agent.run_entrypoint_with_host_and_poller(host, engine, entry, args, poller)
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
    ) -> RuntimeResult<Self> {
        // seed runtime identity from runtime options
        let world = primary_agent.world.clone();
        let host = Host::from_runtime_options(options);
        let primary_agent = Box::new(primary_agent);
        let primary_agent_id = primary_agent.id;
        let runtime_id = primary_agent.runtime_id;
        let runtime_name = world.runtime_name(runtime_id)?;
        let mut agents = BTreeMap::new();
        agents.insert(primary_agent_id, primary_agent);

        // store runtime state
        Ok(Self {
            id: runtime_id,
            name: runtime_name,
            platform_args,
            options: options.clone(),
            world,
            host,
            poller: None,
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

    /// Return mutable references to one agent and the shared runtime poller.
    #[allow(clippy::type_complexity)]
    fn agent_and_poller_mut(
        &mut self,
        agent_id: AgentId,
    ) -> RuntimeResult<(&mut Agent, &Host, &mut Option<Box<dyn HostPoller>>)> {
        // resolve the agent before returning shared runtime references
        let host = &self.host;
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

        Ok((agent, host, poller))
    }
}
