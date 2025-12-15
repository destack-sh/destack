use destack_compiler_macros::DefineTask;
use destack_source::ModuleId;

use crate::{Compiler, ElaborateResult, TaskDependencyError};

/// Task to elaborate something.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Elaborate)]
pub enum ElaborateTask {
    /// Desugar a module.
    #[task(code = 1, trace = "module={module}")]
    ElaborateModuleDesugar { module: ModuleId },

    /// Deload a module.
    #[task(code = 2, trace = "module={module}")]
    ElaborateModuleDeload { module: ModuleId },

    /// Reify a module.
    #[task(code = 3, trace = "module={module}")]
    ElaborateModuleReify { module: ModuleId },

    /// Elaborate a module.
    #[task(code = 4, trace = "module={module}")]
    ElaborateModule { module: ModuleId },
}

impl Compiler {
    /// Process an elaborate task.
    pub fn process_elaborate(&self, task: ElaborateTask) -> ElaborateResult<()> {
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
        Ok(())
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
