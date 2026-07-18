use std::sync::Arc;

use destack_artifact::ConditionSet;
use destack_program as program;
use destack_repository::{Environment, RuntimeOptions};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::machine::{Entry, Execution};
use crate::world::{Run, RunOutcome, RuntimeId, World};

/// Complete startup request for one Destack world.
pub struct Launch {
    /// Runtime options used to construct the initial world and runtime.
    pub options: RuntimeOptions,
    /// The active runtime conditions.
    pub conditions: Arc<ConditionSet>,
    /// Ambient environment exposed to the launched runtime.
    pub environment: Arc<Environment>,
    /// Durable program instantiated by the runtime.
    pub program: Arc<program::Program>,
    /// Execution strategy used by the runtime.
    pub execution: Execution,
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
            .field("conditions", &self.conditions)
            .field("environment", &self.environment)
            .field("program", &self.program)
            .field("execution", &self.execution)
            .field("entry", &self.entry)
            .field("entry_args", &self.entry_args)
            .finish()
    }
}

impl Launch {
    /// Create a launch request.
    pub fn new(
        options: RuntimeOptions,
        conditions: impl Into<Arc<ConditionSet>>,
        environment: impl Into<Arc<Environment>>,
        program: impl Into<Arc<program::Program>>,
        execution: Execution,
        entry: Entry,
    ) -> Self {
        Self {
            options,
            conditions: conditions.into(),
            environment: environment.into(),
            program: program.into(),
            execution,
            entry,
            entry_args: Vec::new(),
        }
    }

    /// Launch one runtime in a new world.
    pub fn run(self) -> RuntimeResult<LaunchResult> {
        let Launch {
            options,
            conditions,
            environment,
            program,
            execution,
            entry,
            entry_args,
        } = self;
        let mut world = World::new(&options, environment.clone())?;

        // bootstrap the initial runtime
        let runtime_id =
            world.spawn_runtime(environment, &options, conditions, program, execution)?;
        let value = world.run_entrypoint(runtime_id, &entry, &entry_args)?;

        // drain work scheduled by the entrypoint
        if let RunOutcome::Stopped { .. } = world.run(Run::UntilIdle)? {
            return Err(RuntimeError::execution_stopped().boxed());
        }

        Ok(LaunchResult {
            world,
            runtime_id,
            value,
        })
    }
}
