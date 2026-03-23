use crate::{Compiler, EmitError, EmitResult};

use destack_artifact::ModuleArtifact;
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
        self.require_module_artifact(
            module_id,
            self.program
                .profile_id_for_target(module_id, target_id)
                .ok_or_else(|| EmitError::TargetNotFound {
                    package: package_id,
                    target: target_id.clone(),
                })?,
            target_id,
        )?;

        // get the generated module artifact for this module + target
        let artifact = self
            .artifacts
            .module_artifact(module_id, target_id)
            .ok_or_else(|| EmitError::TargetNotFound {
                package: package_id,
                target: target_id.clone(),
            })?;

        let package = self.program.packages.get(package_id);
        let package = package.read();
        let package_dir = package
            .path
            .clone()
            .unwrap_or_else(|| self.program.cwd.clone());
        let root_dir = package
            .config
            .as_ref()
            .and_then(|config| config.options.compiler.root_dir.clone());
        let target =
            package
                .targets
                .get(target_id)
                .cloned()
                .ok_or_else(|| EmitError::TargetNotFound {
                    package: package_id,
                    target: target_id.clone(),
                })?;

        // materialize entries from the generated module artifact
        let entries = match artifact.as_ref() {
            ModuleArtifact::Script(script) => {
                let module = self.program.modules.get(module_id);
                destack_codegen_js::render_artifact(
                    module.as_ref(),
                    script,
                    &target,
                    &package_dir,
                    root_dir.as_deref(),
                )
                .map_err(|error| EmitError::Internal {
                    message: format!("failed to render script artifact: {error:?}"),
                })?
            }
            ModuleArtifact::Binary(binary) => crate::emit::render_binary_artifact_entries(
                module,
                binary,
                &target,
                &package_dir,
                root_dir.as_deref(),
            )
            .map_err(|error| EmitError::UnsupportedOutput {
                uri: module.uri.clone(),
                file_type: error.file_type,
            })?,
        };

        // emit each file using its precomputed output path
        for entry in &entries {
            let output_path =
                entry
                    .uri
                    .to_path_buf()
                    .ok_or_else(|| EmitError::InvalidOutputPath {
                        uri: entry.uri.clone(),
                    })?;

            self.write_output_file(entry, &output_path)?;
        }

        Ok(())
    }
}
