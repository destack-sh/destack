use std::sync::Arc;

use destack_artifact::DirCheckedModule;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::CheckModuleState;

/// Output DIR tables for one module.
pub(in crate::check) struct CheckModuleOutput {
    /// Output annotation segment.
    pub(super) annotations: dir::AnnotationSegment,
    /// Output type segment.
    pub(super) types: dir::TypeSegment,
    /// Output static value segment.
    pub(super) statics: dir::StaticSegment,
    /// Output resolution segment.
    pub(super) resolutions: dir::ResolutionSegment,
    /// Output generic segment.
    pub(super) generics: dir::GenericSegment,
    /// Output definition segment.
    pub(super) definitions: dir::DefinitionSegment,
    /// Output coercion segment.
    pub(super) coercions: dir::CoercionSegment,
    /// Output layout segment.
    pub(super) layouts: dir::LayoutSegment,
    /// Output capture segment.
    pub(super) captures: dir::CaptureSegment,
}

impl CheckModuleOutput {
    /// Create output segments for one module.
    pub(in crate::check) fn new(module: ModuleId, state: &CheckModuleState) -> Self {
        Self {
            annotations: dir::AnnotationSegment::new(module),
            types: dir::TypeSegment::from_base(&state.expanded.types),
            statics: dir::StaticSegment::from_base(&state.expanded.statics),
            resolutions: dir::ResolutionSegment::new(module),
            generics: dir::GenericSegment::new(module),
            definitions: dir::DefinitionSegment::new(module),
            coercions: dir::CoercionSegment::new(module),
            layouts: dir::LayoutSegment::new(module),
            captures: dir::CaptureSegment::new(module),
        }
    }
}

impl From<CheckModuleOutput> for DirCheckedModule {
    /// Convert output segments into the artifact payload.
    fn from(output: CheckModuleOutput) -> Self {
        DirCheckedModule {
            annotations: Arc::new(output.annotations),
            types: Arc::new(output.types),
            statics: Arc::new(output.statics),
            resolutions: Arc::new(output.resolutions),
            generics: Arc::new(output.generics),
            definitions: Arc::new(output.definitions),
            coercions: Arc::new(output.coercions),
            layouts: Arc::new(output.layouts),
            captures: Arc::new(output.captures),
        }
    }
}
