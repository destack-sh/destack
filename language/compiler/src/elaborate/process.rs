use destack_compiler_macros::DefineTask;
use destack_source::ModuleId;

use crate::{Compiler, ElaborateResult, TaskDependencyError};

/// Task to elaborate a module: post-analysis transforms that need type information
/// but are target-independent.
///
/// Elaboration has two phases:
/// 1. **Transform** (simplify): reduce syntactic complexity within the same abstraction level
/// 2. **Reify** (make concrete): cross abstraction boundaries, turn abstract concepts into code
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Elaborate)]
pub enum ElaborateTask {
    /// Elaborate a module (umbrella task).
    #[task(code = 1, trace = "module={module}")]
    ElaborateModule { module: ModuleId },

    /// Semantic transforms: simplify control flow and syntax.
    /// - Pattern matching → decision trees
    /// - Range expressions → iterator construction
    /// - Maybe/Must → explicit error handling
    #[task(code = 2, trace = "module={module}")]
    ElaborateModuleTransform { module: ModuleId },

    /// Reification: make abstractions concrete.
    /// - Tree literals → constructor calls
    /// - Operators → resolved method calls (based on Resolution)
    /// - Type descriptors → runtime type objects
    #[task(code = 3, trace = "module={module}")]
    ElaborateModuleReify { module: ModuleId },
}

impl Compiler {
    /// Process an elaborate task.
    pub fn process_elaborate(&self, task: ElaborateTask) -> ElaborateResult<()> {
        match task {
            ElaborateTask::ElaborateModule { module } => {
                self.require_elaborate_module_reify(module)?;
            }
            ElaborateTask::ElaborateModuleTransform { module } => {
                self.elaborate_module_transform_phase(module)?;
            }
            ElaborateTask::ElaborateModuleReify { module } => {
                self.elaborate_module_reify_phase(module)?;
            }
        }
        Ok(())
    }

    /// Ensure a module has been elaborated.
    pub fn require_elaborate_module(&self, module: ModuleId) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(ElaborateTask::ElaborateModule { module })
    }
}
