use crate::{AnalyzeResult, Compiler};
use destack_compiler_macros::DefineTask;
use destack_source::ModuleId;

/// Task to analyze something.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Analyze)]
pub enum AnalyzeTask {
    /// Analyze a module completely (declare, infer, validate).
    #[task(code = 1, trace = "module={module}")]
    AnalyzeModule { module: ModuleId },

    /// Analyze declarations.
    #[task(code = 2, trace = "module={module}")]
    AnalyzeModuleDeclare { module: ModuleId },

    /// Infer expression types.
    #[task(code = 3, trace = "module={module}")]
    AnalyzeModuleInfer { module: ModuleId },

    /// Final validation pass.
    #[task(code = 4, trace = "module={module}")]
    AnalyzeModuleValidate { module: ModuleId },
}

impl Compiler {
    /// Process an analyze task.
    pub fn process_analyze(&self, task: AnalyzeTask) -> AnalyzeResult<()> {
        match task {
            AnalyzeTask::AnalyzeModule { module } => {
                self.require_analyze_module_validate(module)?;
            }
            AnalyzeTask::AnalyzeModuleDeclare { module } => {
                self.require_resolve_module_canonical(module)?;
                self.analyze_module_declare(module)?;
            }
            AnalyzeTask::AnalyzeModuleInfer { module } => {
                self.require_analyze_module_declare(module)?;
                self.analyze_module_infer(module)?;
            }
            AnalyzeTask::AnalyzeModuleValidate { module } => {
                self.require_analyze_module_infer(module)?;
                self.analyze_module_validate(module)?;
            }
        }
        Ok(())
    }
}
