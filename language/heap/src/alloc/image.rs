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
        let mut segment_high_watermarks = BTreeMap::<usize, usize>::new();
        let mut page_count = 0;

        // resolve the reachable page range up front
        for page in &image.pages {
            let next_page_count =
                page.id
                    .index()
                    .checked_add(1)
                    .ok_or(HeapError::InvalidPageId {
                        index: page.id.index(),
                    })?;
            page_count = page_count.max(next_page_count);
        }

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

            let target_page = arena
                .page_slice_mut(page.id)
                .map_err(|_| HeapError::ImageMissingPage { page_id: page.id })?;

            target_page.copy_from_slice(&page.bytes);

            let (segment_index, segment_page_index) = arena.page_position(page.id);
            let segment_high_watermark = segment_high_watermarks.entry(segment_index).or_default();
            let next_high_watermark =
                segment_page_index
                    .checked_add(1)
                    .ok_or(HeapError::InvalidPageId {
                        index: page.id.index(),
                    })?;
            *segment_high_watermark = (*segment_high_watermark).max(next_high_watermark);
        }

        // restore the per-segment fresh-allocation cursors
        for (segment_index, high_watermark) in segment_high_watermarks {
            if !arena.has_published_segment(segment_index) {
                let first_page_index = segment_index.checked_mul(arena.pages_per_segment()).ok_or(
                    HeapError::InvalidPageId {
                        index: segment_index,
                    },
                )?;

                return Err(HeapError::ImageMissingPage {
                    page_id: PageId::new(first_page_index)?,
                });
            }

            arena.raise_segment_high_watermark(segment_index, high_watermark)?;
        }

        Ok(arena)
    }

    /// Capture one page view into serialized arena pages.
    pub fn capture_page_view_pages(&self, page_view: &PageView) -> HeapResult<Box<[ArenaPage]>> {
        let mut pages = Vec::with_capacity(page_view.len());

        // capture the exact bytes for each reachable arena page
        for page in page_view.page_ids() {
            let bytes = self
                .page_slice(page)
                .map_err(|_| HeapError::ImageMissingPage { page_id: page })?
                .to_vec()
                .into_boxed_slice();

            pages.push(ArenaPage { id: page, bytes });
        }

        Ok(pages.into_boxed_slice())
    }

    /// Capture one arbitrary page-id set into one serialized arena image.
    pub fn image_pages_from_ids(&self, pages: &[PageId]) -> HeapResult<ArenaImage> {
        let mut image_pages = Vec::with_capacity(pages.len());

        // capture the exact bytes for each explicit arena page
        for page in pages {
            let bytes = self
                .page_slice(*page)
                .map_err(|_| HeapError::ImageMissingPage { page_id: *page })?
                .to_vec()
                .into_boxed_slice();

            image_pages.push(ArenaPage { id: *page, bytes });
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
            pages: self.capture_page_view_pages(page_view)?,
        })
    }
}
