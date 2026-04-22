use std::collections::BTreeSet;

use super::PageView;
use super::allocator::Allocator;
use crate::{HeapError, HeapResult};

impl Allocator {
    /// Return one byte vector for one logical byte range over one page view.
    pub fn read_bytes(&self, page_view: &PageView, byte_len: usize) -> HeapResult<Vec<u8>> {
        self.bytes_to_vec_from(page_view, 0, byte_len)
    }

    /// Fill one caller-provided buffer from one logical byte range.
    pub fn fill_bytes_from(
        &self,
        page_view: &PageView,
        start: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        let mut copied_bytes = 0usize;

        // validate the requested logical byte range
        self.byte_range_end(page_view, start, target.len())?;

        // stream the requested range into the caller buffer
        self.visit_bytes_from(page_view, start, target.len(), |chunk| {
            let chunk_end = copied_bytes + chunk.len();

            target[copied_bytes..chunk_end].copy_from_slice(chunk);
            copied_bytes = chunk_end;
        })?;

        debug_assert_eq!(copied_bytes, target.len());

        Ok(())
    }

    /// Return the exact mapped bytes for one set of logical page views.
    pub fn mapped_bytes_for_page_views<'a>(
        &self,
        page_views: impl IntoIterator<Item = &'a PageView>,
    ) -> u64 {
        let page_count = page_views
            .into_iter()
            .flat_map(PageView::page_ids)
            .collect::<BTreeSet<_>>()
            .len();

        page_count as u64 * self.page_bytes() as u64
    }

    /// Return the exact borrowed bytes for one set of logical page views.
    pub fn borrowed_bytes_for_page_views<'a>(
        &self,
        page_views: impl IntoIterator<Item = &'a PageView>,
    ) -> HeapResult<u64> {
        let mut borrowed_pages = BTreeSet::new();

        // collect the effective shared pages across every logical view
        for page_view in page_views {
            for page_index in 0..page_view.len() {
                let Some(slot) = page_view.slot(page_index) else {
                    return Err(HeapError::MissingLogicalPage { page_index });
                };

                let is_unique = self.run_is_unique(slot.run)?;
                if is_unique {
                    continue;
                }

                let Some(page_id) = slot.run.page(slot.run_page_index) else {
                    return Err(HeapError::InvalidPatchedRun {
                        page_index,
                        page_count: slot.run.len(),
                    });
                };

                borrowed_pages.insert(page_id);
            }
        }

        Ok(borrowed_pages.len() as u64 * self.page_bytes() as u64)
    }

    /// Visit visible byte chunks for one logical byte range starting at one offset.
    pub fn visit_bytes_from(
        &self,
        page_view: &PageView,
        start: usize,
        byte_len: usize,
        mut callback: impl FnMut(&[u8]),
    ) -> HeapResult<()> {
        let end = self.byte_range_end(page_view, start, byte_len)?;
        let page_bytes = self.page_bytes();
        let start_page = start / page_bytes;
        let end_page = end.div_ceil(page_bytes);

        // visit the requested visible page slices in order
        for page_index in start_page..end_page {
            let Some(page_id) = page_view.page(page_index) else {
                return Err(HeapError::MissingLogicalPage { page_index });
            };

            let page_start = page_index * page_bytes;
            let slice_start = start.saturating_sub(page_start).min(page_bytes);
            let slice_end = end.saturating_sub(page_start).min(page_bytes);
            if slice_start >= slice_end {
                continue;
            }

            let page = self.page_slice(page_id)?;
            let chunk = &page[slice_start..slice_end];

            callback(chunk);
        }

        Ok(())
    }

    /// Return one byte vector for one logical byte range starting at one offset.
    pub fn bytes_to_vec_from(
        &self,
        page_view: &PageView,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<Vec<u8>> {
        let mut bytes = Vec::with_capacity(byte_len);

        // materialize the requested range into one owned buffer
        self.visit_bytes_from(page_view, start, byte_len, |chunk| {
            bytes.extend_from_slice(chunk);
        })?;

        Ok(bytes)
    }

    /// Copy one logical byte range from one page view into another.
    pub(crate) fn copy_bytes_between_page_views(
        &self,
        source_page_view: &PageView,
        source_start: usize,
        target_page_view: &mut PageView,
        target_start: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        let page_bytes = self.page_bytes();
        let mut copied_bytes = 0usize;
        let mut scratch = vec![0; page_bytes.min(byte_len)];

        // validate the exact target range before mutating it
        self.byte_range_end(target_page_view, target_start, byte_len)?;

        // stream the copy through one bounded scratch buffer
        while copied_bytes < byte_len {
            let remaining_bytes = byte_len - copied_bytes;
            let chunk_len = remaining_bytes.min(page_bytes);
            let source_offset = source_start + copied_bytes;
            let target_offset = target_start + copied_bytes;
            let chunk = &mut scratch[..chunk_len];

            self.fill_bytes_from(source_page_view, source_offset, chunk)?;
            self.set_bytes(target_page_view, target_offset, chunk)?;
            copied_bytes += chunk_len;
        }

        Ok(())
    }

    /// Return one byte for one logical byte offset.
    pub fn byte_at(&self, page_view: &PageView, byte_len: usize, index: usize) -> Option<u8> {
        if index >= byte_len {
            return None;
        }

        let page_bytes = self.page_bytes();
        let page_index = index / page_bytes;
        let byte_index = index % page_bytes;
        let page = page_view.page(page_index)?;
        let Ok(page) = self.page_slice(page) else {
            return None;
        };

        page.get(byte_index).copied()
    }

    /// Return the exact end offset for one in-bounds byte range.
    pub(super) fn byte_range_end(
        &self,
        page_view: &PageView,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<usize> {
        let capacity =
            page_view
                .len()
                .checked_mul(self.page_bytes())
                .ok_or(HeapError::InvalidByteRange {
                    start,
                    len: byte_len,
                    capacity: usize::MAX,
                })?;

        // allow empty ranges only when the start stays in bounds
        if byte_len == 0 {
            if start <= capacity {
                return Ok(start);
            }

            return Err(HeapError::InvalidByteRange {
                start,
                len: byte_len,
                capacity,
            });
        }

        let Some(end) = start.checked_add(byte_len) else {
            return Err(HeapError::InvalidByteRange {
                start,
                len: byte_len,
                capacity,
            });
        };

        if end > capacity {
            return Err(HeapError::InvalidByteRange {
                start,
                len: byte_len,
                capacity,
            });
        }

        Ok(end)
    }
}
