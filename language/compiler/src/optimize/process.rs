use crate::{Compiler, OptimizeResult, TaskDependencyError};

use destack_compiler_macros::DefineTask;
use destack_source::ModuleId;
use destack_workspace::TargetId;

/// Task to optimize something.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Optimize)]
pub enum OptimizeTask {
    /// Optimize a module's MIR.
    #[task(code = 1, trace = "module={module} target={target}")]
    OptimizeModule { module: ModuleId, target: TargetId },
}

impl Compiler {
    /// Process an optimize task.
    pub fn process_optimize(&self, task: OptimizeTask) -> OptimizeResult<()> {
        match task {
            OptimizeTask::OptimizeModule { module, target } => {
                self.require_verify_module(module, &target)?;
                self.optimize_module(module, &target)?;
            }
        }
        Ok(())
    }

    /// Ensure a module has been optimized.
    pub fn require_optimize(
        &self,
        module: ModuleId,
        target: &TargetId,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(OptimizeTask::OptimizeModule {
            module,
            target: target.clone(),
        })
    }

    /// Optimize a module's MIR.
    fn optimize_module(&self, _module: ModuleId, _target: &TargetId) -> OptimizeResult<()> {
        // NOTE #Incomplete: implement optimize
        // - dead code elimination
        // - constant folding
        // - inlining
        Ok(())
    }
}
