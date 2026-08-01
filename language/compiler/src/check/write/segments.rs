use std::sync::Arc;

use destack_artifact::{
    ArtifactProjectionFingerprint, DiagnosticControlTable, DirChecked, DirDeclared,
};
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::CheckModuleState;
use crate::{CompilerError, CompilerResult};

/// Solved check segments for one module.
pub(in crate::check) struct CheckModuleSegments {
    /// Checked binding segment.
    pub(super) bindings: dir::BindingSegment,
    /// Checked decorator segment.
    pub(super) decorators: dir::DecoratorSegment,
    /// Checked diagnostic controls.
    pub(super) controls: DiagnosticControlTable,
    /// Checked auto implementation segment.
    pub(super) auto: dir::AutoSegment,
    /// Checked type segment.
    pub(super) types: dir::TypeSegment,
    /// Checked static value segment.
    pub(super) statics: dir::StaticSegment,
    /// Checked resolution segment.
    pub(super) resolutions: dir::ResolutionSegment,
    /// Checked generic segment.
    pub(super) generics: dir::GenericSegment,
    /// Checked definition segment.
    pub(super) definitions: dir::DefinitionSegment,
    /// Checked coercion segment.
    pub(super) coercions: dir::CoercionSegment,
    /// Checked capture segment.
    pub(super) captures: dir::CaptureSegment,
}

impl CheckModuleSegments {
    /// Create check segments from one solved module state.
    pub(in crate::check) fn from_state(state: CheckModuleState) -> Self {
        Self {
            bindings: state.bindings_tail,
            decorators: state.decorators,
            controls: state.controls,
            auto: state.auto,
            types: state.types_tail,
            statics: state.statics,
            resolutions: state.resolutions,
            generics: state.generics,
            definitions: state.definitions,
            coercions: state.coercions,
            captures: state.capture_segment,
        }
    }

    /// Convert segments into one checked DIR module.
    pub(in crate::check) fn into_checked(self, module: ModuleId) -> CompilerResult<DirChecked> {
        let fingerprint = ArtifactProjectionFingerprint::from_serialized_payload(&(
            module,
            &self.bindings,
            &self.decorators,
            &self.controls,
            &self.auto,
            &self.types,
            &self.statics,
            &self.resolutions,
            &self.generics,
            &self.definitions,
            &self.coercions,
            &self.captures,
        ))
        .map_err(|error| CompilerError::Internal {
            message: format!("failed to fingerprint DIR payload for module {module:?}: {error}"),
        })?;

        Ok(DirChecked {
            fingerprint,
            bindings: Arc::new(self.bindings),
            decorators: Arc::new(self.decorators),
            controls: Arc::new(self.controls),
            auto: Arc::new(self.auto),
            types: Arc::new(self.types),
            statics: Arc::new(self.statics),
            resolutions: Arc::new(self.resolutions),
            generics: Arc::new(self.generics),
            definitions: Arc::new(self.definitions),
            coercions: Arc::new(self.coercions),
            captures: Arc::new(self.captures),
        })
    }

    /// Convert segments into one declared DIR module.
    pub(in crate::check) fn into_declared(
        self,
        module: ModuleId,
    ) -> CompilerResult<DirDeclared> {
        let fingerprint = ArtifactProjectionFingerprint::from_serialized_payload(&(
            module,
            &self.bindings,
            &self.decorators,
            &self.types,
            &self.statics,
            &self.generics,
            &self.definitions,
            &self.resolutions,
        ))
        .map_err(|error| CompilerError::Internal {
            message: format!(
                "failed to fingerprint declared DIR payload for module {module:?}: {error}"
            ),
        })?;

        Ok(DirDeclared {
            fingerprint,
            bindings: Arc::new(self.bindings),
            decorators: Arc::new(self.decorators),
            types: Arc::new(self.types),
            statics: Arc::new(self.statics),
            generics: Arc::new(self.generics),
            definitions: Arc::new(self.definitions),
            resolutions: Arc::new(self.resolutions),
        })
    }
}
