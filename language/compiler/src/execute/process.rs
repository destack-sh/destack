use crate::timing::tags;
use crate::{Compiler, ExecuteError, ExecuteResult, TaskDependencyError};

use destack_compiler_macros::DefineTask;
use destack_dir::GlobalNodeIdAny;
use destack_source::{ModuleId, ModuleStamp, ProfileStamp};
use destack_workspace::ProfileId;

/// Task to execute comptime code.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Execute)]
pub enum ExecuteTask {
    /// Prepare comptime state for a module.
    #[task(code = 1, trace = "module={module} profile={profile}")]
    ExecuteModulePrepare {
        /// The module stamp to execute.
        module: ModuleStamp,
        /// The profile stamp to execute.
        profile: ProfileStamp,
    },

    /// Execute comptime code for a module.
    #[task(code = 2, trace = "module={module} profile={profile}")]
    ExecuteModulePatch {
        /// The module stamp to execute.
        module: ModuleStamp,
        /// The profile stamp to execute.
        profile: ProfileStamp,
    },

    /// Execute comptime code for a specific expression.
    #[task(
        code = 3,
        trace = "module={module} profile={profile} expression={expression}"
    )]
    ExecuteExpression {
        /// The module stamp to execute.
        module: ModuleStamp,
        /// The profile stamp to execute.
        profile: ProfileStamp,
        /// Identify the comptime expression to execute.
        expression: GlobalNodeIdAny,
    },
}

impl Compiler {
    /// Process an execute task.
    pub fn process_execute(&self, task: ExecuteTask) -> ExecuteResult<()> {
        match task {
            ExecuteTask::ExecuteModulePrepare { module, profile } => {
                self.ensure_module_profile_matches::<ExecuteError>(
                    module.id,
                    module.version,
                    profile.id,
                    profile.version,
                )?;
                let _timing = self.timing_scope(tags::EXECUTE_MODULE_PREPARE);
                self.execute_module_prepare(
                    module.id,
                    profile.id,
                    module.version,
                    profile.version,
                )?;
            }
            ExecuteTask::ExecuteModulePatch { module, profile } => {
                self.ensure_module_profile_matches::<ExecuteError>(
                    module.id,
                    module.version,
                    profile.id,
                    profile.version,
                )?;
                let _timing = self.timing_scope(tags::EXECUTE_MODULE_PATCH);
                self.execute_module_patch(module.id, profile.id, module.version, profile.version)?;
                if self.is_code_module(module.id) {
                    self.stats.record_execute();
                }
            }
            ExecuteTask::ExecuteExpression {
                module,
                profile,
                expression,
            } => {
                self.ensure_module_profile_matches::<ExecuteError>(
                    module.id,
                    module.version,
                    profile.id,
                    profile.version,
                )?;
                let _timing = self.timing_scope(tags::EXECUTE_EXPRESSION);
                self.execute_expression(
                    module.id,
                    profile.id,
                    module.version,
                    profile.version,
                    expression,
                )?;
            }
        }
        Ok(())
    }

    /// Ensure a module's comptime state has been prepared.
    pub fn require_execute_module_prepare(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        let module = self.module_stamp(module);
        let profile = self.profile_stamp(profile);
        self.do_require_task_internal_only(ExecuteTask::ExecuteModulePrepare { module, profile })
    }

    /// Ensure a module's comptime code has been executed.
    pub fn require_execute_module_patch(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        let module = self.module_stamp(module);
        let profile = self.profile_stamp(profile);
        self.do_require_task_internal_only(ExecuteTask::ExecuteModulePatch { module, profile })
    }

    /// Ensure a comptime expression has been executed.
    pub fn require_execute_expression(
        &self,
        module: ModuleId,
        profile: ProfileId,
        expression: GlobalNodeIdAny,
    ) -> Result<(), TaskDependencyError> {
        let module = self.module_stamp(module);
        let profile = self.profile_stamp(profile);
        self.do_require_task_internal_only(ExecuteTask::ExecuteExpression {
            module,
            profile,
            expression,
        })
    }
}
