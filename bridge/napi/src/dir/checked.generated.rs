// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

use crate::{ArtifactVersion, ComponentId, ModuleId, ProfileId};

/// Typed projection of one checked DIR module artifact.
#[derive(Debug)]
#[napi(object)]
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
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::DirChecked) -> Self {
        Self {
            version: ArtifactVersion::from_bridge(value.version),
            module: ModuleId::from_bridge(value.module),
            profile: ProfileId::from_bridge(value.profile),
            component: ComponentId::from_bridge(value.component),
            entry: ModuleId::from_bridge(value.entry),
        }
    }
}
