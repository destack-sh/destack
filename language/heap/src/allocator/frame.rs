use super::platform::{self, PageFrame, PageStoreHandle};
use crate::HeapResult;

/// The allocator for page frames that back mapped heap pages.
#[derive(Debug)]
pub(super) struct PageFrameAllocator {
    /// The platform page-store handle.
    handle: PageStoreHandle,
}

impl PageFrameAllocator {
    /// Create one empty page-frame allocator.
    pub(super) fn new(byte_len: usize) -> HeapResult<Self> {
        Ok(Self {
            handle: platform::create_page_store(byte_len)?,
        })
    }

    /// Allocate one zeroed page frame.
    pub(super) fn allocate(&self, frame_bytes: usize) -> HeapResult<PageFrame> {
        platform::allocate_frame(&self.handle, frame_bytes)
    }

    /// Copy one visible page into a new page frame.
    pub(super) fn copy(&self, source: *mut u8, frame_bytes: usize) -> HeapResult<PageFrame> {
        platform::copy_page(&self.handle, source, frame_bytes)
    }

    /// Map one page frame into one virtual page.
    pub(super) fn map(
        &self,
        base: *mut u8,
        page_index: usize,
        page_bytes: usize,
        frame: PageFrame,
    ) -> HeapResult<()> {
        platform::map_page(base, page_index, page_bytes, &self.handle, frame)
    }
}
