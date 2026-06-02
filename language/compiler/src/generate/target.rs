use crate::{Compiler, CompilerResult, GenerateError};
use destack_artifact::ModuleOutput;
use destack_workspace::ProviderContext;

use crate::CompilerError;
use destack_source::{ModuleId, TargetId};
use destack_workspace::ProfileId;

impl Compiler {
    /// Generate one module output for one target.
    pub(super) fn generate_target_module_output(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        target_id: &TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ModuleOutput> {
        // look up target from the module package
        let target = self
            .target_or_builtin(context, *target_id)?
            .ok_or_else(|| GenerateError::Internal {
                anchor: (module_id).into(),
                module: module_id,
                message: format!("target '{target_id}' not found"),
            })?;
        let target_name = self.target_name(context.revision(), *target_id)?;

        let resolved_profile =
            self.profile_id_for_target(context.revision(), module_id, target_id)?;
        if resolved_profile != profile {
            return Err(GenerateError::Internal {
                anchor: (module_id).into(),
                module: module_id,
                message: format!(
                    "target '{}' resolved to profile '{resolved_profile:?}', not '{profile:?}'",
                    target_name
                ),
            }
            .into());
        }

        // dispatch through the selected code generation family
        if target.uses_js_generate_pipeline() {
            return self
                .generate_script_module_output(module_id, &target, profile, context)
                .map_err(CompilerError::from);
        }

        // native code generation
        #[cfg(feature = "native")]
        {
            if target.uses_native_generate_pipeline() {
                return self
                    .generate_binary_module_output(module_id, &target, target_id, profile, context)
                    .map_err(CompilerError::from);
            }
        }

        // disabled native code generation
        #[cfg(not(feature = "native"))]
        {
            if target.uses_native_generate_pipeline() {
                return Err(GenerateError::Internal {
                    anchor: (module_id).into(),
                    module: module_id,
                    message: format!(
                        "native codegen is disabled: cannot generate output '{:?}' for target '{}'",
                        target.emit, target_name
                    ),
                }
                .into());
            }
        }

        Err(GenerateError::Internal {
            anchor: (module_id).into(),
            module: module_id,
            message: format!(
                "unsupported output '{:?}' for target '{}'",
                target.emit, target_name
            ),
        }
        .into())
    }
}
