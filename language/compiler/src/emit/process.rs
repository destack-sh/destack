use crate::{Compiler, EmitResult};

use destack_compiler_macros::DefineTask;
use destack_source::{ModuleId, PackageId};

/// Task to emit compiled output to disk.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Emit)]
pub enum EmitTask {
    /// Emit a single module's output for a target.
    #[task(code = 1, trace = "module={module} target={target}")]
    EmitModule {
        /// The module to emit.
        module: ModuleId,
        /// The target name.
        target: String,
    },
    /// Emit all outputs for a package target.
    #[task(code = 2, trace = "package={package} target={target}")]
    EmitPackage {
        /// The package to emit.
        package: PackageId,
        /// The target name.
        target: String,
    },
    /// Emit all outputs for the entire program.
    #[task(code = 3, trace = "target={target}")]
    EmitProgram {
        /// The target name.
        target: String,
    },
}

impl Compiler {
    /// Process an emit task.
    pub fn process_emit(&self, task: EmitTask) -> EmitResult<()> {
        match task {
            EmitTask::EmitModule { module, target } => self.emit_module(module, &target),
            EmitTask::EmitPackage { package, target } => self.emit_package(package, &target),
            EmitTask::EmitProgram { target } => self.emit_program(&target),
        }
    }
}
