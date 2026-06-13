use crate::{Compiler, CompilerResult, GenerateError};
use destack_artifact::{ArtifactKey, ModuleOutput};
use destack_repository::{ArtifactReader, ProviderContext, Target};

use destack_repository::ProfileId;
use destack_source::{ModuleId, TargetId};

impl Compiler {
    /// Return the artifact required to generate one module output.
    pub(super) fn module_output_input(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        target_id: &TargetId,
        target: &Target,
    ) -> CompilerResult<ArtifactKey> {
        // JS generation reads checked DIR
        if target.uses_js_generate_pipeline() {
            return Ok(ArtifactKey::dir_checked(module_id, profile));
        }

        // native generation reads optimized MIR
        #[cfg(feature = "native")]
        {
            if target.uses_native_generate_pipeline() {
                return Ok(ArtifactKey::mir_optimized(module_id, profile, *target_id));
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
                        "native codegen is disabled: cannot generate output '{:?}' for target '{target_id}'",
                        target.emit
                    ),
                }
                .into());
            }
        }

        Err(GenerateError::Internal {
            anchor: (module_id).into(),
            module: module_id,
            message: format!(
                "unsupported output '{:?}' for target '{target_id}'",
                target.emit
            ),
        }
        .into())
    }

    /// Generate one module output for one target.
    pub(super) fn generate_target_module_output(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        target_id: &TargetId,
        target: &Target,
        target_name: &str,
        context: &dyn ProviderContext,
        artifacts: &ArtifactReader<'_>,
    ) -> CompilerResult<ModuleOutput> {
        // dispatch through the selected code generation family
        if target.uses_js_generate_pipeline() {
            return self.generate_js_module_output(module_id, target, profile, context, artifacts);
        }

        // native code generation
        #[cfg(feature = "native")]
        {
            if target.uses_native_generate_pipeline() {
                return self.generate_native_module_output(
                    module_id, target, target_id, profile, context, artifacts,
                );
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
