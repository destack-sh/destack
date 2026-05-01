use super::PageRun;
use super::allocator::Allocator;
use crate::{HeapError, HeapResult};

impl Allocator {
    /// Return one byte vector for one logical byte range over one page run.
    pub fn read_bytes(&self, page_run: &PageRun, byte_len: usize) -> HeapResult<Vec<u8>> {
        self.bytes_to_vec_from(page_run, 0, byte_len)
    }

    /// Fill one caller-provided buffer from one logical byte range.
    pub fn fill_bytes_from(
        &self,
        page_run: &PageRun,
        start: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        // validate the requested logical byte range
        let end = self.byte_range_end(page_run, start, target.len())?;
        let page_bytes = self.page_bytes();
        let start_page = start / page_bytes;
        let end_page = end.div_ceil(page_bytes);
        let mut copied_bytes = 0usize;

        // copy each visible page slice into the caller buffer
        for page_index in start_page..end_page {
            // resolve the logical page first
            let Some(page_id) = page_run.page(page_index) else {
                return Err(HeapError::MissingLogicalPage { page_index });
            };

            // compute the page-local slice
            let page_start = page_index * page_bytes;
            let slice_start = start.max(page_start) - page_start;
            let slice_end = end.min(page_start + page_bytes) - page_start;
            if slice_start >= slice_end {
                continue;
            }

            // copy the visible page slice
            let page = self.page_bytes_box(page_id)?;
            let chunk = &page[slice_start..slice_end];
            let chunk_end = copied_bytes + chunk.len();

            target[copied_bytes..chunk_end].copy_from_slice(chunk);
            copied_bytes = chunk_end;
        }

        debug_assert_eq!(copied_bytes, target.len());

        Ok(())
    }

    /// Return the exact retained bytes for one set of logical page runs.
    pub fn retained_bytes_for_page_runs<'a>(
        &self,
        page_runs: impl IntoIterator<Item = &'a PageRun>,
    ) -> u64 {
        let page_count = page_runs
            .into_iter()
            .map(|page_run| page_run.len())
            .sum::<usize>();

        page_count as u64 * self.page_bytes() as u64
    }

    /// Return one byte vector for one logical byte range starting at one offset.
    pub fn bytes_to_vec_from(
        &self,
        page_run: &PageRun,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<Vec<u8>> {
        let mut bytes = vec![0; byte_len];

        // materialize the requested range into one owned buffer
        self.fill_bytes_from(page_run, start, &mut bytes)?;

        Ok(bytes)
    }

    /// Overwrite one byte range inside one page run.
    pub(crate) fn write_bytes(
        &self,
        page_run: &PageRun,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        let end = self.byte_range_end(page_run, start, bytes.len())?;
        let page_bytes = self.page_bytes();
        let start_page = start / page_bytes;
        let end_page = end.div_ceil(page_bytes);
        let mut byte_offset = 0usize;

        // copy each page-local caller slice in order
        for page_index in start_page..end_page {
            // resolve the logical page first
            let Some(page_id) = page_run.page(page_index) else {
                return Err(HeapError::MissingLogicalPage { page_index });
            };

            // compute the page-local slice
            let page_start = page_index * page_bytes;
            let slice_start = start.max(page_start) - page_start;
            let slice_end = end.min(page_start + page_bytes) - page_start;
            if slice_start >= slice_end {
                continue;
            }

            // copy the matching caller bytes into an owned image page
            let slice_len = slice_end - slice_start;
            let byte_end = byte_offset + slice_len;
            let mut page = self.page_bytes_box(page_id)?;

            page[slice_start..slice_end].copy_from_slice(&bytes[byte_offset..byte_end]);
            self.store_page_bytes(page_id, page)?;
            byte_offset = byte_end;
        }

        Ok(())
    }

    /// Return the exact end offset for one in-bounds byte range.
    pub(super) fn byte_range_end(
        &self,
        page_run: &PageRun,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<usize> {
        let capacity = page_run.len() * self.page_bytes();

        // validate the range before computing the end offset
        if start > capacity || byte_len > capacity - start {
            return Err(HeapError::InvalidByteRange {
                start,
                len: byte_len,
                capacity,
            });
        }

        let end = start + byte_len;

        Ok(end)
    }
}
