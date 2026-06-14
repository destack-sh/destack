use crate::{Compiler, CompilerResult, EmitError};
use destack_artifact::{ArtifactKey, ModuleOutput};
use destack_repository::{ArtifactReader, ProviderContext, Target};

use destack_repository::ProfileId;
use destack_source::{ModuleId, TargetId};

impl Compiler {
    /// Return the artifact required to emit one module output.
    pub(super) fn module_output_input(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        target_id: &TargetId,
        target: &Target,
    ) -> CompilerResult<ArtifactKey> {
        // JS emit reads checked DIR
        if target.uses_js_emit_pipeline() {
            return Ok(ArtifactKey::dir_checked(module_id, profile));
        }

        // native emit reads optimized MIR
        #[cfg(feature = "native")]
        {
            if target.uses_native_emit_pipeline() {
                return Ok(ArtifactKey::mir_optimized(module_id, profile, *target_id));
            }
        }

        // disabled native emit
        #[cfg(not(feature = "native"))]
        {
            if target.uses_native_emit_pipeline() {
                return Err(EmitError::Internal {
                    anchor: (module_id).into(),
                    module: module_id,
                    message: format!(
                        "native emit is disabled: cannot emit output '{:?}' for target '{target_id}'",
                        target.emit
                    ),
                }
                .into());
            }
        }

        Err(EmitError::Internal {
            anchor: (module_id).into(),
            module: module_id,
            message: format!(
                "unsupported output '{:?}' for target '{target_id}'",
                target.emit
            ),
        }
        .into())
    }

    /// Emit one module output for one target.
    pub(super) fn emit_target_module_output(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        target_id: &TargetId,
        target: &Target,
        target_name: &str,
        context: &dyn ProviderContext,
        artifacts: &ArtifactReader<'_>,
    ) -> CompilerResult<ModuleOutput> {
        // dispatch through the selected emit family
        if target.uses_js_emit_pipeline() {
            return self.emit_js_module_output(module_id, target, profile, context, artifacts);
        }

        // native emit
        #[cfg(feature = "native")]
        {
            if target.uses_native_emit_pipeline() {
                return self.emit_native_module_output(
                    module_id, target, target_id, profile, context, artifacts,
                );
            }
        }

        // disabled native emit
        #[cfg(not(feature = "native"))]
        {
            if target.uses_native_emit_pipeline() {
                return Err(EmitError::Internal {
                    anchor: (module_id).into(),
                    module: module_id,
                    message: format!(
                        "native emit is disabled: cannot emit output '{:?}' for target '{}'",
                        target.emit, target_name
                    ),
                }
                .into());
            }
        }

        Err(EmitError::Internal {
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
