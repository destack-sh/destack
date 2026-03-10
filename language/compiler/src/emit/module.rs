use crate::{Compiler, EmitError, EmitResult};

use destack_source::ModuleId;
use destack_workspace::TargetId;

impl Compiler {
    /// Emit a single module's output for a target.
    pub(super) fn emit_module(&self, module_id: ModuleId, target_id: &TargetId) -> EmitResult<()> {
        // get the package for this module
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let package_id = module.package_id;
        drop(module);

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
        if self.is_emit_disabled(package_id) {
            return Err(EmitError::NoEmit {
                package: package_id,
                target: target_id.clone(),
            });
        }

        // ensure linking is complete
        self.require_link_module(package_id, target_id)?;

        // get outputs for this module + target
        let outputs = self
            .program
            .outputs
            .get_by_module_target(module_id, target_id);

        // emit each output using its precomputed output path
        for output in outputs {
            let output_path =
                output
                    .uri
                    .to_path_buf()
                    .ok_or_else(|| EmitError::InvalidOutputPath {
                        output: output.id,
                        uri: output.uri.clone(),
                    })?;

            self.write_output(&output, &output_path)?;
        }

        Ok(())
    }
}
