use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::allocator::Allocator;
use super::{PageId, PageView};
use crate::{HeapError, HeapResult};

/// One serialized allocator page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllocatorPageImage {
    /// The page identifier inside the serialized allocator.
    pub id: PageId,
    /// The exact bytes for this page.
    pub bytes: Box<[u8]>,
}

/// One serialized allocator image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllocatorImage {
    /// The fixed page width for the image.
    pub page_bytes: u32,
    /// The fixed segment width for the image.
    pub segment_bytes: u32,

    /// The serialized page leaves reachable from one frozen root.
    pub pages: Box<[AllocatorPageImage]>,
}

impl Allocator {
    /// Restore one allocator directly from one serialized image.
    pub fn from_image(image: &AllocatorImage) -> HeapResult<Self> {
        let allocator = Self::try_new(image.page_bytes as usize, image.segment_bytes as usize)?;
        let page_bytes = allocator.page_bytes();
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

        // reserve enough segment capacity first
        allocator.ensure_page_capacity(page_count)?;

        // materialize every serialized page into the allocator
        for page in &image.pages {
            if page.bytes.len() != page_bytes {
                return Err(HeapError::ImageInvalidPageBytes {
                    page_id: page.id,
                    expected: page_bytes,
                    actual: page.bytes.len(),
                });
            }

            let target_page_ptr = allocator
                .page_slice_mut_ptr(page.id)
                .map_err(|_| HeapError::ImageMissingPage { page_id: page.id })?;
            let target_page = unsafe { &mut *target_page_ptr };

            target_page.copy_from_slice(&page.bytes);

            let (segment_index, segment_page_index) = allocator.page_position(page.id);
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
            if !allocator.has_segment(segment_index) {
                let first_page_index = segment_index
                    .checked_mul(allocator.pages_per_segment())
                    .ok_or(HeapError::InvalidPageId {
                        index: segment_index,
                    })?;

                return Err(HeapError::ImageMissingPage {
                    page_id: PageId::new(first_page_index)?,
                });
            }

            allocator.raise_segment_high_watermark(segment_index, high_watermark)?;
        }

        Ok(allocator)
    }

    /// Capture one page view into serialized allocator pages.
    pub fn capture_page_view_pages(
        &self,
        page_view: &PageView,
    ) -> HeapResult<Box<[AllocatorPageImage]>> {
        let mut pages = Vec::with_capacity(page_view.len());

        // capture the exact bytes for each reachable allocator page
        for page in page_view.page_ids() {
            let bytes = self
                .page_slice(page)
                .map_err(|_| HeapError::ImageMissingPage { page_id: page })?
                .to_vec()
                .into_boxed_slice();

            pages.push(AllocatorPageImage { id: page, bytes });
        }

        Ok(pages.into_boxed_slice())
    }

    /// Capture one arbitrary page-id set into one serialized allocator image.
    pub fn image_pages_from_ids(&self, pages: &[PageId]) -> HeapResult<AllocatorImage> {
        let mut image_pages = Vec::with_capacity(pages.len());

        // capture the exact bytes for each explicit allocator page
        for page in pages {
            let bytes = self
                .page_slice(*page)
                .map_err(|_| HeapError::ImageMissingPage { page_id: *page })?
                .to_vec()
                .into_boxed_slice();

            image_pages.push(AllocatorPageImage { id: *page, bytes });
        }

        Ok(AllocatorImage {
            page_bytes: self.page_bytes() as u32,
            segment_bytes: self.segment_bytes() as u32,
            pages: image_pages.into_boxed_slice(),
        })
    }

    /// Return one serialized allocator image for one reachable page view.
    pub fn image(&self, page_view: &PageView) -> HeapResult<AllocatorImage> {
        Ok(AllocatorImage {
            page_bytes: self.page_bytes() as u32,
            segment_bytes: self.segment_bytes() as u32,
            pages: self.capture_page_view_pages(page_view)?,
        })
    }
}
