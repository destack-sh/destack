use std::sync::Arc;

use destack_artifact::DirCheckedModule;
use destack_dir as dir;

use crate::check::CheckModuleState;

/// Checked DIR segments for one module.
pub(in crate::check) struct CheckedModuleSegments {
    /// Checked binding segment.
    pub(super) bindings: dir::BindingSegment,
    /// Checked annotation segment.
    pub(super) annotations: dir::AnnotationSegment,
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
    /// Checked layout segment.
    pub(super) layouts: dir::LayoutSegment,
    /// Checked capture segment.
    pub(super) captures: dir::CaptureSegment,
}

impl CheckedModuleSegments {
    /// Create checked segments from one solved module state.
    pub(in crate::check) fn from_state(state: CheckModuleState) -> Self {
        Self {
            bindings: state.bindings_tail,
            annotations: state.annotations,
            types: state.types_tail,
            statics: state.statics,
            resolutions: state.resolutions,
            generics: state.generics,
            definitions: state.definitions,
            coercions: state.coercions,
            layouts: state.layouts,
            captures: state.capture_segment,
        }
    }
}

impl From<CheckedModuleSegments> for DirCheckedModule {
    /// Convert checked segments into the artifact payload.
    fn from(segments: CheckedModuleSegments) -> Self {
        DirCheckedModule {
            bindings: Arc::new(segments.bindings),
            annotations: Arc::new(segments.annotations),
            types: Arc::new(segments.types),
            statics: Arc::new(segments.statics),
            resolutions: Arc::new(segments.resolutions),
            generics: Arc::new(segments.generics),
            definitions: Arc::new(segments.definitions),
            coercions: Arc::new(segments.coercions),
            layouts: Arc::new(segments.layouts),
            captures: Arc::new(segments.captures),
        }
    }
}
