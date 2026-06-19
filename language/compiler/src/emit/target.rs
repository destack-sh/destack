use crate::{Compiler, CompilerResult, EmitError};
use destack_artifact::{ArtifactKey, Object, Script};
use destack_repository::{ArtifactReader, ProviderContext, Target};

use destack_repository::ProfileId;
use destack_source::{ModuleId, TargetId};

impl Compiler {
    /// Return the artifact required to emit one structured script.
    pub(super) fn script_input(
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

        Err(EmitError::Internal {
            anchor: (module_id).into(),
            module: module_id,
            message: format!(
                "unsupported emit script '{:?}' for target '{target_id}'",
                target.emit
            ),
        }
        .into())
    }

    /// Return the artifact required to emit one compiled-code object.
    pub(super) fn object_input(
        &self,
        module_id: ModuleId,
        _profile: ProfileId,
        target_id: &TargetId,
        target: &Target,
    ) -> CompilerResult<ArtifactKey> {
        // native emit reads optimized MIR
        #[cfg(feature = "native")]
        {
            if target.uses_native_emit_pipeline() {
                return Ok(ArtifactKey::mir_optimized(module_id, _profile, *target_id));
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
                        "native emit is disabled: cannot emit object '{:?}' for target '{target_id}'",
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
                "unsupported emit object '{:?}' for target '{target_id}'",
                target.emit
            ),
        }
        .into())
    }

    /// Return the artifact required to emit one opaque asset.
    pub(super) fn asset_input(
        &self,
        module_id: ModuleId,
        _profile: ProfileId,
        _target_id: &TargetId,
        _target: &Target,
    ) -> CompilerResult<ArtifactKey> {
        Ok(ArtifactKey::data(module_id))
    }

    /// Emit one structured script for one target.
    pub(super) fn emit_target_script(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        target: &Target,
        target_name: &str,
        context: &dyn ProviderContext,
        artifacts: &ArtifactReader<'_>,
    ) -> CompilerResult<Script> {
        // dispatch through the selected emit family
        if target.uses_js_emit_pipeline() {
            return self.emit_script(module_id, target, profile, context, artifacts);
        }

        Err(EmitError::Internal {
            anchor: (module_id).into(),
            module: module_id,
            message: format!(
                "unsupported emit script '{:?}' for target '{}'",
                target.emit, target_name
            ),
        }
        .into())
    }

    /// Emit one compiled-code object for one target.
    pub(super) fn emit_target_object(
        &self,
        module_id: ModuleId,
        _profile: ProfileId,
        _target_id: &TargetId,
        target: &Target,
        target_name: &str,
        _context: &dyn ProviderContext,
        _artifacts: &ArtifactReader<'_>,
    ) -> CompilerResult<Object> {
        // native emit
        #[cfg(feature = "native")]
        {
            if target.uses_native_emit_pipeline() {
                return self.emit_object(
                    module_id, target, _target_id, _profile, _context, _artifacts,
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
                        "native emit is disabled: cannot emit object '{:?}' for target '{}'",
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
                "unsupported emit object '{:?}' for target '{}'",
                target.emit, target_name
            ),
        }
        .into())
    }
}
