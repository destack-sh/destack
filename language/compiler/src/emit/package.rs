use crate::{Compiler, EmitError, EmitOutput, EmitResult};

use destack_source::PackageId;

impl Compiler {
    /// Emit all outputs for a package target.
    pub(super) fn emit_package(
        &self,
        package_id: PackageId,
        target_name: &str,
    ) -> EmitResult<EmitOutput> {
        // ensure linking is complete
        self.require_link(package_id, target_name)?;

        // verify target exists
        let has_target = {
            let package_ref = self.program.packages.get(package_id);
            let package = package_ref.read();
            package.targets.contains_key(target_name)
        };
        if !has_target {
            return Err(EmitError::TargetNotFound {
                package: package_id,
                target: target_name.to_string(),
            });
        }

        // get all artifacts for this package + target
        let artifacts = self
            .program
            .artifacts
            .get_by_package_target(package_id, target_name);

        // emit each artifact using its precomputed output path
        let mut output = EmitOutput::default();
        for artifact in artifacts {
            let output_path =
                artifact
                    .uri
                    .to_path_buf()
                    .ok_or_else(|| EmitError::InvalidOutputPath {
                        artifact: artifact.id,
                        uri: artifact.uri.clone(),
                    })?;

            let emitted = self.write_artifact(&artifact, &output_path)?;
            output.artifacts.push(emitted);
        }

        Ok(output)
    }
}
