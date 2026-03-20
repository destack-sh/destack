use crate::{Compiler, EmitError, EmitResult};

use destack_source::ModuleId;
use destack_workspace::TargetId;

impl Compiler {
    /// Emit a single module's output for a target.
    pub fn emit_module(&self, module_id: ModuleId, target_id: &TargetId) -> EmitResult<()> {
        // get the package for this module
        let module = self.program.modules.get(module_id);
        let module = module.as_ref();
        let package_id = module.package_id;
        // verify target exists
        let has_target = {
            let package_ref = self.program.packages.get(package_id);
            let package = package_ref.read();
            package.targets.contains_key(target_id)
        };
        if !has_target {
            return Err(EmitError::TargetNotFound {
                package: package_id,
                target: target_id.clone(),
            });
        }

        // honor noEmit configuration
        if self
            .program
            .packages
            .get(package_id)
            .read()
            .config
            .as_ref()
            .is_some_and(|config| config.options.compiler.no_emit)
        {
            return Err(EmitError::NoEmit {
                package: package_id,
                target: target_id.clone(),
            });
        }

        // ensure generation is complete
        self.require_module_output(
            module_id,
            self.program
                .profile_id_for_target(module_id, target_id)
                .ok_or_else(|| EmitError::TargetNotFound {
                    package: package_id,
                    target: target_id.clone(),
                })?,
            target_id,
        )?;

        // get output entries for this module + target
        let emit = self
            .program
            .artifacts
            .module_output(module_id, target_id)
            .ok_or_else(|| EmitError::TargetNotFound {
                package: package_id,
                target: target_id.clone(),
            })?;

        // emit each file using its precomputed output path
        for entry in &emit.entries {
            let output_path =
                entry
                    .uri
                    .to_path_buf()
                    .ok_or_else(|| EmitError::InvalidOutputPath {
                        uri: entry.uri.clone(),
                    })?;

            self.write_output_entry(entry, &output_path)?;
        }

        Ok(())
    }
}
