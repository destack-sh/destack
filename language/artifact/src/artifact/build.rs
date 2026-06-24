use destack_serde::Reflect;
use destack_source::ContentId;
use serde::{Deserialize, Serialize};

use crate::{BuildLinkage, BuildProfile};

/// One target-built toolchain payload.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct Build {
    /// The build distribution profile.
    pub profile: BuildProfile,
    /// The build linkage.
    pub linkage: BuildLinkage,
    /// The encoded build content.
    pub content: ContentId,
}

impl Build {
    /// Create one build payload.
    pub fn new(profile: BuildProfile, linkage: BuildLinkage, content: ContentId) -> Self {
        Self {
            profile,
            linkage,
            content,
        }
    }

    /// Return all content ids referenced by this build payload.
    pub fn content_ids(&self) -> Vec<ContentId> {
        vec![self.content]
    }
}
