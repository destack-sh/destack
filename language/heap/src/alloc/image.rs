use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::arena::Arena;
use super::{PageId, PageView};
use crate::{HeapError, HeapResult};

/// One serialized arena page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArenaPage {
    /// The page identifier inside the serialized arena.
    pub id: PageId,
    /// The exact bytes for this page.
    pub bytes: Box<[u8]>,
}

/// One serialized arena image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArenaImage {
    /// The fixed page width for the image.
    pub page_bytes: u32,
    /// The fixed segment width for the image.
    pub segment_bytes: u32,
    /// The serialized page leaves reachable from one frozen root.
    pub pages: Box<[ArenaPage]>,
}

impl Arena {
    /// Restore one arena directly from one serialized image.
    pub fn from_image(image: &ArenaImage) -> HeapResult<Self> {
        let arena = Self::try_new(image.page_bytes as usize, image.segment_bytes as usize)?;
        let page_bytes = arena.page_bytes();
        let page_count = image
            .pages
            .iter()
            .map(|page| page.id.index().saturating_add(1))
            .max()
            .unwrap_or(0);
        let mut segment_high_watermarks = BTreeMap::<usize, usize>::new();

        // publish enough segment capacity first
        arena.ensure_page_capacity(page_count)?;

        // materialize every serialized page into the arena
        for page in &image.pages {
            if page.bytes.len() != page_bytes {
                return Err(HeapError::ImageInvalidPageBytes {
                    page_id: page.id,
                    expected: page_bytes,
                    actual: page.bytes.len(),
                });
            }

            let Some(target_page) = arena.page_bytes_mut(page.id) else {
                return Err(HeapError::ImageMissingPage { page_id: page.id });
            };
            target_page.copy_from_slice(&page.bytes);

            let (segment_index, segment_page_index) = arena.page_position(page.id);
            let segment_high_watermark = segment_high_watermarks.entry(segment_index).or_default();
            *segment_high_watermark =
                (*segment_high_watermark).max(segment_page_index.saturating_add(1));
        }

        // restore the per-segment fresh-allocation cursors
        for (segment_index, high_watermark) in segment_high_watermarks {
            if arena.segment(segment_index).is_none() {
                return Err(HeapError::ImageMissingPage {
                    page_id: PageId::new(segment_index.saturating_mul(arena.pages_per_segment()))?,
                });
            }

            arena.raise_segment_high_watermark(segment_index, high_watermark)?;
        }

        Ok(arena)
    }

    /// Capture one page view into serialized arena pages.
    pub fn image_pages(&self, page_view: &PageView) -> HeapResult<Box<[ArenaPage]>> {
        let mut pages = Vec::with_capacity(page_view.len());

        // capture the exact bytes for each reachable arena page
        for page in page_view.page_ids() {
            let Some(bytes) = self.page_bytes_from_id(page) else {
                return Err(HeapError::ImageMissingPage { page_id: page });
            };

            pages.push(ArenaPage {
                id: page,
                bytes: bytes.to_vec().into_boxed_slice(),
            });
        }

        Ok(pages.into_boxed_slice())
    }

    /// Capture one arbitrary page-id set into one serialized arena image.
    pub fn image_pages_from_ids(&self, pages: &[PageId]) -> HeapResult<ArenaImage> {
        let mut image_pages = Vec::with_capacity(pages.len());

        // capture the exact bytes for each explicit arena page
        for page in pages {
            let Some(bytes) = self.page_bytes_from_id(*page) else {
                return Err(HeapError::ImageMissingPage { page_id: *page });
            };

            image_pages.push(ArenaPage {
                id: *page,
                bytes: bytes.to_vec().into_boxed_slice(),
            });
        }

        Ok(ArenaImage {
            page_bytes: self.page_bytes() as u32,
            segment_bytes: self.segment_bytes() as u32,
            pages: image_pages.into_boxed_slice(),
        })
    }

    /// Return one serialized arena image for one reachable page view.
    pub fn image(&self, page_view: &PageView) -> HeapResult<ArenaImage> {
        Ok(ArenaImage {
            page_bytes: self.page_bytes() as u32,
            segment_bytes: self.segment_bytes() as u32,
            pages: self.image_pages(page_view)?,
        })
    }
}
