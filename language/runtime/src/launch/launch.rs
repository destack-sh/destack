use std::sync::Arc;

use destack_program as program;
use destack_repository::{Environment, RuntimeOptions};

use crate::diagnostic::RuntimeResult;
use crate::runtime::executor::{Backend, Entry};
use crate::world::{RuntimeId, World};

/// Complete startup request for one Destack world.
pub struct Launch {
    /// Runtime options used to construct the initial world and runtime.
    pub options: RuntimeOptions,
    /// Ambient environment exposed to the launched runtime.
    pub environment: Arc<Environment>,
    /// Execution backend installed into the initial worker.
    pub backend: Backend,
    /// User entrypoint invoked after runtime bootstrap.
    pub entry: Entry,
    /// Values passed to the user entrypoint.
    pub entry_args: Vec<program::Value>,
}

/// Result of launching one Destack world.
#[derive(Debug)]
pub struct LaunchResult {
    /// Live world after entrypoint execution and idle draining.
    pub world: World,
    /// Initial runtime identifier.
    pub runtime_id: RuntimeId,
    /// Value returned by the user entrypoint.
    pub value: program::Value,
}

impl std::fmt::Debug for Launch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Launch")
            .field("options", &self.options)
            .field("environment", &self.environment)
            .field("backend", &self.backend)
            .field("entry", &self.entry)
            .field("entry_args", &self.entry_args)
            .finish()
    }
}

impl Launch {
    /// Create a launch request.
    pub fn new(
        options: RuntimeOptions,
        environment: impl Into<Arc<Environment>>,
        backend: impl Into<Backend>,
        entry: Entry,
    ) -> Self {
        Self {
            options,
            environment: environment.into(),
            backend: backend.into(),
            entry,
            entry_args: Vec::new(),
        }
    }

    /// Launch one runtime in a new world.
    pub fn run(self) -> RuntimeResult<LaunchResult> {
        let Launch {
            options,
            environment,
            backend,
            entry,
            entry_args,
        } = self;
        let mut world = World::new(&options, environment.clone())?;

        // bootstrap the initial runtime
        let runtime_id = world.spawn_runtime(environment, &options, backend)?;
        let value = world.run_entrypoint(runtime_id, &entry, &entry_args)?;

        // drain work scheduled by the entrypoint
        world.tick_until_idle()?;

        Ok(LaunchResult {
            world,
            runtime_id,
            value,
        })
    }
}
