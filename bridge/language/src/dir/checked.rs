use destack_artifact as artifact;

use crate::{ArtifactVersion, ComponentId, ModuleId, ProfileId, bridge};

/// Typed projection of one checked DIR module artifact.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirChecked {
    /// Exact checked facade artifact version.
    pub version: ArtifactVersion,
    /// Checked module id.
    pub module: ModuleId,
    /// Checked semantic profile.
    pub profile: ProfileId,
    /// Component that owns the checked module output.
    pub component: ComponentId,
    /// Component entry module.
    pub entry: ModuleId,
}

impl DirChecked {
    /// Project one checked DIR artifact.
    pub fn from_artifact(
        version: artifact::ArtifactVersion,
        module: ModuleId,
        profile: ProfileId,
        checked: &artifact::DirChecked,
    ) -> Self {
        Self {
            version: ArtifactVersion::from_artifact(version),
            module,
            profile,
            component: checked.component.into(),
            entry: checked.entry.into(),
        }
    }
}
