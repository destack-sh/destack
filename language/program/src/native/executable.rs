use destack_source::ContentId;
use serde::{Deserialize, Serialize};

use super::Relocation;

/// Durable native executable program payload.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Executable {
    /// Native text section content.
    pub text: Option<ContentId>,
    /// Native relocation records.
    pub relocations: Vec<Relocation>,
    /// Encoded native stack maps when separated from common layout metadata.
    pub stack_maps: Option<ContentId>,
    /// Encoded native safepoints when separated from common layout metadata.
    pub safepoints: Option<ContentId>,
    /// Encoded native materializations when separated from common layout metadata.
    pub materializations: Option<ContentId>,
}

impl Executable {
    /// Create one native executable.
    pub fn new(
        text: Option<ContentId>,
        relocations: Vec<Relocation>,
        stack_maps: Option<ContentId>,
        safepoints: Option<ContentId>,
        materializations: Option<ContentId>,
    ) -> Self {
        Self {
            text,
            relocations,
            stack_maps,
            safepoints,
            materializations,
        }
    }

    /// Return all content ids referenced by this native executable.
    pub fn content_ids(&self) -> Vec<ContentId> {
        [
            self.text,
            self.stack_maps,
            self.safepoints,
            self.materializations,
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}
