use crate::{Compiler, CompilerContext, GenerateError, GenerateResult};

use destack_source::{ModuleId, TargetId};
use destack_workspace::ProfileId;

impl Compiler {
    /// Generate one module artifact for one target.
    pub(super) fn generate_target_module_output(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        target_id: &TargetId,
        context: &CompilerContext<'_>,
    ) -> GenerateResult<()> {
        // look up target from the module package
        let target = {
            let module = context.module(module_id);
            let package = context.package(module.package_id);
            package.targets.get(target_id).cloned()
        };

        let target = target.ok_or_else(|| GenerateError::Internal {
            module: module_id,
            message: format!("target '{}' not found", self.target_name(target_id)),
        })?;

        let resolved_profile = context
            .profile_id_for_target(module_id, target_id)
            .ok_or_else(|| GenerateError::Internal {
                module: module_id,
                message: format!(
                    "profile not found for target '{}'",
                    self.target_name(target_id)
                ),
            })?;
        if resolved_profile != profile {
            return Ok(());
        }

        self.require_dir_patched(context.revision(), module_id, profile)?;

        // generate one script artifact through the script pipeline
        if target.uses_js_generate_pipeline() {
            return self.generate_script_module_output(module_id, &target, profile, context);
        }

        #[cfg(feature = "native-codegen")]
        {
            // otherwise generate one binary artifact through the native pipeline
            if target.uses_native_generate_pipeline() {
                return self.generate_binary_module_output(module_id, &target, profile, context);
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
