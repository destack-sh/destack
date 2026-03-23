use crate::{Compiler, EmitError, EmitResult};

use destack_source::PackageId;
use destack_workspace::TargetId;

impl Compiler {
    /// Emit all outputs for a package target.
    pub fn emit_package(&self, package_id: PackageId, target_id: &TargetId) -> EmitResult<()> {
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

        // ensure linking is complete
        self.require_package_output(package_id, target_id)?;

        // get the package output artifact
        let emit = self
            .artifacts
            .package_output(package_id, target_id)
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
