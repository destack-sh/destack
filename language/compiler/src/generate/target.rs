use crate::{Compiler, GenerateError, GenerateResult};

use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::{ProfileId, TargetId};

impl Compiler {
    /// Generate one module artifact for one target.
    pub(super) fn generate_target_module_artifact(
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

        // look up target from the module package
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

        // generate one script artifact through the script pipeline
        if target.uses_js_generate_pipeline() {
            return self.generate_script_module_artifact(module_id, &target, profile);
        }

        #[cfg(feature = "native-codegen")]
        {
            // otherwise generate one binary artifact through the native pipeline
            if target.uses_native_generate_pipeline() {
                return self.generate_binary_module_artifact(module_id, &target, profile);
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
}
