use std::collections::BTreeMap;
use std::sync::Arc;

use destack_heap as heap;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::trace::TraceImage;

use super::{Image, ImageId, ROOT_IMAGE_ID, ROOT_REVISION_ID, RevisionId};

/// First allocated image identifier after the root image.
const INITIAL_IMAGE_ID: u128 = 1;

/// Retained image ownership for one world lineage family.
#[derive(Debug)]
pub(crate) struct ImageStore {
    /// The next image identifier to allocate.
    pub next_image_id: u128,
    /// The known image payloads keyed by image identifier.
    pub images: BTreeMap<ImageId, Arc<Image>>,
    /// The known trace image payloads keyed by revision identifier.
    pub trace_images: BTreeMap<RevisionId, Arc<TraceImage>>,
}

/// Durable image-store payload captured in one world snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageStoreSnapshot {
    /// The serialized arena pages reachable from this image store.
    pub arena: heap::ArenaSnapshot,
    /// The next image identifier to allocate.
    pub next_image_id: u128,
    /// The known image payloads keyed by image identifier.
    pub images: BTreeMap<ImageId, Image>,
    /// The known trace image payloads keyed by revision identifier.
    pub trace_images: BTreeMap<RevisionId, TraceImage>,
}

impl ImageStore {
    /// Create one image store with one fully materialized root image.
    pub(crate) fn new_root(image: Arc<Image>, trace_image: Arc<TraceImage>) -> Self {
        let mut images = BTreeMap::new();
        let mut trace_images = BTreeMap::new();

        images.insert(ROOT_IMAGE_ID, image);
        trace_images.insert(ROOT_REVISION_ID, trace_image);

        Self {
            next_image_id: INITIAL_IMAGE_ID,
            images,
            trace_images,
        }
    }

    /// Capture one durable image-store snapshot.
    pub(crate) fn snapshot(&self, arena: &Arc<heap::Arena>) -> ImageStoreSnapshot {
        let mut reachable_pages = Vec::new();

        for image in self.images.values() {
            reachable_pages.extend(image.shared.page_ids());

            for agent in image.agents.values() {
                reachable_pages.extend(agent.heap_image.page_ids());
            }
        }

        reachable_pages.sort_unstable();
        reachable_pages.dedup();

        let images = self
            .images
            .iter()
            .map(|(image_id, image)| (*image_id, image.as_ref().clone()))
            .collect();
        let trace_images = self
            .trace_images
            .iter()
            .map(|(revision_id, trace_image)| (*revision_id, trace_image.as_ref().clone()))
            .collect();

        ImageStoreSnapshot {
            arena: arena.snapshot_pages_from_ids(&reachable_pages),
            next_image_id: self.next_image_id,
            images,
            trace_images,
        }
    }

    /// Rebuild one image store from one durable snapshot.
    pub(crate) fn from_snapshot(snapshot: ImageStoreSnapshot) -> (Arc<heap::Arena>, Self) {
        let arena = Arc::new(heap::Arena::from_snapshot(&snapshot.arena));
        let images = snapshot
            .images
            .into_iter()
            .map(|(image_id, mut image)| {
                for agent in image.agents.values_mut() {
                    agent.heap_image = agent.heap_image.with_arena(arena.clone());
                }

                (image_id, Arc::new(image))
            })
            .collect();
        let trace_images = snapshot
            .trace_images
            .into_iter()
            .map(|(revision_id, trace_image)| (revision_id, Arc::new(trace_image)))
            .collect();

        (
            arena,
            Self {
                next_image_id: snapshot.next_image_id,
                images,
                trace_images,
            },
        )
    }

    /// Allocate one new image identifier.
    pub(crate) fn allocate_image_id(&mut self) -> ImageId {
        let image_id = ImageId::new(self.next_image_id);
        self.next_image_id += 1;
        image_id
    }

    /// Insert one retained image payload.
    pub(crate) fn insert_image(&mut self, image: Arc<Image>) {
        self.images.insert(image.id, image);
    }

    /// Insert one retained trace image payload.
    pub(crate) fn insert_trace_image(
        &mut self,
        revision_id: RevisionId,
        trace_image: Arc<TraceImage>,
    ) {
        self.trace_images.insert(revision_id, trace_image);
    }

    /// Return one retained image payload.
    pub(crate) fn image(&self, image_id: ImageId) -> RuntimeResult<Arc<Image>> {
        self.images.get(&image_id).cloned().ok_or_else(|| {
            RuntimeError::ImageNotFound {
                image_id: image_id.get(),
            }
            .boxed()
        })
    }

    /// Return one retained trace image payload.
    pub(crate) fn trace_image(&self, revision_id: RevisionId) -> RuntimeResult<Arc<TraceImage>> {
        self.trace_images.get(&revision_id).cloned().ok_or_else(|| {
            RuntimeError::RevisionTraceImageMissing {
                revision_id: revision_id.get(),
            }
            .boxed()
        })
    }

    /// Return whether one retained image payload exists.
    pub(crate) fn contains_image(&self, image_id: ImageId) -> bool {
        self.images.contains_key(&image_id)
    }

    /// Return identifiers for all retained images in stable order.
    pub(crate) fn image_ids(&self) -> Vec<ImageId> {
        self.images.keys().copied().collect()
    }
}
