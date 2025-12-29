use crate::{Compiler, ExecuteResult, TaskDependencyError};

use destack_compiler_macros::DefineTask;
use destack_dir::GlobalNodeIdAny;
use destack_source::ModuleId;
use destack_workspace::ProfileId;

/// Task to execute comptime code.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Execute)]
pub enum ExecuteTask {
    /// Prepare comptime state for a module.
    #[task(code = 1, trace = "module={module} profile={profile}")]
    ExecuteModulePrepare {
        /// Identify the module to execute.
        module: ModuleId,
        /// Identify the profile to execute.
        profile: ProfileId,
    },
    /// Execute comptime code for a module.
    #[task(code = 2, trace = "module={module} profile={profile}")]
    ExecuteModulePatch {
        /// Identify the module to execute.
        module: ModuleId,
        /// Identify the profile to execute.
        profile: ProfileId,
    },
    /// Execute comptime code for a specific expression.
    #[task(
        code = 3,
        trace = "module={module} profile={profile} expression={expression}"
    )]
    ExecuteExpression {
        /// Identify the module to execute.
        module: ModuleId,
        /// Identify the profile to execute.
        profile: ProfileId,
        /// Identify the comptime expression to execute.
        expression: GlobalNodeIdAny,
    },
}

impl Compiler {
    /// Process an execute task.
    pub fn process_execute(&self, task: ExecuteTask) -> ExecuteResult<()> {
        match task {
            ExecuteTask::ExecuteModulePrepare { module, profile } => {
                self.execute_module_prepare(module, profile)?;
            }
            ExecuteTask::ExecuteModulePatch { module, profile } => {
                self.execute_module_patch(module, profile)?;
            }
            ExecuteTask::ExecuteExpression {
                module,
                profile,
                expression,
            } => {
                self.execute_expression(module, profile, expression)?;
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
        self.do_require_task_internal_only(ExecuteTask::ExecuteModulePrepare { module, profile })
    }

    /// Ensure a module's comptime code has been executed.
    pub fn require_execute_module_patch(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(ExecuteTask::ExecuteModulePatch { module, profile })
    }

    /// Ensure a comptime expression has been executed.
    pub fn require_execute_expression(
        &self,
        module: ModuleId,
        profile: ProfileId,
        expression: GlobalNodeIdAny,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(ExecuteTask::ExecuteExpression {
            module,
            profile,
            expression,
        })
    }
}
