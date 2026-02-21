use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_compiler_macros::DefineTask;
use destack_source::{ModuleStamp, ProfileStamp};
use destack_workspace::ModuleGraphStamp;

/// Task to analyze something.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Analyze)]
pub enum AnalyzeTask {
    /// Analyze a module completely (declare, interface, infer, capture, validate).
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

    /// Infer one module interface surface.
    #[task(code = 3, trace = "module={module} profile={profile}")]
    AnalyzeModuleInterface {
        module: ModuleStamp,
        profile: ProfileStamp,
    },

    /// Infer one strongly connected interface component.
    #[task(code = 7, trace = "module={module} profile={profile} graph={graph}")]
    AnalyzeInterfaceComponent {
        module: ModuleStamp,
        profile: ProfileStamp,
        graph: ModuleGraphStamp,
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
            AnalyzeTask::AnalyzeModuleInterface { module, profile } => {
                self.ensure_module_profile_matches::<AnalyzeError>(
                    module.id,
                    module.version,
                    profile.id,
                    profile.version,
                )?;
                self.analyze_module_interface(
                    module.id,
                    profile.id,
                    module.version,
                    profile.version,
                )?;
            }
            AnalyzeTask::AnalyzeInterfaceComponent {
                module,
                profile,
                graph,
            } => {
                self.ensure_module_profile_matches::<AnalyzeError>(
                    module.id,
                    module.version,
                    profile.id,
                    profile.version,
                )?;
                self.ensure_module_graph_version_matches::<AnalyzeError>(
                    graph.profile_id,
                    graph.version,
                )?;
                self.analyze_interface_component(
                    module.id,
                    profile.id,
                    module.version,
                    profile.version,
                    graph.version,
                )?;
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
