use std::ops::{Bound, RangeBounds};
use std::ptr::{copy_nonoverlapping, read_volatile, write_bytes, write_volatile};
use std::sync::Arc;

use parking_lot::Mutex;

use super::table::{PageState, PageTable};
use crate::platform::{self, PageFrame, PageFrameAllocator, VirtualSpace, WriteWatchRegistration};
use crate::{MemoryError, MemoryResult};

/// The mapped pages for one forkable address space.
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
    /// The atomic page state table.
    pages: Arc<PageTable>,
    /// The write watch registration for this map.
    write_watch: Option<WriteWatchRegistration>,
    /// The page-table mutation lock.
    lock: Mutex<()>,
}

impl PageMap {
    /// Reserve one forkable page map.
    pub(super) fn reserve(byte_len: usize, page_bytes: usize) -> MemoryResult<Self> {
        // align requested pages to native page frames
        let frame_bytes = page_bytes.max(platform::system_frame_bytes()?);
        let page_count = byte_len.div_ceil(frame_bytes);
        let byte_len = page_count * frame_bytes;

        // reserve the backing frame allocator and virtual range
        let frames = Arc::new(platform::create_page_frame_allocator(byte_len)?);
        let space = platform::reserve_virtual_space(byte_len)?;

        Self::new(space, byte_len, frame_bytes, frames)
    }

    /// Fork this page map and isolate pages on first write.
    pub(super) fn fork_lazy(&self) -> MemoryResult<Self> {
        // reserve the child range before changing parent mappings
        let space = platform::reserve_virtual_space(self.byte_len)?;
        let fork = Self::new(space, self.byte_len, self.frame_bytes, self.frames.clone())?;

        // native targets use read-only cow mappings for shared pages
        if platform::SUPPORTS_SHARED_PAGE_FRAMES {
            self.fork_shared_frames(&fork)?;
        }
        // wasm copies materialized linear-memory pages
        else {
            self.fork_copied_frames(&fork)?;
        }

        Ok(fork)
    }

    /// Fork this page map and eagerly isolate mapped pages in one byte range.
    pub(super) fn fork_eager<R>(&self, range: R) -> MemoryResult<Self>
    where
        R: RangeBounds<usize>,
    {
        let fork = self.fork_lazy()?;
        let (offset, byte_len) = self.byte_range(range)?;

        // prepare only pages that already exist in the fork
        fork.make_mapped_writable(offset, byte_len)?;

        Ok(fork)
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
    pub(super) fn zero(&self, offset: usize, byte_len: usize) -> MemoryResult<()> {
        self.make_writable(offset, byte_len)?;

        // empty ranges only validate the address
        if byte_len == 0 {
            return Ok(());
        }

        self.zero_mapped_bytes(offset, byte_len);

        Ok(())
    }

    /// Read bytes into a caller-provided buffer.
    pub(super) fn read_bytes_into(&self, offset: usize, target: &mut [u8]) -> MemoryResult<()> {
        let (first_frame, end_frame) = self.frame_range(offset, target.len())?;

        // empty reads only validate the range
        if target.is_empty() {
            return Ok(());
        }

        let mut written = 0;

        // copy mapped pages and synthesize zeroes for reserved pages
        for page_index in first_frame..end_frame {
            let page_start = page_index * self.frame_bytes;
            let page_end = page_start + self.frame_bytes;
            let copy_start = offset.max(page_start);
            let copy_end = (offset + target.len()).min(page_end);
            let copy_len = copy_end - copy_start;
            let target_start = copy_start - offset;

            // mapped pages read from virtual memory
            if self.pages.is_mapped(page_index) {
                self.copy_mapped_bytes_to(copy_start, &mut target[target_start..][..copy_len]);

                written += copy_len;
                continue;
            }

            // reserved pages are logically zero
            target[target_start..target_start + copy_len].fill(0);
            written += copy_len;
        }

        debug_assert_eq!(written, target.len());

        Ok(())
    }

    /// Return one owned byte vector from this page map.
    pub(super) fn read_bytes(&self, offset: usize, byte_len: usize) -> MemoryResult<Vec<u8>> {
        let mut bytes = vec![0; byte_len];

        // fill the owned buffer from the mapped range
        self.read_bytes_into(offset, &mut bytes)?;

        Ok(bytes)
    }

    /// Return one checked address inside this page map.
    pub(super) fn address(&self, offset: usize, byte_len: usize) -> MemoryResult<*mut u8> {
        self.materialize(offset, byte_len)?;

        Ok(self.mapped_address(offset))
    }

    /// Materialize one byte range inside this page map.
    pub(super) fn materialize(&self, offset: usize, byte_len: usize) -> MemoryResult<()> {
        let (first_frame, end_frame) = self.frame_range(offset, byte_len)?;
        let _lock = self.lock.lock();
        let mut page_index = first_frame;

        // allocate contiguous frame ranges for reserved page runs
        while page_index < end_frame {
            if self.pages.is_mapped(page_index) {
                page_index += 1;
                continue;
            }

            let range_start = page_index;
            while page_index < end_frame && !self.pages.is_mapped(page_index) {
                page_index += 1;
            }

            let page_count = page_index - range_start;
            let byte_len = page_count * self.frame_bytes;
            let frame = platform::allocate_frame_range(&self.frames, byte_len, self.frame_bytes)?;

            // new pages start as owned writable mappings
            platform::map_frame_range_writable(
                self.space.base(),
                range_start,
                self.frame_bytes,
                byte_len,
                &self.frames,
                frame,
            )?;

            // publish every page after the platform mapping succeeds
            for page_offset in 0..page_count {
                let page_frame = platform::frame_at(frame, page_offset, self.frame_bytes);

                self.pages
                    .set_state(range_start + page_offset, PageState::Owned(page_frame));
            }
        }

        Ok(())
    }

    /// Make one byte range writable for an upcoming bulk write.
    pub(super) fn make_writable(&self, offset: usize, byte_len: usize) -> MemoryResult<()> {
        let (first_frame, end_frame) = self.frame_range(offset, byte_len)?;

        // empty ranges only validate the range
        if byte_len == 0 {
            return Ok(());
        }

        // reserved pages need backing frames before protection changes
        self.materialize(offset, byte_len)?;

        self.make_mapped_frame_range_writable(first_frame, end_frame)
    }

    /// Make already mapped pages in one byte range writable.
    fn make_mapped_writable(&self, offset: usize, byte_len: usize) -> MemoryResult<()> {
        let (first_frame, end_frame) = self.frame_range(offset, byte_len)?;

        // empty ranges only validate the range
        if byte_len == 0 {
            return Ok(());
        }

        self.make_mapped_frame_range_writable(first_frame, end_frame)
    }

    /// Write caller-provided bytes directly into this page map.
    pub(super) fn write_bytes(&self, offset: usize, bytes: &[u8]) -> MemoryResult<()> {
        self.make_writable(offset, bytes.len())?;

        self.copy_bytes_to_mapped(offset, bytes);

        Ok(())
    }

    /// Make already mapped shared pages in one page-frame range writable.
    fn make_mapped_frame_range_writable(
        &self,
        first_frame: usize,
        end_frame: usize,
    ) -> MemoryResult<()> {
        let _lock = self.lock.lock();
        let mut page_index = first_frame;

        // make shared page runs writable in one platform call
        while page_index < end_frame {
            let PageState::Shared(_) = self.pages.state(page_index) else {
                page_index += 1;
                continue;
            };
            let run_start = page_index;
            page_index += 1;

            while page_index < end_frame {
                let PageState::Shared(_) = self.pages.state(page_index) else {
                    break;
                };

                page_index += 1;
            }

            let run_len = page_index - run_start;
            let byte_len = run_len * self.frame_bytes;

            platform::make_shared_pages_writable(
                self.space.base(),
                run_start,
                self.frame_bytes,
                byte_len,
            )?;

            // force private pages before later stores
            for page_offset in 0..run_len {
                let offset = (run_start + page_offset) * self.frame_bytes;

                self.force_private_page(offset);
            }

            // publish the modified state after the protection change succeeds
            for page_offset in 0..run_len {
                let page_index = run_start + page_offset;
                let PageState::Shared(frame) = self.pages.state(page_index) else {
                    continue;
                };

                self.pages.set_state(page_index, PageState::Modified(frame));
            }
        }

        Ok(())
    }

    /// Create one page map from a reserved virtual space.
    fn new(
        space: VirtualSpace,
        byte_len: usize,
        frame_bytes: usize,
        frames: Arc<PageFrameAllocator>,
    ) -> MemoryResult<Self> {
        // register stable page metadata for native write watch
        let pages = Arc::new(PageTable::new(space.base() as usize, byte_len, frame_bytes));
        let write_watch = if byte_len == 0 {
            None
        } else {
            Some(platform::register_write_watch(
                space.base(),
                byte_len,
                Arc::as_ptr(&pages).cast(),
            )?)
        };

        Ok(Self {
            space,
            byte_len,
            frame_bytes,
            frames,
            pages,
            write_watch,
            lock: Mutex::new(()),
        })
    }

    /// Fork this page map by sharing page frames.
    fn fork_shared_frames(&self, fork: &Self) -> MemoryResult<()> {
        // serialize parent and child metadata changes
        let _lock = self.lock.lock();
        let _fork_lock = fork.lock.lock();

        // collect mapped page states once so mutation cannot affect iteration
        let pages = self.pages.mapped_states().collect::<Vec<_>>();
        let mut parent_frames = Vec::new();
        let mut child_frames = Vec::with_capacity(pages.len());

        // copy modified runs and queue shareable frames for remapping
        self.collect_shared_fork_frames(&pages, fork, &mut parent_frames, &mut child_frames)?;

        // parent owned pages become shared pages
        self.map_cow_frame_runs(self.space.base(), &parent_frames)?;
        self.set_shared_frame_states(&parent_frames);

        // child shared pages use the same backing frames as the parent
        self.map_cow_frame_runs(fork.space.base(), &child_frames)?;
        platform::retain_frames(
            &self.frames,
            child_frames.iter().map(|(_, frame)| *frame),
            self.frame_bytes,
        );
        fork.set_shared_frame_states(&child_frames);

        Ok(())
    }

    /// Collect shared fork frame mappings and copy modified page runs.
    fn collect_shared_fork_frames(
        &self,
        pages: &[(usize, PageState)],
        fork: &Self,
        parent_frames: &mut Vec<(usize, PageFrame)>,
        child_frames: &mut Vec<(usize, PageFrame)>,
    ) -> MemoryResult<()> {
        let mut page_offset = 0;
        while page_offset < pages.len() {
            let (page_index, state) = pages[page_offset];

            match state {
                // reserved pages are filtered out by mapped_states
                PageState::Reserved => {
                    page_offset += 1;
                }
                // owned pages become read-only cow frames in both maps
                PageState::Owned(frame) => {
                    parent_frames.push((page_index, frame));
                    child_frames.push((page_index, frame));
                    page_offset += 1;
                }
                // shared pages are already shareable, so only the child needs mapping
                PageState::Shared(frame) => {
                    child_frames.push((page_index, frame));
                    page_offset += 1;
                }
                // modified pages need fresh child frames with the parent's visible bytes
                PageState::Modified(frame) => {
                    let run_len = self.modified_run_len(pages, page_offset, page_index, frame);
                    self.fork_modified_frame_run(fork, page_index, run_len)?;
                    page_offset += run_len;
                }
            }
        }

        Ok(())
    }

    /// Return the contiguous modified run length from one mapped page state.
    fn modified_run_len(
        &self,
        pages: &[(usize, PageState)],
        start_offset: usize,
        start_page: usize,
        start_frame: PageFrame,
    ) -> usize {
        let mut page_offset = start_offset + 1;

        while page_offset < pages.len() {
            let page_run_offset = page_offset - start_offset;
            let (page_index, state) = pages[page_offset];
            let expected_page = start_page + page_run_offset;
            let expected_frame = platform::frame_at(start_frame, page_run_offset, self.frame_bytes);
            let PageState::Modified(frame) = state else {
                break;
            };

            // stop when modified page or frame continuity breaks
            if page_index != expected_page || frame != expected_frame {
                break;
            }

            page_offset += 1;
        }

        page_offset - start_offset
    }

    /// Fork one contiguous modified frame run into the child map.
    fn fork_modified_frame_run(
        &self,
        fork: &Self,
        first_page: usize,
        page_count: usize,
    ) -> MemoryResult<()> {
        let byte_len = page_count * self.frame_bytes;
        let source = self.mapped_address(first_page * self.frame_bytes);

        // child receives a fresh writable frame with current parent bytes
        let frame = platform::copy_frame_range(&self.frames, source, byte_len, self.frame_bytes)?;
        platform::map_frame_range_writable(
            fork.space.base(),
            first_page,
            self.frame_bytes,
            byte_len,
            &self.frames,
            frame,
        )?;

        // publish child owned states after the mappings exist
        for page_offset in 0..page_count {
            let page_index = first_page + page_offset;
            let frame = platform::frame_at(frame, page_offset, self.frame_bytes);

            fork.pages.set_state(page_index, PageState::Owned(frame));
        }

        Ok(())
    }

    /// Mark mapped frame entries as shared.
    fn set_shared_frame_states(&self, frames: &[(usize, PageFrame)]) {
        for (page_index, frame) in frames {
            self.pages.set_state(*page_index, PageState::Shared(*frame));
        }
    }

    /// Return one mapped address without validating the range.
    #[inline(always)]
    fn mapped_address(&self, offset: usize) -> *mut u8 {
        // callers validate and materialize the range first
        unsafe { self.space.base().add(offset) }
    }

    /// Copy mapped bytes into one caller buffer.
    #[inline(always)]
    fn copy_mapped_bytes_to(&self, offset: usize, target: &mut [u8]) {
        let source = self.mapped_address(offset);

        // callers only copy from mapped page ranges
        unsafe {
            copy_nonoverlapping(source, target.as_mut_ptr(), target.len());
        }
    }

    /// Copy caller bytes into one mapped page range.
    #[inline(always)]
    fn copy_bytes_to_mapped(&self, offset: usize, bytes: &[u8]) {
        let target = self.mapped_address(offset);

        // callers prepare page protections first
        unsafe {
            copy_nonoverlapping(bytes.as_ptr(), target, bytes.len());
        }
    }

    /// Zero one mapped page range.
    #[inline(always)]
    fn zero_mapped_bytes(&self, offset: usize, byte_len: usize) {
        let target = self.mapped_address(offset);

        // callers prepare page protections first
        unsafe {
            write_bytes(target, 0, byte_len);
        }
    }

    /// Force one shared page to become privately writable.
    #[inline(always)]
    fn force_private_page(&self, offset: usize) {
        let page_address = self.mapped_address(offset);

        // write one byte after protection changes to trigger private backing
        unsafe {
            let byte = read_volatile(page_address);

            write_volatile(page_address, byte);
        }
    }

    /// Fork this page map by copying materialized pages.
    fn fork_copied_frames(&self, fork: &Self) -> MemoryResult<()> {
        let _lock = self.lock.lock();
        let _fork_lock = fork.lock.lock();

        // wasm has no native mappings, so materialized pages are copied
        for page_index in self.pages.mapped_pages() {
            let source = self.mapped_address(page_index * self.frame_bytes);
            let frame = platform::copy_page(&self.frames, source, self.frame_bytes)?;

            // copy the frame into the child linear memory
            platform::map_page_writable(
                fork.space.base(),
                page_index,
                self.frame_bytes,
                &self.frames,
                frame,
            )?;

            fork.pages.set_state(page_index, PageState::Owned(frame));
        }

        Ok(())
    }

    /// Map contiguous page frames as cow runs.
    fn map_cow_frame_runs(&self, base: *mut u8, frames: &[(usize, PageFrame)]) -> MemoryResult<()> {
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

            // map the completed cow run
            platform::map_frame_range_cow(
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

        // map the trailing cow run
        platform::map_frame_range_cow(
            base,
            run_page,
            self.frame_bytes,
            run_len * self.frame_bytes,
            &self.frames,
            run_frame,
        )
    }

    /// Return the half-open page-frame range touched by one byte range.
    fn frame_range(&self, offset: usize, byte_len: usize) -> MemoryResult<(usize, usize)> {
        // reject ranges that start outside the reservation
        if offset > self.byte_len {
            return Err(MemoryError::InvalidByteRange {
                start: offset,
                len: byte_len,
                capacity: self.byte_len,
            });
        }

        // reject ranges that extend past the reservation
        let remaining = self.byte_len - offset;
        if byte_len > remaining {
            return Err(MemoryError::InvalidByteRange {
                start: offset,
                len: byte_len,
                capacity: self.byte_len,
            });
        }

        // convert byte bounds to page-frame bounds
        let end = offset + byte_len;
        let first_frame = offset / self.frame_bytes;
        let end_frame = end.div_ceil(self.frame_bytes);

        Ok((first_frame, end_frame))
    }

    /// Return one byte range from Rust range bounds.
    fn byte_range<R>(&self, range: R) -> MemoryResult<(usize, usize)>
    where
        R: RangeBounds<usize>,
    {
        let start = match range.start_bound() {
            Bound::Included(start) => *start,
            Bound::Excluded(start) => *start + 1,
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(end) => *end + 1,
            Bound::Excluded(end) => *end,
            Bound::Unbounded => self.byte_len,
        };

        // reject reversed ranges through the normal byte-range error
        if end < start {
            return Err(MemoryError::InvalidByteRange {
                start,
                len: 0,
                capacity: self.byte_len,
            });
        }

        Ok((start, end - start))
    }
}

impl Drop for PageMap {
    fn drop(&mut self) {
        let frames = self
            .pages
            .mapped_states()
            .filter_map(|(_, state)| state.frame())
            .collect::<Vec<_>>();

        if let Some(write_watch) = &self.write_watch {
            platform::unregister_write_watch(write_watch);
        }
        self.space
            .unmap(self.frame_bytes, self.pages.mapped_pages());

        platform::release_frames(&self.frames, frames, self.frame_bytes);
    }
}
