use crate::{Compiler, EmitError, EmitResult};

use destack_source::PackageId;
use destack_workspace::TargetId;

impl Compiler {
    /// Emit all outputs for a package target.
    pub(super) fn emit_package(&self, package_id: PackageId, target_id: &TargetId) -> EmitResult<()> {
        // ensure linking is complete
        self.require_link_module(package_id, target_id)?;

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

        // get all artifacts for this package + target
        let artifacts = self
            .program
            .artifacts
            .get_by_package_target(package_id, target_id);

        // emit each artifact using its precomputed output path
        for artifact in artifacts {
            let output_path =
                artifact
                    .uri
                    .to_path_buf()
                    .ok_or_else(|| EmitError::InvalidOutputPath {
                        artifact: artifact.id,
                        uri: artifact.uri.clone(),
                    })?;

            self.write_artifact(&artifact, &output_path)?;
        }

        Ok(())
    }
}
