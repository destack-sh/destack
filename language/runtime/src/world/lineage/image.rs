use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tspp_core::CaptureMode;
use tspp_serde::Reflect;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::world::topology::LabelSet;
use crate::world::trace::TraceImageIndex;
use crate::world::{Moment, World, WorldImage};

use super::RevisionId;

/// One retained World image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Image {
    /// The retained image identifier.
    pub id: ImageId,
    /// The committed Moment captured by this image.
    pub moment: Moment,
    /// The image name, when explicitly named.
    pub name: Option<String>,
    /// The image labels.
    pub labels: LabelSet,
}

/// Identifier for one revision state image.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct ImageId(u64);

/// One retained Image and its captured World state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ImageEntry {
    /// Public Image metadata.
    pub image: Image,
    /// The Revision anchored by this Image.
    pub revision_id: RevisionId,
    /// Captured World state.
    pub world_image: Arc<WorldImage>,
}

impl ImageEntry {
    /// Create one retained Image entry.
    pub(crate) fn new(image: Image, revision_id: RevisionId, world_image: Arc<WorldImage>) -> Self {
        Self {
            image,
            revision_id,
            world_image,
        }
    }
}

impl World {
    /// Capture one named Image for the active Branch.
    pub fn capture(&mut self, name: impl Into<String>) -> RuntimeResult<Image> {
        let committed = self.commit(CaptureMode::Fork, Some(name.into()))?;
        let image = committed.image.ok_or_else(|| {
            RuntimeError::Internal {
                message: "named image commit did not retain its image".to_string(),
            }
            .boxed()
        })?;
        let sequence = self.state.trace.store().next_sequence();

        let (size_bytes, hash) = World::image_size_and_hash(&committed.world_image)?;
        self.state
            .trace
            .store()
            .record_image_exact(TraceImageIndex {
                image_id: image.id,
                revision_id: committed.revision_id,
                sequence,
                path: TraceImageIndex::memory_path(image.id),
                hash,
                size_bytes,
            })?;

        Ok(image)
    }

    /// Set one label on one retained Image.
    pub fn label_image(
        &self,
        image_id: ImageId,
        key: impl Into<Box<str>>,
        value: impl Into<Box<str>>,
    ) -> RuntimeResult<()> {
        let mut lineage = self.lineage.write();
        let image = lineage
            .images
            .get_mut(&image_id)
            .ok_or_else(|| RuntimeError::image_not_found(image_id.get()).boxed())?;
        image.image.labels.insert(key, value);

        Ok(())
    }
}

impl ImageId {
    /// Create a new image identifier.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw image identifier value.
    pub const fn get(self) -> u64 {
        self.0
    }
}
