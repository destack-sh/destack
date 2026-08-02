use destack_serde::Reflect;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{ArtifactProjectionFingerprint, ArtifactProjectionKey};

/// Content digests of the implicit global modules for one profile.
#[derive(Debug, Clone, Default, Hash, Serialize, Deserialize, Reflect)]
pub struct GlobalEnvironmentDigest {
    /// One digest per implicit module, in stable module order.
    pub modules: Vec<ModuleDigest>,
}

/// The combined stage content digest of one implicit module.
#[derive(Debug, Clone, Hash, Serialize, Deserialize, Reflect)]
pub struct ModuleDigest {
    /// The digested module.
    pub module: ModuleId,
    /// The digest over the bound, expanded, and resolved stage contents.
    pub content: ArtifactProjectionFingerprint,
}

impl GlobalEnvironmentDigest {
    /// Fingerprint one observable projection of this artifact.
    pub(crate) fn fingerprint_projection(
        &self,
        projection: ArtifactProjectionKey,
    ) -> Option<ArtifactProjectionFingerprint> {
        match projection {
            ArtifactProjectionKey::Content => {
                Some(ArtifactProjectionFingerprint::new(&self.modules))
            }
            _ => None,
        }
    }
}
