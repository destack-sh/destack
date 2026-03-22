use crate::timing::tags;
use crate::{ArtifactRequirementError, Compiler, GenerateError, GenerateResult};

use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::{ArtifactKey, ProfileId, TargetId};

impl Compiler {
    /// Build one module output.
    pub fn process_module_output(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> GenerateResult<()> {
        let module_stamp = self.module_stamp(module);
        let profile_stamp = self.profile_stamp(profile);
        self.ensure_module_profile_matches::<GenerateError>(
            module_stamp.id,
            module_stamp.version,
            profile_stamp.id,
            profile_stamp.version,
        )?;
        let artifact_key = ArtifactKey::module_output(module, target.clone());

        // reuse one persisted module output image when available
        if self
            .load_published_artifact(artifact_key.clone(), |compiler| {
                compiler.load_module_output_image(module, module_stamp.version, profile, &target)
            })
            .is_some()
        {
            return Ok(());
        }

        let _timing = self.timing_scope(tags::GENERATE_MODULE);
        self.generate_module(
            module_stamp.id,
            profile_stamp.id,
            module_stamp.version,
            profile_stamp.version,
            &target,
        )?;
        let output = self
            .program
            .artifacts
            .module_output(module, &target)
            .ok_or_else(|| GenerateError::Internal {
                module,
                message: format!("missing module output artifact for target '{target}'"),
            })?;
        self.store_artifact(&artifact_key, output.as_ref(), |compiler, output| {
            compiler.store_module_output_image(module, profile, &target, output)
        });
        self.stats.record_generate();

        Ok(())
    }

    /// Generate code for a module.
    fn generate_module(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
        target_id: &TargetId,
    ) -> GenerateResult<()> {
        // skip stale tasks
        self.ensure_module_profile_matches::<GenerateError>(
            module_id,
            module_version,
            profile,
            profile_version,
        )?;

        // look up target from module's package
        let target = {
            let module = self.program.modules.get(module_id);
            let module = module.as_ref();
            let package = self.program.packages.get(module.package_id);
            let package = package.read();
            package.targets.get(target_id).cloned()
        };

        let target = target.ok_or_else(|| GenerateError::Internal {
            module: module_id,
            message: format!("target '{}' not found", target_id.name),
        })?;

        let resolved_profile = self
            .program
            .profile_id_for_target(module_id, target_id)
            .ok_or_else(|| GenerateError::Internal {
                module: module_id,
                message: format!("profile not found for target '{}'", target_id.name),
            })?;
        if resolved_profile != profile {
            return Ok(());
        }

        self.require_dir_patched(module_id, profile)?;

        // generate through the JavaScript pipeline
        if target.uses_js_generate_pipeline() {
            return self.generate_js(module_id, &target, profile);
        }

        #[cfg(feature = "native-codegen")]
        {
            // otherwise generate through the native pipeline when enabled
            if target.uses_native_generate_pipeline() {
                return self.generate_cranelift(module_id, &target, profile);
            }
        }

        #[cfg(not(feature = "native-codegen"))]
        {
            // otherwise report disabled native generation
            if target.uses_native_generate_pipeline() {
                return Err(GenerateError::Internal {
                    module: module_id,
                    message: format!(
                        "native codegen is disabled: cannot generate output '{:?}' for target '{}'",
                        target.emit, target.name
                    ),
                });
            }
        }

        // otherwise reject the unsupported output kind
        Err(GenerateError::Internal {
            module: module_id,
            message: format!(
                "unsupported output '{:?}' for target '{}'",
                target.emit, target.name
            ),
        })
    }

    /// Require one module output artifact.
    pub fn require_module_output(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Result<(), ArtifactRequirementError> {
        let _ = profile;
        self.require_artifact(ArtifactKey::module_output(module, target.clone()))
    }
}
