use crate::{Compiler, Task, TaskDebug, TaskOutput, TaskResultCollector, ValidateResult};

use destack_dir::{ModuleId, Program};

/// Task to validate something.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ValidateTask {
    /// Validate a module.
    Validate { module: ModuleId },
}

impl ValidateTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Validate { .. } => 1,
        }
    }
}

impl TaskDebug for ValidateTask {
    fn name(&self) -> &'static str {
        match self {
            Self::Validate { .. } => "module",
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            Self::Validate { module } => {
                let module = program.modules.get(*module);
                let uri = module.read().uri.clone().to_string();
                format!(r#"module="{uri}""#)
            }
        }
    }
}

impl From<ValidateTask> for Task {
    fn from(task: ValidateTask) -> Self {
        Task::Validate(task)
    }
}

/// Output of a validate task.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidateOutput {}

impl From<ValidateOutput> for TaskOutput {
    fn from(output: ValidateOutput) -> Self {
        TaskOutput::Validate(output)
    }
}

impl Compiler {
    /// Process a validate task.
    pub fn process_validate(&self, task: ValidateTask) -> ValidateResult<ValidateOutput> {
        match task {
            ValidateTask::Validate { module } => self.validate_module(module)?,
        }
        Ok(ValidateOutput {})
    }

    /// Validate a module.
    pub fn validate_module(&self, module_id: ModuleId) -> ValidateResult<()> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let _tree = module.tree.read();
        let _symbols = module.symbols.read();
        let mut _types = module.types.write();
        let mut _instances = module.instances.write();
        let _collector = TaskResultCollector::new();

        // TODO #Incomplete: validate module

        Ok(())
    }
}
