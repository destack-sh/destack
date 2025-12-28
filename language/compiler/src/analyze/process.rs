use crate::{AnalyzeResult, Compiler};
use destack_compiler_macros::DefineTask;
use destack_source::ModuleId;
use destack_workspace::ProfileId;

/// Task to analyze something.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Analyze)]
pub enum AnalyzeTask {
    /// Analyze a module completely (declare, infer, validate).
    #[task(code = 1, trace = "module={module} profile={profile}")]
    AnalyzeModule {
        module: ModuleId,
        profile: ProfileId,
    },

    /// Analyze declarations.
    #[task(code = 2, trace = "module={module} profile={profile}")]
    AnalyzeModuleDeclare {
        module: ModuleId,
        profile: ProfileId,
    },

    /// Infer expression types.
    #[task(code = 3, trace = "module={module} profile={profile}")]
    AnalyzeModuleInfer {
        module: ModuleId,
        profile: ProfileId,
    },

    /// Final validation pass.
    #[task(code = 4, trace = "module={module} profile={profile}")]
    AnalyzeModuleValidate {
        module: ModuleId,
        profile: ProfileId,
    },
}

impl Compiler {
    /// Process an analyze task.
    pub fn process_analyze(&self, task: AnalyzeTask) -> AnalyzeResult<()> {
        match task {
            AnalyzeTask::AnalyzeModule { module, profile } => {
                self.require_analyze_module_validate(module, profile)?;
            }
            AnalyzeTask::AnalyzeModuleDeclare { module, profile } => {
                self.require_resolve_module_canonical(module, profile)?;
                self.analyze_module_declare(module, profile)?;
            }
            AnalyzeTask::AnalyzeModuleInfer { module, profile } => {
                self.require_analyze_module_declare(module, profile)?;
                self.analyze_module_infer(module, profile)?;
            }
            AnalyzeTask::AnalyzeModuleValidate { module, profile } => {
                self.require_analyze_module_infer(module, profile)?;
                self.analyze_module_validate(module, profile)?;
                self.stats.record_analyze();
            }
        }
        Ok(())
    }
}
