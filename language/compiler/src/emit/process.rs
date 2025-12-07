use std::path::PathBuf;

use crate::{Compiler, EmitResult, Task, TaskDebug, TaskOutput};

use destack_source::{ModuleId, PackageId};
use destack_workspace::{ArtifactId, Program};

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

/// Record of a single artifact written during emit.
#[derive(Debug, Clone, PartialEq)]
pub struct EmittedArtifact {
    /// The artifact that was emitted.
    pub artifact: ArtifactId,
    /// The output path where the artifact was written.
    pub path: PathBuf,
    /// Size of the written content in bytes.
    pub size: usize,
}

/// Output of an emit task.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EmitOutput {
    /// Artifacts that were written.
    pub artifacts: Vec<EmittedArtifact>,
}

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
}
