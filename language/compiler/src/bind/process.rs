use destack_source::ModuleId;

use crate::{BindResult, Compiler, Task, TaskDebug, TaskDependencyError, TaskOutput};

use destack_workspace::{Module, Program};

/// Task to bind AST into DIR.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum BindTask {
    /// Build DIR for a module by binding its AST.
    BindModuleBuild { module: ModuleId },
    /// Validate module semantics that depend on structural context (but not types).
    BindModuleValidate { module: ModuleId },
}

impl BindTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            BindTask::BindModuleBuild { .. } => 1,
            BindTask::BindModuleValidate { .. } => 2,
        }
    }
}

impl TaskDebug for BindTask {
    fn name(&self) -> &'static str {
        match self {
            BindTask::BindModuleBuild { .. } => "module_build",
            BindTask::BindModuleValidate { .. } => "module_validate",
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            BindTask::BindModuleBuild { module } | BindTask::BindModuleValidate { module } => {
                let module = program.modules.get(*module);
                let uri = module.read().uri.clone().to_string();
                format!(r#"module="{uri}""#)
            }
        }
    }
}

impl From<BindTask> for Task {
    fn from(task: BindTask) -> Self {
        Task::Bind(task)
    }
}

/// Output of a bind task.
#[derive(Debug, Clone, PartialEq)]
pub struct BindOutput {}

impl From<BindOutput> for TaskOutput {
    fn from(output: BindOutput) -> Self {
        TaskOutput::Bind(output)
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Process a bind task.
    pub fn process_bind(&self, task: BindTask) -> BindResult<BindOutput> {
        match task {
            BindTask::BindModuleBuild { module } => {
                self.require_import_module(module)?;
                let module = self.program.modules.get(module);
                let mut module = module.write();
                self.bind_module_build(&mut module);
            }
            BindTask::BindModuleValidate { module } => {
                self.require_bind_module_build(module)?;
                let module = self.program.modules.get(module);
                let module = module.read();
                self.bind_module_validate(&module);
            }
        }
        Ok(BindOutput {})
    }

    /// Ensure a module has been bound (DIR built).
    pub fn require_bind_module_build(&self, module: ModuleId) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(BindTask::BindModuleBuild { module })
    }

    /// Ensure a module has been validated after binding.
    pub fn require_bind_module_validate(
        &self,
        module: ModuleId,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(BindTask::BindModuleValidate { module })
    }

    /// Bind a module and build its DIR.
    fn bind_module_build(&self, module: &mut Module) {
        // bind AST into DIR
        self.bind_module_roots(module);

        // bind module exports
        self.bind_module_exports(module);
    }

    /// Validate a module after binding into DIR.
    fn bind_module_validate(&self, module: &Module) {
        self.validate_binding_names(module);
        self.validate_binding_conflicts(module);
        self.validate_flow(module);
    }
}
