use crate::{Compiler, CompilerContext};
use destack_artifact::ModuleOutput;
use destack_source::{ModuleId, TargetId};

use super::EmitError;

impl Compiler {
    /// Emit a single module's output for a target.
    pub fn emit_module(
        &self,
        module_id: ModuleId,
        target_id: &TargetId,
        context: &CompilerContext<'_>,
    ) -> Result<(), EmitError> {
        // current snapshots
        let module = context.module(module_id);
        let package_id = module.package_id;
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

        // ensure generation is complete
        self.require_module_output(
            context.revision(),
            module_id,
            context.profile_id_for_target(module_id, target_id).ok_or(
                EmitError::TargetNotFound {
                    package: package_id,
                    target: *target_id,
                },
            )?,
            target_id,
        )?;

        // get the generated module artifact for this module + target
        let artifact =
            self.module_output(module_id, target_id)
                .ok_or(EmitError::TargetNotFound {
                    package: package_id,
                    target: *target_id,
                })?;

        let package_dir = package
            .path
            .clone()
            .unwrap_or_else(|| self.repository.workspace_root().to_path_buf());
        let root_dir = package_options
            .as_ref()
            .and_then(|config| config.compiler.root_dir.clone());
        let target = package
            .targets
            .get(target_id)
            .cloned()
            .ok_or(EmitError::TargetNotFound {
                package: package_id,
                target: *target_id,
            })?;

        // materialize entries from the generated module artifact
        let entries = match artifact.as_ref() {
            ModuleOutput::Script(script) => self
                .emit_script_artifact_output(
                    module.as_ref(),
                    script,
                    target_id,
                    &target,
                    &package_dir,
                    root_dir.as_deref(),
                    context,
                )
                .map_err(|message| EmitError::Internal {
                    message: format!("failed to emit script artifact: {message}"),
                })?,
            ModuleOutput::Binary(binary) => crate::emit::emit_binary_artifact_files(
                module.as_ref(),
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

            self.emit_output_file(entry, &output_path)?;
        }

        Ok(())
    }
}
