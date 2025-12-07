use crate::{Compiler, EmitError, EmitResult, Task, TaskDebug, TaskDependencyError, TaskOutput};

use destack_source::{ModuleId, PackageId};
use destack_workspace::Program;

/// Task to emit compiled output to disk.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum EmitTask {
    /// Emit a single module's output for a target.
    EmitModule {
        /// The module to emit.
        module: ModuleId,
        /// The target name.
        target: String,
    },
    /// Emit all outputs for a package target.
    EmitPackage {
        /// The package to emit.
        package: PackageId,
        /// The target name.
        target: String,
    },
    /// Emit all outputs for the entire program.
    EmitProgram {
        /// The target name.
        target: String,
    },
}

impl EmitTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::EmitModule { .. } => 1,
            Self::EmitPackage { .. } => 2,
            Self::EmitProgram { .. } => 3,
        }
    }
}

impl TaskDebug for EmitTask {
    fn name(&self) -> &'static str {
        match self {
            Self::EmitModule { .. } => "module",
            Self::EmitPackage { .. } => "package",
            Self::EmitProgram { .. } => "program",
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            Self::EmitModule { module, target } => {
                let module = program.modules.get(*module);
                let uri = module.read().uri.clone().to_string();
                format!(r#"module="{uri}" target="{target}""#)
            }
            Self::EmitPackage { package, target } => {
                let package = program.packages.get(*package);
                let uri = package.read().uri.clone().to_string();
                format!(r#"package="{uri}" target="{target}""#)
            }
            Self::EmitProgram { target } => {
                format!(r#"target="{target}""#)
            }
        }
    }
}

impl From<EmitTask> for Task {
    fn from(task: EmitTask) -> Self {
        Task::Emit(task)
    }
}

/// Output of an emit task.
#[derive(Debug, Clone, PartialEq)]
pub struct EmitOutput {}

impl From<EmitOutput> for TaskOutput {
    fn from(output: EmitOutput) -> Self {
        TaskOutput::Emit(output)
    }
}

impl Compiler {
    /// Process an emit task.
    pub fn process_emit(&self, task: EmitTask) -> EmitResult<EmitOutput> {
        match task {
            EmitTask::EmitModule { module, target } => self.emit_module(module, &target),
            EmitTask::EmitPackage { package, target } => self.emit_package(package, &target),
            EmitTask::EmitProgram { target } => self.emit_program(&target),
        }
    }

    /// Emit a single module's output for a target.
    fn emit_module(&self, module: ModuleId, target: &str) -> EmitResult<EmitOutput> {
        // get the package for this module
        let module_arc = self.program.modules.get(module);
        let module_guard = module_arc.read();
        let package = module_guard.package_id;

        // ensure link is complete
        self.ensure_linked(package, target).map_err(|e| match e {
            TaskDependencyError::NotReady { dependency } => EmitError::Yield { dependency },
            TaskDependencyError::Failed { dependency } => {
                EmitError::UnsatisfiedDependency { dependency }
            }
        })?;

        // NOTE #Incomplete: implement emit_module
        // 1. look up target from package
        // 2. get the generated artifact for this module
        // 3. determine output path from target.out_dir + module path
        // 4. write file to disk
        let _ = (module, target);
        Ok(EmitOutput {})
    }

    /// Emit all outputs for a package target.
    fn emit_package(&self, package: PackageId, target: &str) -> EmitResult<EmitOutput> {
        // ensure link is complete
        self.ensure_linked(package, target).map_err(|e| match e {
            TaskDependencyError::NotReady { dependency } => EmitError::Yield { dependency },
            TaskDependencyError::Failed { dependency } => {
                EmitError::UnsatisfiedDependency { dependency }
            }
        })?;

        // NOTE #Incomplete: implement emit_package
        // 1. look up target from package
        // 2. get all linked artifacts for the package
        // 3. write each file to target.out_dir
        // 4. if target.is_single_file(), write single output to target.out_file
        let _ = (package, target);
        Ok(EmitOutput {})
    }

    /// Emit all outputs for the entire program.
    fn emit_program(&self, target: &str) -> EmitResult<EmitOutput> {
        // NOTE #Incomplete: implement emit_program
        // 1. collect all packages
        // 2. emit each package that has this target
        let _ = target;
        Ok(EmitOutput {})
    }
}
