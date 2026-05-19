use std::sync::Arc;

use destack_engine as engine;
use destack_workspace::{Environment, RuntimeOptions};

use crate::diagnostic::RuntimeResult;
use crate::runtime::engine::{Engine, Entry};
use crate::world::{RuntimeId, World};

/// Complete startup request for one Destack world.
pub struct Launch {
    /// Runtime options used to construct the initial world and runtime.
    pub options: RuntimeOptions,
    /// Ambient environment exposed to the launched runtime.
    pub environment: Arc<Environment>,
    /// Execution backend installed into the initial worker.
    pub engine: Engine,
    /// User entrypoint invoked after runtime bootstrap.
    pub entry: Entry,
    /// Values passed to the user entrypoint.
    pub entry_args: Vec<engine::Value>,
}

/// Result of launching one Destack world.
#[derive(Debug)]
pub struct LaunchResult {
    /// Live world after entrypoint execution and idle draining.
    pub world: World,
    /// Initial runtime identifier.
    pub runtime_id: RuntimeId,
    /// Value returned by the user entrypoint.
    pub value: engine::Value,
}

impl std::fmt::Debug for Launch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Launch")
            .field("options", &self.options)
            .field("environment", &self.environment)
            .field("engine", &"<launch engine>")
            .field("entry", &self.entry)
            .field("entry_args", &self.entry_args)
            .finish()
    }
}

impl Launch {
    /// Create a launch request.
    pub fn new(options: RuntimeOptions, engine: impl Into<Engine>, entry: Entry) -> Self {
        Self {
            options,
            environment: Arc::new(Environment::default()),
            engine: engine.into(),
            entry,
            entry_args: Vec::new(),
        }
    }

    /// Replace the ambient environment.
    pub fn with_environment(mut self, environment: impl Into<Arc<Environment>>) -> Self {
        self.environment = environment.into();
        self
    }

    /// Replace the entrypoint arguments.
    pub fn with_entry_args(mut self, entry_args: impl Into<Vec<engine::Value>>) -> Self {
        self.entry_args = entry_args.into();
        self
    }

    /// Launch one runtime in a new world.
    pub fn run(self) -> RuntimeResult<LaunchResult> {
        let Launch {
            options,
            environment,
            engine,
            entry,
            entry_args,
        } = self;
        let mut world = World::from_options(&options)?;

        // bootstrap the initial runtime
        let runtime_id = world.spawn_runtime(environment, &options, engine)?;
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
