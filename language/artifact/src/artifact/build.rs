use destack_core::Blob;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{BuildLinkage, BuildProfile};

/// One target-built toolchain payload.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct Build {
    /// The build distribution profile.
    pub profile: BuildProfile,
    /// The build linkage.
    pub linkage: BuildLinkage,
    /// The encoded build bytes.
    pub blob: Blob,
}

impl Build {
    /// Create one build payload.
    pub fn new(profile: BuildProfile, linkage: BuildLinkage, blob: Blob) -> Self {
        Self {
            profile,
            linkage,
            blob,
        }
    }

    /// Return every Blob referenced by this build payload.
    pub fn blobs(&self) -> Vec<Blob> {
        vec![self.blob]
    }
}
