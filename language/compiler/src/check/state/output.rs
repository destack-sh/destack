use destack_artifact::DirExpanded;
use destack_dir as dir;
use destack_source::ModuleId;

/// Checked DIR outputs for one module.
#[derive(Debug)]
pub(in crate::check) struct CheckOutputState {
    /// Checked type segment.
    pub(in crate::check) types: dir::TypeSegment,
    /// Checked static value segment.
    pub(in crate::check) statics: dir::StaticSegment,
    /// Checked resolution segment.
    pub(in crate::check) resolutions: dir::ResolutionSegment,
    /// Checked generic segment.
    pub(in crate::check) generics: dir::GenericSegment,
    /// Checked relation segment.
    pub(in crate::check) relations: dir::RelationSegment,
    /// Checked coercion segment.
    pub(in crate::check) coercions: dir::CoercionSegment,
    /// Checked extension segment.
    pub(in crate::check) extensions: dir::ExtensionSegment,
    /// Checked layout segment.
    pub(in crate::check) layouts: dir::LayoutSegment,
    /// Checked capture segment.
    pub(in crate::check) captures: dir::CaptureSegment,
}

impl CheckOutputState {
    /// Create checked output segments for one module.
    pub(in crate::check) fn new(module: ModuleId, expanded: &DirExpanded) -> Self {
        Self {
            types: dir::TypeSegment::from_base(&expanded.types),
            statics: dir::StaticSegment::from_base(&expanded.statics),
            resolutions: dir::ResolutionSegment::new(module),
            generics: dir::GenericSegment::new(module),
            relations: dir::RelationSegment::new(module),
            coercions: dir::CoercionSegment::new(module),
            extensions: dir::ExtensionSegment::new(module),
            layouts: dir::LayoutSegment::new(module),
            captures: dir::CaptureSegment::new(module),
        }
    }
}
