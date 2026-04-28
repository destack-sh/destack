use std::collections::BTreeMap;
use std::sync::Arc;

use parking_lot::Mutex;

use super::frame::PageFrameAllocator;
use super::platform::{self, PageFrame, VirtualSpace};
use crate::{HeapError, HeapResult};

/// The mapped pages for one forkable heap space.
#[derive(Debug)]
pub(super) struct PageMap {
    /// The reserved virtual byte space.
    space: VirtualSpace,
    /// The reserved byte length.
    byte_len: usize,
    /// The fixed platform page-frame width.
    frame_bytes: usize,
    /// The allocator for page frames backing materialized pages.
    frames: Arc<PageFrameAllocator>,
    /// The materialized page frames keyed by page index.
    pages: Mutex<BTreeMap<usize, PageFrame>>,
}

impl PageMap {
    /// Reserve one forkable page map.
    pub(super) fn reserve(byte_len: usize, page_bytes: usize) -> HeapResult<Self> {
        // use the larger of heap pages and platform pages
        let frame_bytes = page_bytes.max(platform::system_page_bytes()?);
        let page_count = byte_len.div_ceil(frame_bytes);
        let byte_len = page_count * frame_bytes;

        // reserve virtual space and its page frames
        let frames = Arc::new(PageFrameAllocator::new(byte_len)?);
        let space = platform::reserve_virtual_space(byte_len)?;

        Ok(Self {
            space,
            byte_len,
            frame_bytes,
            frames,
            pages: Mutex::new(BTreeMap::new()),
        })
    }

    /// Fork this page map with page-granular isolation.
    pub(super) fn fork(&self) -> HeapResult<Self> {
        // reserve the child address range first
        let space = platform::reserve_virtual_space(self.byte_len)?;
        let fork = Self {
            space,
            byte_len: self.byte_len,
            frame_bytes: self.frame_bytes,
            frames: self.frames.clone(),
            pages: Mutex::new(BTreeMap::new()),
        };

        // native mappings use kernel copy-on-write after fork
        if platform::SUPPORTS_SHARED_PAGE_FRAMES {
            self.fork_shared_frames(&fork)?;

            return Ok(fork);
        }

        // wasm has linear memory, so each fork receives freshly copied pages
        self.fork_copied_frames(&fork)?;

        Ok(fork)
    }

    /// Return the reserved virtual byte length.
    pub(super) const fn byte_len(&self) -> usize {
        self.byte_len
    }

    /// Zero one byte range inside this page map.
    pub(super) fn zero(&self, offset: usize, byte_len: usize) -> HeapResult<()> {
        let target = self.address(offset, byte_len)?;

        // nothing to clear
        if byte_len == 0 {
            return Ok(());
        }

        // clear the validated mapped range
        unsafe {
            std::ptr::write_bytes(target, 0, byte_len);
        }

        Ok(())
    }

    /// Fill one caller-provided buffer from this page map.
    pub(super) fn read(&self, offset: usize, target: &mut [u8]) -> HeapResult<()> {
        let (first_frame, end_frame) = self.frame_range(offset, target.len())?;

        // empty reads only validate the range
        if target.is_empty() {
            return Ok(());
        }

        let mut written = 0;
        let pages = self.pages.lock();

        // copy page by page so sparse pages read back as zero
        for page_index in first_frame..end_frame {
            let page_start = page_index * self.frame_bytes;
            let page_end = page_start + self.frame_bytes;
            let copy_start = offset.max(page_start);
            let copy_end = (offset + target.len()).min(page_end);
            let copy_len = copy_end - copy_start;
            let target_start = copy_start - offset;

            // materialized pages read from their mapping
            if pages.contains_key(&page_index) {
                let source = unsafe { self.space.base().add(copy_start) };
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        source,
                        target[target_start..].as_mut_ptr(),
                        copy_len,
                    );
                }

                written += copy_len;
                continue;
            }

            // sparse pages are logically zero
            target[target_start..target_start + copy_len].fill(0);
            written += copy_len;
        }

        debug_assert_eq!(written, target.len());

        Ok(())
    }

    /// Return one owned byte vector from this page map.
    pub(super) fn bytes(&self, offset: usize, byte_len: usize) -> HeapResult<Vec<u8>> {
        let mut bytes = vec![0; byte_len];

        // fill the owned buffer from the mapped range
        self.read(offset, &mut bytes)?;

        Ok(bytes)
    }

    /// Return one checked address inside this page map.
    pub(super) fn address(&self, offset: usize, byte_len: usize) -> HeapResult<*mut u8> {
        let (first_frame, end_frame) = self.frame_range(offset, byte_len)?;

        // materialize sparse pages before exposing a raw address
        for page_index in first_frame..end_frame {
            self.materialize_page(page_index)?;
        }

        Ok(unsafe { self.space.base().add(offset) })
    }

    /// Write caller-provided bytes directly into this page map.
    pub(super) fn write(&self, offset: usize, bytes: &[u8]) -> HeapResult<()> {
        let target = self.address(offset, bytes.len())?;

        // copy into the validated mapped range
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), target, bytes.len());
        }

        Ok(())
    }

    /// Fork this page map by sharing page frames.
    fn fork_shared_frames(&self, fork: &Self) -> HeapResult<()> {
        let mut pages = self.pages.lock();
        let mut fork_pages = fork.pages.lock();

        for (page_index, page) in &mut *pages {
            // copy the parent-visible bytes into a shareable frame
            let source = unsafe { self.space.base().add(*page_index * self.frame_bytes) };
            let frame = self.frames.copy(source, self.frame_bytes)?;

            // replace the parent page first so parent metadata stays coherent
            self.frames
                .map(self.space.base(), *page_index, self.frame_bytes, frame)?;
            *page = frame;

            // map the child to the same frame with kernel copy-on-write
            self.frames
                .map(fork.space.base(), *page_index, self.frame_bytes, frame)?;
            fork_pages.insert(*page_index, frame);
        }

        Ok(())
    }

    /// Fork this page map by copying materialized pages.
    fn fork_copied_frames(&self, fork: &Self) -> HeapResult<()> {
        let pages = self.pages.lock();
        let mut fork_pages = fork.pages.lock();

        for page_index in pages.keys() {
            // wasm has no separate virtual memory mappings
            let source = unsafe { self.space.base().add(*page_index * self.frame_bytes) };
            let frame = self.frames.copy(source, self.frame_bytes)?;

            // copy the frame into the child linear memory
            self.frames
                .map(fork.space.base(), *page_index, self.frame_bytes, frame)?;

            fork_pages.insert(*page_index, frame);
        }

        Ok(())
    }

    /// Materialize one private writable page frame.
    fn materialize_page(&self, page_index: usize) -> HeapResult<()> {
        let mut pages = self.pages.lock();

        // already materialized
        if pages.contains_key(&page_index) {
            return Ok(());
        }

        // allocate one private frame and map it into this page map
        let frame = self.frames.allocate(self.frame_bytes)?;
        self.frames
            .map(self.space.base(), page_index, self.frame_bytes, frame)?;

        pages.insert(page_index, frame);

        Ok(())
    }

    /// Return the half-open page-frame range touched by one byte range.
    fn frame_range(&self, offset: usize, byte_len: usize) -> HeapResult<(usize, usize)> {
        // reject ranges that start outside the reservation
        if offset > self.byte_len {
            return Err(HeapError::InvalidByteRange {
                start: offset,
                len: byte_len,
                capacity: self.byte_len,
            });
        }

        // reject ranges that extend past the reservation
        let remaining = self.byte_len - offset;
        if byte_len > remaining {
            return Err(HeapError::InvalidByteRange {
                start: offset,
                len: byte_len,
                capacity: self.byte_len,
            });
        }

        let end = offset + byte_len;
        let first_frame = offset / self.frame_bytes;
        let end_frame = end.div_ceil(self.frame_bytes);

        Ok((first_frame, end_frame))
    }
}

impl Drop for PageMap {
    fn drop(&mut self) {
        let pages = self.pages.lock();
        self.space.unmap(self.frame_bytes, pages.keys().copied());
    }
}
