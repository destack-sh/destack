use std::collections::BTreeMap;
use std::sync::Arc;

use parking_lot::Mutex;

use super::platform::{self, PageFrame, PageFrameAllocator, VirtualSpace};
use crate::{HeapError, HeapResult};

/// The mapping state for one materialized page.
#[derive(Debug, Clone, Copy)]
enum PageState {
    /// The mapping is shared writable and the backing frame is current.
    Exclusive(PageFrame),
    /// The mapping is private copy-on-write against the backing frame.
    Forked,
}

/// The mapped pages for one forkable heap space.
#[derive(Debug)]
pub(super) struct PageMap {
    /// The reserved virtual byte space.
    space: VirtualSpace,
    /// The reserved byte length.
    byte_len: usize,
    /// The fixed platform page-frame width.
    frame_bytes: usize,
    /// The platform allocator for mapped page frames.
    frames: Arc<PageFrameAllocator>,
    /// The materialized pages keyed by page index.
    pages: Mutex<BTreeMap<usize, PageState>>,
}

impl PageMap {
    /// Reserve one forkable page map.
    pub(super) fn reserve(byte_len: usize, page_bytes: usize) -> HeapResult<Self> {
        // use the larger of heap pages and platform pages
        let frame_bytes = page_bytes.max(platform::system_page_bytes()?);
        let page_count = byte_len.div_ceil(frame_bytes);
        let byte_len = page_count * frame_bytes;

        // reserve virtual space and its page frames
        let frames = Arc::new(platform::create_page_frame_allocator(byte_len)?);
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

            Ok(fork)
        }
        // each fork receives freshly copied pages (e.g., WASM)
        else {
            self.fork_copied_frames(&fork)?;

            Ok(fork)
        }
    }

    /// Return the reserved virtual byte length.
    pub(super) const fn byte_len(&self) -> usize {
        self.byte_len
    }

    /// Return the native page-frame width used by this map.
    pub(super) const fn frame_bytes(&self) -> usize {
        self.frame_bytes
    }

    /// Return the base native address for this page map.
    pub(super) fn base_address(&self) -> usize {
        self.space.base() as usize
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
        self.materialize(offset, byte_len)?;

        Ok(unsafe { self.space.base().add(offset) })
    }

    /// Materialize one byte range inside this page map.
    pub(super) fn materialize(&self, offset: usize, byte_len: usize) -> HeapResult<()> {
        let (first_frame, end_frame) = self.frame_range(offset, byte_len)?;
        let mut pages = self.pages.lock();
        let mut page_index = first_frame;

        // sparse pages are mapped as contiguous frame ranges
        while page_index < end_frame {
            if pages.contains_key(&page_index) {
                page_index += 1;
                continue;
            }

            let range_start = page_index;
            while page_index < end_frame && !pages.contains_key(&page_index) {
                page_index += 1;
            }

            let page_count = page_index - range_start;
            let byte_len = page_count * self.frame_bytes;
            let frame = platform::allocate_frame_range(&self.frames, byte_len, self.frame_bytes)?;

            platform::map_frame_range_shared(
                self.space.base(),
                range_start,
                self.frame_bytes,
                byte_len,
                &self.frames,
                frame,
            )?;

            for page_offset in 0..page_count {
                let page_frame = platform::frame_at(frame, page_offset, self.frame_bytes);
                pages.insert(range_start + page_offset, PageState::Exclusive(page_frame));
            }
        }

        Ok(())
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
        let mut frames = Vec::with_capacity(pages.len());

        for (page_index, page) in &mut *pages {
            let frame = match *page {
                // exclusive pages already have current bytes in their backing frame
                PageState::Exclusive(frame) => frame,
                // private pages may contain dirty bytes outside their backing frame
                PageState::Forked => {
                    let source = unsafe { self.space.base().add(*page_index * self.frame_bytes) };

                    platform::copy_page(&self.frames, source, self.frame_bytes)?
                }
            };

            *page = PageState::Forked;
            frames.push((*page_index, frame));
        }

        // remap parent and child as private runs backed by the same frames
        self.map_private_frame_runs(self.space.base(), &frames)?;
        self.map_private_frame_runs(fork.space.base(), &frames)?;

        for (page_index, _) in frames {
            fork_pages.insert(page_index, PageState::Forked);
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
            let frame = platform::copy_page(&self.frames, source, self.frame_bytes)?;

            // copy the frame into the child linear memory
            platform::map_page_shared(
                fork.space.base(),
                *page_index,
                self.frame_bytes,
                &self.frames,
                frame,
            )?;

            fork_pages.insert(*page_index, PageState::Exclusive(frame));
        }

        Ok(())
    }

    /// Map contiguous page frames as private copy-on-write runs.
    fn map_private_frame_runs(
        &self,
        base: *mut u8,
        frames: &[(usize, PageFrame)],
    ) -> HeapResult<()> {
        let Some((first_page, first_frame)) = frames.first().copied() else {
            return Ok(());
        };
        let mut run_page = first_page;
        let mut run_frame = first_frame;
        let mut run_len = 1;

        // coalesce adjacent pages backed by adjacent frames
        for (page_index, frame) in frames.iter().copied().skip(1) {
            let next_frame = platform::frame_at(run_frame, run_len, self.frame_bytes);
            if page_index == run_page + run_len && frame == next_frame {
                run_len += 1;

                continue;
            }

            platform::map_frame_range_private(
                base,
                run_page,
                self.frame_bytes,
                run_len * self.frame_bytes,
                &self.frames,
                run_frame,
            )?;

            run_page = page_index;
            run_frame = frame;
            run_len = 1;
        }

        platform::map_frame_range_private(
            base,
            run_page,
            self.frame_bytes,
            run_len * self.frame_bytes,
            &self.frames,
            run_frame,
        )
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
