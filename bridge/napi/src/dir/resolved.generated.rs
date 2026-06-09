// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

use crate::{ArtifactVersion, ModuleId, ProfileId};

/// Typed projection of one resolved DIR artifact.
#[derive(Debug)]
#[napi(object)]
pub struct DirResolved {
    /// Exact resolved artifact version.
    pub version: ArtifactVersion,
    /// Resolved module id.
    pub module: ModuleId,
    /// Resolved semantic profile.
    pub profile: ProfileId,
}

impl DirResolved {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::DirResolved) -> Self {
        Self {
            version: ArtifactVersion::from_bridge(value.version),
            module: ModuleId::from_bridge(value.module),
            profile: ProfileId::from_bridge(value.profile),
        }
    }
}
