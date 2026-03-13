use std::collections::BTreeMap;
use std::sync::Arc;

use destack_core::CaptureMode;
use destack_heap as heap;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::HostEvent;
use crate::runtime::engine::{Engine, EngineOutput, Entry, EntryReference};
use crate::runtime::poller::PollerEvent;
use crate::runtime::trace::Outcome;
use crate::runtime::{Agent, AgentId, AgentImage, Runtime, RuntimeImage};
use destack_workspace::RuntimeOptions;

use super::{Input, RebindContext, RuntimeId, World};

/// Coordinator-visible arrived work that is not caused by world time advancing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RuntimeIngress {
    /// Runtime-wide host semantic arrival.
    Host {
        /// Runtime that owns the host integration.
        runtime_id: RuntimeId,
        /// Host event payload.
        event: HostEvent,
    },
    /// Runtime-wide poller arrival.
    Poller {
        /// Runtime that owns the shared poller.
        runtime_id: RuntimeId,
        /// Poller event payload.
        event: PollerEvent,
    },
}

impl World {
    /// Spawn one live runtime owned by this world and return its identifier.
    pub fn spawn_runtime(
        &self,
        platform_args: impl Into<Arc<[String]>>,
        options: &RuntimeOptions,
        engine: impl Engine + 'static,
    ) -> RuntimeResult<RuntimeId> {
        let _activity = self.enter_activity()?;
        let mode = self.trace.mode();
        let mut runtime =
            Runtime::from_options_in_world(platform_args, options, self, Box::new(engine))?;
        let runtime_id = runtime.runtime_id();

        // fast and deterministic modes do not need one structural spawn image
        let replay_image = if mode == destack_workspace::ExecutionMode::Record {
            Some(runtime.capture_image(CaptureMode::Suspend)?)
        } else {
            None
        };

        let mut runtimes = self.runtimes.write();
        if runtimes.insert(runtime_id, Box::new(runtime)).is_some() {
            return Err(RuntimeError::RuntimeAlreadyExists {
                runtime_id: runtime_id.0,
            }
            .boxed());
        }

        // record mode needs one structural spawn record for suffix replay
        if let Some((runtime, agents)) = replay_image {
            self.accept(Outcome::RuntimeSpawned { runtime, agents })?;
        }

        Ok(runtime_id)
    }

    /// Remove one stored runtime and all of its agents.
    pub fn remove_runtime(&self, runtime_id: RuntimeId) -> RuntimeResult<Box<Runtime>> {
        let input = Input::RemoveRuntime { runtime_id };
        let input = self.resolve_input(input)?;

        if self.trace.mode() == destack_workspace::ExecutionMode::Record {
            self.ingest(input.clone())?;
        }

        self.remove_runtime_inner(runtime_id)
    }

    /// Spawn one additional agent in one stored runtime.
    pub fn spawn_agent(
        &self,
        runtime_id: RuntimeId,
        engine: impl Engine + 'static,
    ) -> RuntimeResult<AgentId> {
        self.spawn_agent_with_options(runtime_id, &RuntimeOptions::default(), engine)
    }

    /// Spawn one additional agent with explicit options in one stored runtime.
    pub fn spawn_agent_with_options(
        &self,
        runtime_id: RuntimeId,
        options: &RuntimeOptions,
        engine: impl Engine + 'static,
    ) -> RuntimeResult<AgentId> {
        // shared world activity
        let _activity = self.enter_activity()?;

        // runtime lookup
        let mut runtimes = self.runtimes.write();
        let runtime = runtimes.get_mut(&runtime_id).ok_or_else(|| {
            RuntimeError::RuntimeNotFound {
                runtime_id: runtime_id.0,
            }
            .boxed()
        })?;

        let mode = self.trace.mode();
        let agent_id = runtime.spawn_agent_with_options(self, options, Box::new(engine))?;

        // record mode needs one structural spawn record for suffix replay
        let replay_image = if mode == destack_workspace::ExecutionMode::Record {
            let agent = runtime.agent_mut(agent_id).ok_or_else(|| {
                RuntimeError::AgentNotFound {
                    agent_id: agent_id.0,
                }
                .boxed()
            })?;

            Some(agent.capture_image(CaptureMode::Suspend)?)
        } else {
            None
        };

        if let Some(agent) = replay_image {
            self.accept(Outcome::AgentSpawned { agent })?;
        }

        Ok(agent_id)
    }

    /// Run one entrypoint on one stored runtime.
    pub fn run_entrypoint(
        &self,
        runtime_id: RuntimeId,
        entry: &Entry,
        args: &[heap::Value],
    ) -> RuntimeResult<EngineOutput> {
        let input = Input::RunEntrypoint {
            runtime_id,
            entry: entry.entry_reference(),
            args: args.to_vec(),
        };
        let input = self.resolve_input(input)?;

        if self.trace.mode() == destack_workspace::ExecutionMode::Record {
            self.ingest(input.clone())?;
        }

        self.run_entrypoint_inner(runtime_id, entry, args)
    }

    /// Return the stored runtime ids in stable order.
    pub fn runtime_ids(&self) -> Vec<RuntimeId> {
        self.runtimes.read().keys().copied().collect()
    }

    /// Run one closure with one stored runtime immutably borrowed.
    pub(crate) fn with_runtime<R>(
        &self,
        runtime_id: RuntimeId,
        callback: impl FnOnce(&Runtime) -> RuntimeResult<R>,
    ) -> RuntimeResult<R> {
        let runtimes = self.runtimes.read();
        let runtime = runtimes.get(&runtime_id).ok_or_else(|| {
            RuntimeError::RuntimeNotFound {
                runtime_id: runtime_id.0,
            }
            .boxed()
        })?;

        callback(runtime)
    }

    /// Run one closure with one stored runtime mutably borrowed.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn with_runtime_mut<R>(
        &self,
        runtime_id: RuntimeId,
        callback: impl FnOnce(&mut Runtime) -> RuntimeResult<R>,
    ) -> RuntimeResult<R> {
        let mut runtimes = self.runtimes.write();
        let runtime = runtimes.get_mut(&runtime_id).ok_or_else(|| {
            RuntimeError::RuntimeNotFound {
                runtime_id: runtime_id.0,
            }
            .boxed()
        })?;

        callback(runtime)
    }

    /// Remove one stored runtime without tracing the outer invocation.
    pub(crate) fn remove_runtime_inner(
        &self,
        runtime_id: RuntimeId,
    ) -> RuntimeResult<Box<Runtime>> {
        // detach the runtime first
        let _activity = self.enter_activity()?;
        let runtime = {
            let mut runtimes = self.runtimes.write();
            runtimes.remove(&runtime_id).ok_or_else(|| {
                RuntimeError::RuntimeNotFound {
                    runtime_id: runtime_id.0,
                }
                .boxed()
            })?
        };

        // remove all owned agents through the normal world mutation path
        let agent_ids = runtime.agent_ids();
        for agent_id in agent_ids {
            self.remove_agent(agent_id)?;
        }

        Ok(runtime)
    }

    /// Run one entrypoint without tracing the outer invocation.
    pub(crate) fn run_entrypoint_inner(
        &self,
        runtime_id: RuntimeId,
        entry: &Entry,
        args: &[heap::Value],
    ) -> RuntimeResult<EngineOutput> {
        let _activity = self.enter_activity()?;
        let mut runtimes = self.runtimes.write();
        let runtime = runtimes.get_mut(&runtime_id).ok_or_else(|| {
            RuntimeError::RuntimeNotFound {
                runtime_id: runtime_id.0,
            }
            .boxed()
        })?;

        runtime.run_entrypoint(self, entry, args)
    }

    /// Run one replayable entrypoint without tracing the outer invocation.
    pub(crate) fn run_replayable_entrypoint_inner(
        &self,
        runtime_id: RuntimeId,
        entry: &EntryReference,
        args: &[heap::Value],
    ) -> RuntimeResult<EngineOutput> {
        let _activity = self.enter_activity()?;
        let mut runtimes = self.runtimes.write();
        let runtime = runtimes.get_mut(&runtime_id).ok_or_else(|| {
            RuntimeError::RuntimeNotFound {
                runtime_id: runtime_id.0,
            }
            .boxed()
        })?;

        runtime.run_replayable_entrypoint_for_agent(self, runtime.primary_agent_id(), entry, args)
    }

    /// Install one restored runtime image without tracing the outer invocation.
    pub(crate) fn install_runtime_image(
        &self,
        runtime_image: &RuntimeImage,
        agent_images: &std::collections::BTreeMap<AgentId, AgentImage>,
        rebind_context: Option<&RebindContext>,
    ) -> RuntimeResult<()> {
        let runtime = Runtime::from_image(self, runtime_image, agent_images, rebind_context)?;
        let runtime_id = runtime.runtime_id();

        let _activity = self.enter_activity()?;
        let mut runtimes = self.runtimes.write();
        if runtimes.insert(runtime_id, Box::new(runtime)).is_some() {
            return Err(RuntimeError::RuntimeAlreadyExists {
                runtime_id: runtime_id.0,
            }
            .boxed());
        }

        Ok(())
    }

    /// Register one runtime in world topology without tracing one mutation.
    pub(crate) fn register_runtime_topology(
        &self,
        runtime_id: RuntimeId,
        runtime_name: String,
        runtime_labels: BTreeMap<String, String>,
        primary_agent_id: AgentId,
        primary_agent_name: String,
        primary_agent_labels: BTreeMap<String, String>,
    ) -> RuntimeResult<()> {
        let _activity = self.enter_activity()?;

        let mut topology = self.topology.write();
        topology
            .add_runtime(
                runtime_id,
                runtime_name,
                runtime_labels,
                primary_agent_id,
                primary_agent_name,
                primary_agent_labels,
            )
            .map_err(|message| {
                RuntimeError::Internal {
                    message: message.to_string(),
                }
                .boxed()
            })?;

        Ok(())
    }

    /// Register one agent in world topology without tracing one mutation.
    pub(crate) fn register_agent_topology(
        &self,
        runtime_id: RuntimeId,
        agent_id: AgentId,
        agent_name: String,
        agent_labels: BTreeMap<String, String>,
    ) -> RuntimeResult<()> {
        let _activity = self.enter_activity()?;

        let mut topology = self.topology.write();
        topology
            .add_agent(runtime_id, agent_id, agent_name, agent_labels)
            .map_err(|message| {
                RuntimeError::Internal {
                    message: message.to_string(),
                }
                .boxed()
            })?;

        Ok(())
    }

    /// Install one restored agent image without tracing the outer invocation.
    pub(crate) fn install_agent_image(
        &self,
        agent_image: &AgentImage,
        rebind_context: Option<&RebindContext>,
    ) -> RuntimeResult<()> {
        let platform_args = {
            let runtimes = self.runtimes.read();
            let runtime = runtimes.get(&agent_image.runtime_id).ok_or_else(|| {
                RuntimeError::RuntimeNotFound {
                    runtime_id: agent_image.runtime_id.0,
                }
                .boxed()
            })?;

            runtime.platform_args_arc()
        };

        let agent = Agent::from_image(self, platform_args, agent_image, rebind_context)?;

        let _activity = self.enter_activity()?;
        let mut runtimes = self.runtimes.write();
        let runtime = runtimes.get_mut(&agent_image.runtime_id).ok_or_else(|| {
            RuntimeError::RuntimeNotFound {
                runtime_id: agent_image.runtime_id.0,
            }
            .boxed()
        })?;

        runtime.insert_restored_agent(agent)?;

        Ok(())
    }
}
