use crate::{BindResult, Compiler};
use destack_compiler_macros::DefineTask;
use destack_source::ModuleId;

/// Task to bind AST into DIR (including syntactic desugaring).
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Bind)]
pub enum BindTask {
    /// Bind a module completely (build, desugar, validate).
    #[task(code = 1, trace = "module={module}")]
    BindModule { module: ModuleId },

    /// Build DIR for a module by binding its AST.
    #[task(code = 2, trace = "module={module}")]
    BindModuleBuild { module: ModuleId },

    /// Desugar the module syntactically.
    #[task(code = 3, trace = "module={module}")]
    BindModuleDesugar { module: ModuleId },

    /// Validate module "syntactic" correctness.
    #[task(code = 4, trace = "module={module}")]
    BindModuleValidate { module: ModuleId },
}

impl Compiler {
    /// Process a bind task.
    pub fn process_bind(&self, task: BindTask) -> BindResult<()> {
        match task {
            BindTask::BindModule { module } => {
                self.require_bind_module_desugar(module)?;
            }
            BindTask::BindModuleBuild { module } => {
                self.bind_module_build(module)?;
            }
            BindTask::BindModuleDesugar { module } => {
                self.bind_module_desugar_phase(module)?;
                self.stats.record_bind();
            }
            BindTask::BindModuleValidate { module } => {
                self.bind_module_validate(module)?;
            }
        }
        Ok(())
    }
}
