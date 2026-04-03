use std::sync::Arc;

use destack_core::CaptureMode;
use destack_heap as heap;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::HostEvent;
use crate::platform::resource::ResourceRebinders;
use crate::runtime::engine::{Engine, Entry, ExecutionOutput};
use crate::runtime::poller::PollerEvent;
use crate::runtime::trace::Outcome;
use crate::runtime::{Agent, AgentId, AgentImage, Runtime, RuntimeImage};
use destack_workspace::{ExecutionMode, RuntimeOptions};

use super::{Input, RuntimeId, World};

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
        &mut self,
        platform_args: impl Into<Arc<[String]>>,
        options: &RuntimeOptions,
        engine: impl Engine + 'static,
    ) -> RuntimeResult<RuntimeId> {
        let mode = self.trace.mode();
        let world = self.world_ref();
        let mut runtime =
            Runtime::from_options_in_world(platform_args, options, &world, Box::new(engine))?;
        let runtime_id = runtime.runtime_id();

        // fast and deterministic modes do not need one structural spawn image
        let replay_image = if mode == ExecutionMode::Record {
            Some(runtime.capture_image(CaptureMode::Suspend)?)
        } else {
            None
        };

        // install the live runtime
        if self
            .runtimes
            .insert(runtime_id, Box::new(runtime))
            .is_some()
        {
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
    pub fn remove_runtime(&mut self, runtime_id: RuntimeId) -> RuntimeResult<Box<Runtime>> {
        let input = Input::RemoveRuntime { runtime_id };
        let input = self.resolve_input(input)?;

        if self.trace.mode() == ExecutionMode::Record {
            self.ingest(input.clone())?;
        }

        let runtime = self.remove_runtime_inner(runtime_id)?;

        Ok(runtime)
    }

    /// Spawn one additional agent in one stored runtime.
    pub fn spawn_agent(
        &mut self,
        runtime_id: RuntimeId,
        engine: impl Engine + 'static,
    ) -> RuntimeResult<AgentId> {
        self.spawn_agent_with_options(runtime_id, &RuntimeOptions::default(), engine)
    }

    /// Spawn one additional agent with explicit options in one stored runtime.
    pub fn spawn_agent_with_options(
        &mut self,
        runtime_id: RuntimeId,
        options: &RuntimeOptions,
        engine: impl Engine + 'static,
    ) -> RuntimeResult<AgentId> {
        let world = self.world_ref();
        let runtime = self.runtimes.get_mut(&runtime_id).ok_or_else(|| {
            RuntimeError::RuntimeNotFound {
                runtime_id: runtime_id.0,
            }
            .boxed()
        })?;

        let mode = self.trace.mode();
        let agent_id = runtime.spawn_agent_with_options(&world, options, Box::new(engine))?;

        // record mode needs one structural spawn record for suffix replay
        let replay_image = if mode == ExecutionMode::Record {
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
        &mut self,
        runtime_id: RuntimeId,
        entry: &Entry,
        args: &[heap::Value],
    ) -> RuntimeResult<ExecutionOutput> {
        let input = Input::RunEntrypoint {
            runtime_id,
            entry: entry.clone(),
            args: args.to_vec(),
        };
        let input = self.resolve_input(input)?;

        if self.trace.mode() == ExecutionMode::Record {
            self.ingest(input.clone())?;
        }

        self.run_entrypoint_inner(runtime_id, entry, args)
    }

    /// Return the stored runtime ids in stable order.
    pub fn runtime_ids(&self) -> Vec<RuntimeId> {
        self.runtimes.keys().copied().collect()
    }

    /// Borrow one stored runtime immutably.
    pub(crate) fn runtime(&self, runtime_id: RuntimeId) -> RuntimeResult<&Runtime> {
        self.runtimes
            .get(&runtime_id)
            .map(Box::as_ref)
            .ok_or_else(|| {
                RuntimeError::RuntimeNotFound {
                    runtime_id: runtime_id.0,
                }
                .boxed()
            })
    }

    /// Borrow one stored runtime mutably.
    #[cfg(test)]
    pub(crate) fn runtime_mut(&mut self, runtime_id: RuntimeId) -> RuntimeResult<&mut Runtime> {
        self.runtimes
            .get_mut(&runtime_id)
            .map(Box::as_mut)
            .ok_or_else(|| {
                RuntimeError::RuntimeNotFound {
                    runtime_id: runtime_id.0,
                }
                .boxed()
            })
    }

    /// Remove one stored runtime without tracing the outer invocation.
    pub(crate) fn remove_runtime_inner(
        &mut self,
        runtime_id: RuntimeId,
    ) -> RuntimeResult<Box<Runtime>> {
        let runtime = self.runtimes.remove(&runtime_id).ok_or_else(|| {
            RuntimeError::RuntimeNotFound {
                runtime_id: runtime_id.0,
            }
            .boxed()
        })?;

        // remove all owned agents through the normal world mutation path
        let agent_ids = runtime.agent_ids();
        for agent_id in agent_ids {
            self.remove_agent(agent_id)?;
        }

        Ok(runtime)
    }

    /// Run one entrypoint without tracing the outer invocation.
    pub(crate) fn run_entrypoint_inner(
        &mut self,
        runtime_id: RuntimeId,
        entry: &Entry,
        args: &[heap::Value],
    ) -> RuntimeResult<ExecutionOutput> {
        let world = self.world_ref();
        let runtime = self.runtimes.get_mut(&runtime_id).ok_or_else(|| {
            RuntimeError::RuntimeNotFound {
                runtime_id: runtime_id.0,
            }
            .boxed()
        })?;

        runtime.run_entrypoint(&world, entry, args)
    }

    /// Run one replayable entrypoint without tracing the outer invocation.
    pub(crate) fn run_replayable_entrypoint_inner(
        &mut self,
        runtime_id: RuntimeId,
        entry: &Entry,
        args: &[heap::Value],
    ) -> RuntimeResult<ExecutionOutput> {
        let world = self.world_ref();
        let runtime = self.runtimes.get_mut(&runtime_id).ok_or_else(|| {
            RuntimeError::RuntimeNotFound {
                runtime_id: runtime_id.0,
            }
            .boxed()
        })?;

        runtime.run_replayable_entrypoint_for_agent(&world, runtime.primary_agent_id(), entry, args)
    }

    /// Install one restored runtime image without tracing the outer invocation.
    pub(crate) fn install_runtime_image(
        &mut self,
        runtime_image: &RuntimeImage,
        agent_images: &std::collections::BTreeMap<AgentId, AgentImage>,
        rebind_context: Option<&ResourceRebinders>,
    ) -> RuntimeResult<()> {
        let world = self.world_ref();
        let runtime = Runtime::from_image(&world, runtime_image, agent_images, rebind_context)?;
        let runtime_id = runtime.runtime_id();

        if self
            .runtimes
            .insert(runtime_id, Box::new(runtime))
            .is_some()
        {
            return Err(RuntimeError::RuntimeAlreadyExists {
                runtime_id: runtime_id.0,
            }
            .boxed());
        }

        Ok(())
    }

    /// Install one restored agent image without tracing the outer invocation.
    pub(crate) fn install_agent_image(
        &mut self,
        agent_image: &AgentImage,
        rebind_context: Option<&ResourceRebinders>,
    ) -> RuntimeResult<()> {
        let platform_args = self
            .runtimes
            .get(&agent_image.runtime_id)
            .ok_or_else(|| {
                RuntimeError::RuntimeNotFound {
                    runtime_id: agent_image.runtime_id.0,
                }
                .boxed()
            })?
            .platform_args_arc();

        let world = self.world_ref();
        let agent = Agent::from_image(&world, platform_args, agent_image, rebind_context)?;

        let runtime = self
            .runtimes
            .get_mut(&agent_image.runtime_id)
            .ok_or_else(|| {
                RuntimeError::RuntimeNotFound {
                    runtime_id: agent_image.runtime_id.0,
                }
                .boxed()
            })?;

        runtime.insert_restored_agent(agent)?;

        Ok(())
    }
}
