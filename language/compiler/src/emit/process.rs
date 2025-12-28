use crate::{Compiler, EmitResult};

use destack_compiler_macros::DefineTask;
use destack_source::{ModuleId, PackageId};
use destack_workspace::TargetId;

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
        target: TargetId,
    },
    /// Emit all outputs for a package target.
    #[task(code = 2, trace = "package={package} target={target}")]
    EmitPackage {
        /// The package to emit.
        package: PackageId,
        /// The target name.
        target: TargetId,
    },
    /// Emit all outputs for the entire program.
    #[task(code = 3, trace = "target={target}")]
    EmitProgram {
        /// The target name.
        target: TargetId,
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

    /// Check whether emit is disabled for a package.
    pub(crate) fn is_emit_disabled(&self, package_id: PackageId) -> bool {
        let package = self.program.packages.get(package_id);
        let package = package.read();
        package
            .dsconfig
            .as_ref()
            .map(|dsconfig| dsconfig.options.compiler.no_emit)
            .unwrap_or(false)
    }
}
