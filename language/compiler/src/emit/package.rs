use crate::{Compiler, CompilerContext};
use destack_source::{PackageId, TargetId};

use super::EmitError;

impl Compiler {
    /// Emit all outputs for a package target.
    pub fn emit_package(
        &self,
        package_id: PackageId,
        target_id: &TargetId,
        context: &CompilerContext<'_>,
    ) -> Result<(), EmitError> {
        // current snapshot
        let package = context.package(package_id);
        let package_options = context.package_options(package_id);

        // verify target exists
        if !package.targets.contains_key(target_id) {
            return Err(EmitError::TargetNotFound {
                package: package_id,
                target: *target_id,
            });
        }

        // honor noEmit configuration
        if package_options
            .as_ref()
            .is_some_and(|config| config.compiler.no_emit)
        {
            return Err(EmitError::NoEmit {
                package: package_id,
                target: *target_id,
            });
        }

        // ensure linking is complete
        self.require_package_output(context.revision(), package_id, target_id)?;

        // get the package output artifact
        let emit = self
            .package_output(package_id, target_id)
            .ok_or(EmitError::TargetNotFound {
                package: package_id,
                target: *target_id,
            })?;

        // emit each file using its precomputed output path
        for entry in emit.files() {
            let output_path =
                entry
                    .uri
                    .to_path_buf()
                    .ok_or_else(|| EmitError::InvalidOutputPath {
                        uri: entry.uri.clone(),
                    })?;

            self.emit_output_file(entry, &output_path)?;
        }

        Ok(())
    }
}
