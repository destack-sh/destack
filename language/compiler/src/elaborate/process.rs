use destack_source::ModuleId;
use destack_workspace::Program;

use crate::{Compiler, ElaborateResult, Task, TaskDebug, TaskDependencyError, TaskOutput};

// nocheckin: introduce a task macro (like the DefineError/DefineWarning macros)
// nocheckin: remove TaskOutputs?

/// Task to elaborate something.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ElaborateTask {
    /// Desugar a module.
    ElaborateModuleDesugar { module: ModuleId },

    /// Deload a module.
    ElaborateModuleDeload { module: ModuleId },

    /// Reify a module.
    ElaborateModuleReify { module: ModuleId },

    /// Elaborate a module.
    ElaborateModule { module: ModuleId },
}

impl ElaborateTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::ElaborateModuleDesugar { .. } => 1,
            Self::ElaborateModuleDeload { .. } => 2,
            Self::ElaborateModuleReify { .. } => 3,
            Self::ElaborateModule { .. } => 4,
        }
    }
}

impl TaskDebug for ElaborateTask {
    fn name(&self) -> &'static str {
        match self {
            Self::ElaborateModuleDesugar { .. } => "desugar module",
            Self::ElaborateModuleDeload { .. } => "deload module",
            Self::ElaborateModuleReify { .. } => "reify module",
            Self::ElaborateModule { .. } => "elaborate module",
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            Self::ElaborateModuleDesugar { module } => {
                let module = program.modules.get(*module);
                let uri = module.read().uri.clone().to_string();
                format!(r#"module="{uri}""#)
            }
            Self::ElaborateModuleDeload { module } => {
                let module = program.modules.get(*module);
                let uri = module.read().uri.clone().to_string();
                format!(r#"module="{uri}""#)
            }
            Self::ElaborateModuleReify { module } => {
                let module = program.modules.get(*module);
                let uri = module.read().uri.clone().to_string();
                format!(r#"module="{uri}""#)
            }
            Self::ElaborateModule { module } => {
                let module = program.modules.get(*module);
                let uri = module.read().uri.clone().to_string();
                format!(r#"module="{uri}""#)
            }
        }
    }
}

impl From<ElaborateTask> for Task {
    fn from(task: ElaborateTask) -> Self {
        Task::Elaborate(task)
    }
}

/// Output of an elaborate task.
#[derive(Debug, Clone, PartialEq)]
pub struct ElaborateOutput {}

impl From<ElaborateOutput> for TaskOutput {
    fn from(output: ElaborateOutput) -> Self {
        TaskOutput::Elaborate(output)
    }
}

impl Compiler {
    /// Process an elaborate task.
    pub fn process_elaborate(&self, task: ElaborateTask) -> ElaborateResult<ElaborateOutput> {
        match task {
            ElaborateTask::ElaborateModuleDesugar { module } => {
                self.require_analyze_module(module)?;
                self.desugar_module(module)?;
            }
            ElaborateTask::ElaborateModuleDeload { module } => {
                self.require_elaborate_module_desugar(module)?;
                self.deload_module(module)?;
            }
            ElaborateTask::ElaborateModuleReify { module } => {
                self.require_elaborate_module_deload(module)?;
                self.reify_module(module)?;
            }
            ElaborateTask::ElaborateModule { module } => {
                self.require_elaborate_module_reify(module)?;
            }
        }
        Ok(ElaborateOutput {})
    }

    /// Ensure a module has been desugared.
    pub fn require_elaborate_module_desugar(
        &self,
        module: ModuleId,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(ElaborateTask::ElaborateModuleDesugar { module })
    }

    /// Ensure a module has been deloaded.
    pub fn require_elaborate_module_deload(
        &self,
        module: ModuleId,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(ElaborateTask::ElaborateModuleDeload { module })
    }

    /// Ensure a module has been reified.
    pub fn require_elaborate_module_reify(
        &self,
        module: ModuleId,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(ElaborateTask::ElaborateModuleReify { module })
    }

    /// Ensure a module has been elaborated.
    pub fn require_elaborate_module(&self, module: ModuleId) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(ElaborateTask::ElaborateModule { module })
    }
}
