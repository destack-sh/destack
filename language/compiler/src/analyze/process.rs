use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_compiler_macros::DefineTask;
use destack_source::{ModuleStamp, ProfileStamp};

/// Task to analyze something.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Analyze)]
pub enum AnalyzeTask {
    /// Analyze a module completely (declare, export, infer, capture, validate).
    #[task(code = 1, trace = "module={module} profile={profile}")]
    AnalyzeModule {
        module: ModuleStamp,
        profile: ProfileStamp,
    },

    /// Analyze declarations.
    #[task(code = 2, trace = "module={module} profile={profile}")]
    AnalyzeModuleDeclare {
        module: ModuleStamp,
        profile: ProfileStamp,
    },

    /// Infer exported surface types.
    #[task(code = 3, trace = "module={module} profile={profile}")]
    AnalyzeModuleExport {
        module: ModuleStamp,
        profile: ProfileStamp,
    },

    /// Infer expression types.
    #[task(code = 4, trace = "module={module} profile={profile}")]
    AnalyzeModuleInfer {
        module: ModuleStamp,
        profile: ProfileStamp,
    },

    /// Resolve closure captures.
    #[task(code = 5, trace = "module={module} profile={profile}")]
    AnalyzeModuleCapture {
        module: ModuleStamp,
        profile: ProfileStamp,
    },

    /// Final validation pass.
    #[task(code = 6, trace = "module={module} profile={profile}")]
    AnalyzeModuleValidate {
        module: ModuleStamp,
        profile: ProfileStamp,
    },
}

impl Compiler {
    /// Process an analyze task.
    pub fn process_analyze(&self, task: AnalyzeTask) -> AnalyzeResult<()> {
        match task {
            AnalyzeTask::AnalyzeModule { module, profile } => {
                self.ensure_module_profile_matches::<AnalyzeError>(
                    module.id,
                    module.version,
                    profile.id,
                    profile.version,
                )?;
                self.require_analyze_module_validate(module.id, profile.id)?;
            }
            AnalyzeTask::AnalyzeModuleDeclare { module, profile } => {
                self.ensure_module_profile_matches::<AnalyzeError>(
                    module.id,
                    module.version,
                    profile.id,
                    profile.version,
                )?;
                self.analyze_module_declare(
                    module.id,
                    profile.id,
                    module.version,
                    profile.version,
                )?;
            }
            AnalyzeTask::AnalyzeModuleExport { module, profile } => {
                self.ensure_module_profile_matches::<AnalyzeError>(
                    module.id,
                    module.version,
                    profile.id,
                    profile.version,
                )?;
                self.analyze_module_export(module.id, profile.id, module.version, profile.version)?;
            }
            AnalyzeTask::AnalyzeModuleInfer { module, profile } => {
                self.ensure_module_profile_matches::<AnalyzeError>(
                    module.id,
                    module.version,
                    profile.id,
                    profile.version,
                )?;
                self.analyze_module_infer(module.id, profile.id, module.version, profile.version)?;
            }
            AnalyzeTask::AnalyzeModuleCapture { module, profile } => {
                self.ensure_module_profile_matches::<AnalyzeError>(
                    module.id,
                    module.version,
                    profile.id,
                    profile.version,
                )?;
                self.analyze_module_capture(
                    module.id,
                    profile.id,
                    module.version,
                    profile.version,
                )?;
            }
            AnalyzeTask::AnalyzeModuleValidate { module, profile } => {
                self.ensure_module_profile_matches::<AnalyzeError>(
                    module.id,
                    module.version,
                    profile.id,
                    profile.version,
                )?;
                self.analyze_module_validate(
                    module.id,
                    profile.id,
                    module.version,
                    profile.version,
                )?;
                if self.is_code_module(module.id) {
                    self.stats.record_analyze();
                }
            }
        }
        Ok(())
    }
}
