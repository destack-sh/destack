use crate::{Compiler, EmitError, EmitOutput, EmitResult};

use destack_source::ModuleId;

impl Compiler {
    /// Emit a single module's output for a target.
    pub(super) fn emit_module(
        &self,
        module_id: ModuleId,
        target_name: &str,
    ) -> EmitResult<EmitOutput> {
        // get the package for this module
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let package_id = module.package_id;
        drop(module);

        // ensure linking is complete
        self.ensure_linked(package_id, target_name)?;

        // verify target exists
        let has_target = {
            let package_ref = self.program.packages.get(package_id);
            let package = package_ref.read();
            package.targets.contains_key(target_name)
        };
        if !has_target {
            return Err(EmitError::TargetNotFound {
                node: self.program.root_node_id,
                target: target_name.to_string(),
            });
        }

        // get artifacts for this module + target
        let artifacts = self
            .program
            .artifacts
            .get_by_module_target(module_id, target_name);

        // emit each artifact using its precomputed output path
        let mut output = EmitOutput::default();
        for artifact in artifacts {
            let output_path =
                artifact
                    .uri
                    .to_path_buf()
                    .ok_or_else(|| EmitError::InvalidOutputPath {
                        artifact: artifact.id,
                        node: self.program.root_node_id,
                        uri: artifact.uri.clone(),
                    })?;

            let emitted = self.write_artifact(&artifact, &output_path)?;
            output.artifacts.push(emitted);
        }

        Ok(output)
    }
}
