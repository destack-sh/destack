use crate::{Compiler, EmitError, EmitResult};

use destack_compiler_macros::DefineTask;
use destack_source::{ModuleStamp, PackageId, PackageStamp};
use destack_workspace::{ProgramStamp, TargetId};

/// Task to emit compiled output to disk.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Emit)]
pub enum EmitTask {
    /// Emit a single module's output for a target.
    #[task(code = 1, trace = "module={module} target={target}")]
    EmitModule {
        /// The module stamp to emit.
        module: ModuleStamp,
        /// The package stamp to emit.
        package: PackageStamp,
        /// The target name.
        target: TargetId,
    },
    /// Emit all outputs for a package target.
    #[task(code = 2, trace = "package={package} target={target}")]
    EmitPackage {
        /// The package to emit.
        package: PackageStamp,
        /// The target name.
        target: TargetId,
    },
    /// Emit all outputs for the entire program.
    #[task(code = 3, trace = "target={target}")]
    EmitProgram {
        /// The target name.
        target: TargetId,
        /// Stamp of the current package versions.
        program_stamp: ProgramStamp,
    },
}

impl Compiler {
    /// Process an emit task.
    pub fn process_emit(&self, task: EmitTask) -> EmitResult<()> {
        match task {
            EmitTask::EmitModule {
                module,
                package,
                target,
            } => {
                self.ensure_module_version_matches::<EmitError>(module.id, module.version)?;
                let module_package = self.program.modules.get(module.id).read().package_id;
                if module_package != package.id {
                    return Ok(());
                }
                self.ensure_package_version_matches::<EmitError>(package.id, package.version)?;
                self.emit_module(module.id, &target)
            }
            EmitTask::EmitPackage { package, target } => {
                self.ensure_package_version_matches::<EmitError>(package.id, package.version)?;
                self.emit_package(package.id, &target)
            }
            EmitTask::EmitProgram {
                target,
                program_stamp,
            } => {
                self.ensure_program_stamp_matches::<EmitError>(program_stamp)?;
                self.emit_program(&target)
            }
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
