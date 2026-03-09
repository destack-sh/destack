use std::sync::Arc;

use destack_heap as heap;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::engine::{Engine, EngineOutput, Entry};
use crate::runtime::{AgentId, Runtime};
use destack_workspace::RuntimeOptions;

use super::{RuntimeId, World};

impl World {
    /// Spawn one live runtime owned by this world and return its identifier.
    pub fn spawn_runtime(
        &self,
        platform_args: impl Into<Arc<[String]>>,
        options: &RuntimeOptions,
        engine: impl Engine + 'static,
    ) -> RuntimeResult<RuntimeId> {
        let _activity = self.enter_activity()?;
        let runtime =
            Runtime::from_options_in_world(platform_args, options, self, Box::new(engine))?;
        let runtime_id = runtime.runtime_id();

        let mut runtimes = self.runtimes.write();
        if runtimes.insert(runtime_id, Box::new(runtime)).is_some() {
            return Err(RuntimeError::Internal {
                message: format!("world already contains runtime {}", runtime_id.0),
            }
            .boxed());
        }

        Ok(runtime_id)
    }

    /// Spawn one additional agent in one stored runtime.
    pub fn spawn_agent(
        &self,
        runtime_id: RuntimeId,
        engine: impl Engine + 'static,
    ) -> RuntimeResult<AgentId> {
        let _activity = self.enter_activity()?;
        let mut runtimes = self.runtimes.write();
        let runtime = runtimes.get_mut(&runtime_id).ok_or_else(|| {
            RuntimeError::Internal {
                message: format!("runtime {} does not exist", runtime_id.0),
            }
            .boxed()
        })?;

        runtime.spawn_agent(self, Box::new(engine))
    }

    /// Run one entrypoint on one stored runtime.
    pub fn run_entrypoint(
        &self,
        runtime_id: RuntimeId,
        entry: &Entry,
        args: &[heap::Value],
    ) -> RuntimeResult<EngineOutput> {
        let _activity = self.enter_activity()?;
        let mut runtimes = self.runtimes.write();
        let runtime = runtimes.get_mut(&runtime_id).ok_or_else(|| {
            RuntimeError::Internal {
                message: format!("runtime {} does not exist", runtime_id.0),
            }
            .boxed()
        })?;

        runtime.run_entrypoint(self, entry, args)
    }

    /// Return the stored runtime ids in stable order.
    pub fn runtime_ids(&self) -> Vec<RuntimeId> {
        self.runtimes.read().keys().copied().collect()
    }

    /// Run one closure with one stored runtime immutably borrowed.
    #[cfg(test)]
    pub(crate) fn with_runtime<R>(
        &self,
        runtime_id: RuntimeId,
        callback: impl FnOnce(&Runtime) -> RuntimeResult<R>,
    ) -> RuntimeResult<R> {
        let runtimes = self.runtimes.read();
        let runtime = runtimes.get(&runtime_id).ok_or_else(|| {
            RuntimeError::Internal {
                message: format!("runtime {} does not exist", runtime_id.0),
            }
            .boxed()
        })?;

        callback(runtime)
    }

    /// Run one closure with one stored runtime mutably borrowed.
    #[cfg(test)]
    pub(crate) fn with_runtime_mut<R>(
        &self,
        runtime_id: RuntimeId,
        callback: impl FnOnce(&mut Runtime) -> RuntimeResult<R>,
    ) -> RuntimeResult<R> {
        let mut runtimes = self.runtimes.write();
        let runtime = runtimes.get_mut(&runtime_id).ok_or_else(|| {
            RuntimeError::Internal {
                message: format!("runtime {} does not exist", runtime_id.0),
            }
            .boxed()
        })?;

        callback(runtime)
    }
}
