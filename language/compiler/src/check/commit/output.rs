use std::sync::Arc;

use destack_artifact::DirCheckedModule;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::CheckModuleState;

/// Checked DIR output for one module.
pub(in crate::check) struct CheckModuleOutput {
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
    /// Checked nominal segment.
    pub(super) nominals: dir::NominalSegment,
    /// Checked relation segment.
    pub(super) relations: dir::RelationSegment,
    /// Checked coercion segment.
    pub(super) coercions: dir::CoercionSegment,
    /// Checked extension segment.
    pub(super) extensions: dir::ExtensionSegment,
    /// Checked layout segment.
    pub(super) layouts: dir::LayoutSegment,
    /// Checked capture segment.
    pub(super) captures: dir::CaptureSegment,
}

impl CheckModuleOutput {
    /// Create checked output segments for one module.
    pub(in crate::check) fn new(module: ModuleId, state: &CheckModuleState) -> Self {
        Self {
            annotations: dir::AnnotationSegment::new(module),
            types: dir::TypeSegment::from_base(&state.expanded.types),
            statics: dir::StaticSegment::from_base(&state.expanded.statics),
            resolutions: dir::ResolutionSegment::new(module),
            generics: dir::GenericSegment::new(module),
            nominals: dir::NominalSegment::new(module),
            relations: dir::RelationSegment::new(module),
            coercions: dir::CoercionSegment::new(module),
            extensions: dir::ExtensionSegment::new(module),
            layouts: dir::LayoutSegment::new(module),
            captures: dir::CaptureSegment::new(module),
        }
    }

    /// Convert checked output segments into the artifact payload.
    pub(in crate::check) fn finish(self) -> DirCheckedModule {
        DirCheckedModule {
            annotations: Arc::new(self.annotations),
            types: Arc::new(self.types),
            statics: Arc::new(self.statics),
            resolutions: Arc::new(self.resolutions),
            generics: Arc::new(self.generics),
            nominals: Arc::new(self.nominals),
            relations: Arc::new(self.relations),
            coercions: Arc::new(self.coercions),
            extensions: Arc::new(self.extensions),
            layouts: Arc::new(self.layouts),
            captures: Arc::new(self.captures),
        }
    }
}
