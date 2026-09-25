use std::collections::BTreeMap;
use std::ops::Range;
use std::ptr::{copy_nonoverlapping, from_ref, write_bytes};
use std::slice;
use std::sync::Arc;

use parking_lot::Mutex;
use serde::ser::{SerializeSeq, SerializeTuple};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tspp_serde::Reflect;

use super::table::{PageState, PageTable};
use crate::platform::{self, PageFrame, PageFrameAllocator, VirtualSpace, WriteWatchRegistration};
use crate::{MemoryError, MemoryResult};

/// One forkable world-relative memory map.
///
/// Forking and dropping require exclusive access to exposed native addresses.
#[derive(Debug)]
pub struct MemoryMap {
    /// The reserved virtual byte space.
    space: VirtualSpace,
    /// The reserved byte length.
    byte_len: usize,
    /// The fixed mapping frame width.
    frame_size_bytes: usize,
    /// The platform allocator for mapped page frames.
    frames: Arc<PageFrameAllocator>,
    /// The atomic page state table, pinned for the registered write watch.
    pages: Box<PageTable>,
    /// The write watch registration for this map.
    write_watch: Option<WriteWatchRegistration>,
    /// The page table mutation lock.
    lock: Mutex<()>,
    /// The allocated logical byte ranges.
    range_allocator: Mutex<RangeAllocator>,
}

/// One immutable copy-on-write memory image.
///
/// Serialization retains exact target pointer and mapping-frame geometry.
#[derive(Debug, Clone)]
pub struct MemoryImage {
    /// The retained memory map.
    memory: Arc<MemoryMap>,
}

/// One allocated logical byte range.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct MemoryRange {
    /// The byte offset inside the memory map.
    pub offset: usize,
    /// The allocated byte length.
    pub byte_len: usize,
}

/// Logical range allocation state for one memory map.
#[derive(Debug, Clone)]
struct RangeAllocator {
    /// The first allocatable byte offset.
    first_offset: usize,
    /// The next never allocated byte offset.
    frontier: usize,
    /// Released byte ranges keyed by offset.
    free_ranges: BTreeMap<usize, usize>,
}

/// Materialized mappings serialized directly from one memory map.
struct Mappings<'a>(&'a MemoryMap);

/// One contiguous mapped byte range.
#[derive(Debug, Serialize, Deserialize)]
struct Mapping<T> {
    /// The first byte offset inside the memory map.
    offset: usize,
    /// Whether the mapped pages are immutable until released.
    is_immutable: bool,
    /// The mapped bytes.
    bytes: T,
}

/// One contiguous virtual page and backing frame run.
#[derive(Debug, Clone, Copy)]
struct FrameRun {
    /// The first virtual page index.
    first_page: usize,
    /// The first backing frame.
    first_frame: PageFrame,
    /// The contiguous page count.
    page_count: usize,
}

// SAFETY: page table mutations are synchronized inside the memory map
unsafe impl Send for MemoryMap {}

// SAFETY: exposed raw writes are caller synchronized via safepoints
unsafe impl Sync for MemoryMap {}

// mapped_bytes_mut hands out exclusive windows under a caller-proved contract
#[allow(clippy::mut_from_ref)]
impl MemoryMap {
    /// Reserve one forkable memory map.
    pub fn reserve(byte_len: usize, frame_size_bytes: usize) -> MemoryResult<Self> {
        // align requested frames to native memory pages
        let frame_size_bytes = frame_size_bytes.max(platform::system_page_size_bytes()?);
        let page_count = byte_len.div_ceil(frame_size_bytes);
        let byte_len = page_count * frame_size_bytes;

        // reserve the backing frame allocator and virtual range
        let frames = Arc::new(platform::create_page_frame_allocator(byte_len)?);
        let space = platform::reserve_virtual_space(byte_len)?;

        let range_allocator = RangeAllocator::new(frame_size_bytes);

        Self::new(space, byte_len, frame_size_bytes, frames, range_allocator)
    }

    /// Fork this memory map and isolate pages on first write.
    ///
    /// No native writes may race with remapping.
    pub fn fork_lazy(&self) -> MemoryResult<Self> {
        // retain one coherent logical and physical parent state throughout the fork
        let range_allocator = self.range_allocator.lock();
        let space = platform::reserve_virtual_space(self.byte_len)?;
        let fork = Self::new(
            space,
            self.byte_len,
            self.frame_size_bytes,
            self.frames.clone(),
            range_allocator.clone(),
        )?;

        // native targets share read only page frames until either map writes
        if platform::SUPPORTS_SHARED_PAGE_FRAMES {
            self.fork_shared_frames(&fork)?;
        }
        // wasm copies materialized linear memory pages
        else {
            self.fork_copied_frames(&fork)?;
        }

        Ok(fork)
    }

    /// Fork this memory map and eagerly isolate mapped pages in one byte range.
    ///
    /// No native writes may race with remapping.
    pub fn fork_eager(&self, range: Range<usize>) -> MemoryResult<Self> {
        let (first_frame, end_frame) = self.range_frames(range)?;
        let fork = self.fork_lazy()?;

        // prepare only pages that already exist in the fork
        fork.make_mapped_frame_range_writable(first_frame, end_frame)?;

        Ok(fork)
    }

    /// Capture one immutable copy-on-write image.
    ///
    /// No native writes may race with remapping.
    pub fn capture(&self) -> MemoryResult<MemoryImage> {
        let memory = Arc::new(self.fork_lazy()?);

        Ok(MemoryImage { memory })
    }

    /// Return the reserved virtual byte length.
    pub const fn byte_len(&self) -> usize {
        self.byte_len
    }

    /// Return the memory mapping frame width.
    pub const fn frame_size_bytes(&self) -> usize {
        self.frame_size_bytes
    }

    /// Return the base native address for this memory map.
    pub fn base_address(&self) -> usize {
        self.space.base() as usize
    }

    /// Allocate one aligned logical byte range.
    pub fn allocate(&self, byte_len: usize, alignment: usize) -> MemoryResult<MemoryRange> {
        let mut range_allocator = self.range_allocator.lock();

        range_allocator.allocate(byte_len, alignment, self.byte_len)
    }

    /// Allocate one aligned logical range initialized from the provided bytes.
    pub fn allocate_bytes(&self, bytes: &[u8], alignment: usize) -> MemoryResult<MemoryRange> {
        let range = self.allocate(bytes.len(), alignment)?;

        // initialize the allocation or return it to the allocator on failure
        if let Err(error) = self.write_bytes(range.offset, bytes) {
            self.release(range)?;

            return Err(error);
        }

        Ok(range)
    }

    /// Make one complete mapping-frame range immutable until released.
    pub fn freeze(&self, range: MemoryRange) -> MemoryResult<()> {
        // require page ownership because protection applies to complete mapping frames
        if !range.offset.is_multiple_of(self.frame_size_bytes)
            || !range.byte_len.is_multiple_of(self.frame_size_bytes)
        {
            return Err(MemoryError::UnalignedRange {
                offset: range.offset,
                byte_len: range.byte_len,
                alignment: self.frame_size_bytes,
            });
        }

        let (first_frame, end_frame) = self.frame_range(range.offset, range.byte_len)?;
        let range_allocator = self.range_allocator.lock();
        range_allocator.require_live(range)?;
        let _lock = self.lock.lock();

        // materialize reserved pages before changing their protection
        self.materialize_frame_range(first_frame, end_frame)?;

        self.freeze_frame_range(first_frame, end_frame)
    }

    /// Claim one exact logical byte range while restoring memory state.
    pub fn claim(&self, range: MemoryRange) -> MemoryResult<()> {
        let mut range_allocator = self.range_allocator.lock();

        range_allocator.claim(range, self.byte_len)
    }

    /// Release one allocated logical byte range.
    pub fn release(&self, range: MemoryRange) -> MemoryResult<()> {
        let (first_frame, end_frame) = self.frame_range(range.offset, range.byte_len)?;
        let mut range_allocator = self.range_allocator.lock();
        let has_immutable_page =
            (first_frame..end_frame).any(|page_index| self.pages.state(page_index).is_immutable());

        // mutable ranges only touch logical allocation state
        if !has_immutable_page {
            return range_allocator.release(range);
        }

        // released immutable pages must become private before their range is reusable
        range_allocator.require_live(range)?;
        let _lock = self.lock.lock();
        self.thaw_frame_range(first_frame, end_frame)?;
        range_allocator.release_valid(range);

        Ok(())
    }

    /// Zero one byte range inside this memory map.
    pub fn zero(&self, offset: usize, byte_len: usize) -> MemoryResult<()> {
        self.make_writable(offset, byte_len)?;

        // empty ranges only validate the address
        if byte_len == 0 {
            return Ok(());
        }

        // SAFETY: make_writable materialized the full range above
        unsafe {
            self.zero_mapped_bytes(offset, byte_len);
        }

        Ok(())
    }

    /// Read bytes into a caller provided buffer.
    pub fn read_bytes_into(&self, offset: usize, target: &mut [u8]) -> MemoryResult<()> {
        let (first_frame, end_frame) = self.frame_range(offset, target.len())?;

        // empty reads only validate the range
        if target.is_empty() {
            return Ok(());
        }

        let read_end = offset + target.len();

        // copy mapped pages and synthesize zeroes for reserved pages
        for page_index in first_frame..end_frame {
            let page_start = page_index * self.frame_size_bytes;
            let page_end = page_start + self.frame_size_bytes;
            let copy_start = offset.max(page_start);
            let copy_end = read_end.min(page_end);
            let copy_len = copy_end - copy_start;
            let target_start = copy_start - offset;

            // mapped pages read from virtual memory
            if self.pages.is_mapped(page_index) {
                self.copy_mapped_bytes_to(copy_start, &mut target[target_start..][..copy_len]);

                continue;
            }

            // reserved pages are logically zero
            target[target_start..target_start + copy_len].fill(0);
        }

        Ok(())
    }

    /// Return one owned byte vector from this memory map.
    pub fn read_bytes(&self, offset: usize, byte_len: usize) -> MemoryResult<Vec<u8>> {
        let mut bytes = vec![0; byte_len];

        // fill the owned buffer from the mapped range
        self.read_bytes_into(offset, &mut bytes)?;

        Ok(bytes)
    }

    /// Return one checked address inside this memory map.
    ///
    /// Writes may fault once after a lazy fork to isolate the touched frame.
    pub fn address(&self, offset: usize, byte_len: usize) -> MemoryResult<*mut u8> {
        self.materialize(offset, byte_len)?;

        Ok(self.mapped_address(offset))
    }

    /// Materialize one byte range inside this memory map.
    pub fn materialize(&self, offset: usize, byte_len: usize) -> MemoryResult<()> {
        let (first_frame, end_frame) = self.frame_range(offset, byte_len)?;
        let _lock = self.lock.lock();

        self.materialize_frame_range(first_frame, end_frame)
    }

    /// Materialize one page-frame range while holding the page table lock.
    fn materialize_frame_range(&self, first_frame: usize, end_frame: usize) -> MemoryResult<()> {
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
            let byte_len = page_count * self.frame_size_bytes;
            let frame =
                platform::allocate_frame_range(&self.frames, byte_len, self.frame_size_bytes)?;

            // new pages start as owned writable mappings
            platform::map_frame_range_writable(
                self.space.base(),
                range_start,
                self.frame_size_bytes,
                byte_len,
                &self.frames,
                frame,
            )?;

            // publish every page after the platform mapping succeeds
            for page_offset in 0..page_count {
                let page_frame = platform::frame_at(frame, page_offset, self.frame_size_bytes);

                self.pages
                    .set_state(range_start + page_offset, PageState::Owned(page_frame));
            }
        }

        Ok(())
    }

    /// Make one byte range writable for an upcoming bulk write.
    pub fn make_writable(&self, offset: usize, byte_len: usize) -> MemoryResult<()> {
        let (first_frame, end_frame) = self.frame_range(offset, byte_len)?;

        // empty ranges only validate the range
        if byte_len == 0 {
            return Ok(());
        }

        // reserved pages need backing frames before protection changes
        self.materialize(offset, byte_len)?;

        self.make_mapped_frame_range_writable(first_frame, end_frame)
    }

    /// Write caller provided bytes directly into this memory map.
    pub fn write_bytes(&self, offset: usize, bytes: &[u8]) -> MemoryResult<()> {
        self.make_writable(offset, bytes.len())?;

        self.copy_bytes_to_mapped(offset, bytes);

        Ok(())
    }

    /// Copy bytes into a range the caller knows is already mapped.
    ///
    /// # Safety
    ///
    /// The byte range must be live, writable, and fully materialized in this memory map.
    #[inline(always)]
    pub unsafe fn write_mapped_bytes(&self, offset: usize, bytes: &[u8]) {
        self.copy_bytes_to_mapped(offset, bytes);
    }

    /// Zero a range that the caller knows is already mapped.
    ///
    /// # Safety
    ///
    /// The byte range must be live, writable, and fully materialized in this memory map.
    #[inline(always)]
    pub unsafe fn zero_mapped_bytes(&self, offset: usize, byte_len: usize) {
        let target = self.mapped_address(offset);

        // SAFETY: the caller owns the mapped range invariant
        unsafe {
            write_bytes(target, 0, byte_len);
        }
    }

    /// Copy bytes between two ranges that the caller knows are already mapped.
    ///
    /// # Safety
    ///
    /// The ranges must be live, fully materialized, disjoint, and the target writable.
    #[inline(always)]
    pub unsafe fn copy_mapped_bytes(
        &self,
        source_offset: usize,
        target_offset: usize,
        byte_len: usize,
    ) {
        let source = self.mapped_address(source_offset);
        let target = self.mapped_address(target_offset);

        // SAFETY: caller owns the mapped range and disjointness invariants
        unsafe {
            std::ptr::copy_nonoverlapping(source, target, byte_len);
        }
    }

    /// Borrow a mutable byte window that the caller knows is already mapped.
    ///
    /// # Safety
    ///
    /// The range must be live, writable, fully materialized, and exclusively borrowed.
    #[inline(always)]
    pub unsafe fn mapped_bytes_mut(&self, offset: usize, byte_len: usize) -> &mut [u8] {
        let address = self.mapped_address(offset);

        // SAFETY: caller owns the mapped range and exclusivity invariants
        unsafe { std::slice::from_raw_parts_mut(address, byte_len) }
    }

    /// Freeze one fully materialized mapping-frame range.
    fn freeze_frame_range(&self, first_frame: usize, end_frame: usize) -> MemoryResult<()> {
        let mut page_index = first_frame;

        // preserve existing immutable and fork-shared mappings without remapping
        while page_index < end_frame {
            match self.pages.state(page_index) {
                PageState::Reserved => {
                    return Err(MemoryError::internal("reserved page after materialization"));
                }
                PageState::Immutable(_) => {
                    page_index += 1;
                }
                PageState::Shared(frame) => {
                    self.pages
                        .set_state(page_index, PageState::Immutable(frame));
                    page_index += 1;
                }
                PageState::Owned(frame) => {
                    let mut run = FrameRun::new(page_index, frame);
                    page_index += 1;

                    while page_index < end_frame {
                        let PageState::Owned(frame) = self.pages.state(page_index) else {
                            break;
                        };
                        if !run.extend(page_index, frame, self.frame_size_bytes) {
                            break;
                        }

                        page_index += 1;
                    }

                    self.remap_readonly_frame_run(run)?;
                    self.set_immutable_frame_run(run);
                }
                PageState::Modified(_) => {
                    page_index = self.freeze_modified_frame_run(page_index, end_frame)?;
                }
            }
        }

        Ok(())
    }

    /// Freeze one contiguous run whose visible bytes differ from its backing frames.
    fn freeze_modified_frame_run(
        &self,
        first_page: usize,
        end_frame: usize,
    ) -> MemoryResult<usize> {
        let mut end_page = first_page;
        let mut previous_frames = Vec::new();

        // collect the current private run and its superseded backing frames
        while end_page < end_frame {
            let PageState::Modified(frame) = self.pages.state(end_page) else {
                break;
            };

            previous_frames.push(frame);
            end_page += 1;
        }

        // copy current private bytes into one immutable backing-frame run
        let page_count = end_page - first_page;
        let byte_len = page_count * self.frame_size_bytes;
        let source = self.mapped_address(first_page * self.frame_size_bytes);
        let frame =
            platform::copy_frame_range(&self.frames, source, byte_len, self.frame_size_bytes)?;
        let run = FrameRun {
            first_page,
            first_frame: frame,
            page_count,
        };

        // release the fresh frames if their read only mapping cannot be installed
        if let Err(error) = self.remap_readonly_frame_run(run) {
            platform::release_frames(
                &self.frames,
                run.frames(self.frame_size_bytes),
                self.frame_size_bytes,
            );

            return Err(error);
        }

        // publish the immutable frames before releasing replaced backing storage
        self.set_immutable_frame_run(run);
        platform::release_frames(&self.frames, previous_frames, self.frame_size_bytes);

        Ok(end_page)
    }

    /// Make immutable mappings private before their logical range is reused.
    fn thaw_frame_range(&self, first_frame: usize, end_frame: usize) -> MemoryResult<()> {
        let mut page_index = first_frame;

        // convert immutable runs into private writable mappings
        while page_index < end_frame {
            let PageState::Immutable(_) = self.pages.state(page_index) else {
                page_index += 1;
                continue;
            };
            let run_start = page_index;
            page_index += 1;

            while page_index < end_frame
                && matches!(self.pages.state(page_index), PageState::Immutable(_))
            {
                page_index += 1;
            }

            let run_len = page_index - run_start;
            let byte_len = run_len * self.frame_size_bytes;
            platform::make_shared_pages_writable(
                self.space.base(),
                run_start,
                self.frame_size_bytes,
                byte_len,
            )?;

            // private mappings may now diverge from their retained backing frames
            for page_index in run_start..page_index {
                let PageState::Immutable(frame) = self.pages.state(page_index) else {
                    unreachable!("immutable run changed while thawing");
                };

                self.pages.set_state(page_index, PageState::Modified(frame));
            }
        }

        Ok(())
    }

    /// Make already mapped shared pages in one page frame range writable.
    fn make_mapped_frame_range_writable(
        &self,
        first_frame: usize,
        end_frame: usize,
    ) -> MemoryResult<()> {
        let _lock = self.lock.lock();
        let mut page_index = first_frame;
        let mut has_shared_page = false;

        // reject immutable writes before changing protection and detect shared pages
        for page_index in first_frame..end_frame {
            match self.pages.state(page_index) {
                // immutable pages reject the complete write before any transition
                PageState::Immutable(_) => {
                    return Err(MemoryError::ImmutableRange {
                        offset: page_index * self.frame_size_bytes,
                        byte_len: self.frame_size_bytes,
                    });
                }
                // shared pages require a second pass to coalesce protection changes
                PageState::Shared(_) => {
                    has_shared_page = true;
                }
                // owned and modified pages are writable, while reserved pages remain untouched
                PageState::Reserved | PageState::Owned(_) | PageState::Modified(_) => {}
            }
        }
        if !has_shared_page {
            return Ok(());
        }

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
            let byte_len = run_len * self.frame_size_bytes;

            platform::make_shared_pages_writable(
                self.space.base(),
                run_start,
                self.frame_size_bytes,
                byte_len,
            )?;

            // later stores copy the shared pages on first write
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

    /// Create one memory map from a reserved virtual space.
    fn new(
        mut space: VirtualSpace,
        byte_len: usize,
        frame_size_bytes: usize,
        frames: Arc<PageFrameAllocator>,
        range_allocator: RangeAllocator,
    ) -> MemoryResult<Self> {
        // register stable page metadata for native write watch
        let pages = Box::new(PageTable::new(
            space.base() as usize,
            byte_len,
            frame_size_bytes,
        ));
        let write_watch = if byte_len == 0 {
            None
        } else {
            let write_watch = platform::register_write_watch(
                space.base(),
                byte_len,
                from_ref(pages.as_ref()).cast(),
            );
            match write_watch {
                Ok(write_watch) => Some(write_watch),
                Err(error) => {
                    space.unmap(frame_size_bytes, std::iter::empty());

                    return Err(error);
                }
            }
        };

        Ok(Self {
            space,
            byte_len,
            frame_size_bytes,
            frames,
            pages,
            write_watch,
            lock: Mutex::new(()),
            range_allocator: Mutex::new(range_allocator),
        })
    }

    /// Fork this memory map by sharing page frames.
    fn fork_shared_frames(&self, fork: &Self) -> MemoryResult<()> {
        // serialize parent and child metadata changes
        let _lock = self.lock.lock();
        let _fork_lock = fork.lock.lock();

        let mut parent_runs = Vec::new();
        let mut child_runs = Vec::new();
        let mut immutable_runs = Vec::new();

        // copy modified runs and queue shareable frames for remapping
        self.collect_fork_runs(fork, &mut parent_runs, &mut child_runs, &mut immutable_runs)?;

        // parent owned runs become shared mappings
        for run in &parent_runs {
            self.remap_readonly_frame_run(*run)?;
            self.set_shared_frame_run(*run);
        }

        // child runs retain and map the same backing frames
        for run in &child_runs {
            self.map_readonly_frame_run(fork, *run)?;
            platform::retain_frames(
                &self.frames,
                run.frames(self.frame_size_bytes),
                self.frame_size_bytes,
            );
            fork.set_shared_frame_run(*run);
        }

        // immutable runs remain read only and bypass copy on write tracking
        for run in &immutable_runs {
            self.map_readonly_frame_run(fork, *run)?;
            platform::retain_frames(
                &self.frames,
                run.frames(self.frame_size_bytes),
                self.frame_size_bytes,
            );
            fork.set_immutable_frame_run(*run);
        }

        Ok(())
    }

    /// Collect shareable frame runs and copy modified runs.
    fn collect_fork_runs(
        &self,
        fork: &Self,
        parent_runs: &mut Vec<FrameRun>,
        child_runs: &mut Vec<FrameRun>,
        immutable_runs: &mut Vec<FrameRun>,
    ) -> MemoryResult<()> {
        let mut modified_run = None;

        // collect mapping runs directly from sparse page state
        for (page_index, state) in self.pages.mapped_states() {
            if !matches!(state, PageState::Modified(_))
                && let Some(run) = modified_run.take()
            {
                self.fork_modified_frame_run(fork, run)?;
            }

            match state {
                // mapped state iteration must never expose reserved pages
                PageState::Reserved => {
                    return Err(MemoryError::internal(
                        "reserved page in mapped page iteration",
                    ));
                }
                // owned pages become shared read only frames in both maps
                PageState::Owned(frame) => {
                    FrameRun::record(parent_runs, page_index, frame, self.frame_size_bytes);
                    FrameRun::record(child_runs, page_index, frame, self.frame_size_bytes);
                }
                // shared pages are already shareable, so only the child needs mapping
                PageState::Shared(frame) => {
                    FrameRun::record(child_runs, page_index, frame, self.frame_size_bytes);
                }
                // modified pages need fresh child frames with the parent's visible bytes
                PageState::Modified(frame) => {
                    let is_extended = if let Some(run) = &mut modified_run {
                        run.extend(page_index, frame, self.frame_size_bytes)
                    } else {
                        false
                    };
                    if !is_extended
                        && let Some(run) = modified_run.replace(FrameRun::new(page_index, frame))
                    {
                        self.fork_modified_frame_run(fork, run)?;
                    }
                }
                // immutable pages retain their backing frames
                PageState::Immutable(frame) => {
                    FrameRun::record(immutable_runs, page_index, frame, self.frame_size_bytes);
                }
            }
        }

        // copy the trailing modified run
        if let Some(run) = modified_run {
            self.fork_modified_frame_run(fork, run)?;
        }

        Ok(())
    }

    /// Fork one contiguous modified frame run into the child map.
    fn fork_modified_frame_run(&self, fork: &Self, run: FrameRun) -> MemoryResult<()> {
        let byte_len = run.page_count * self.frame_size_bytes;
        let source = self.mapped_address(run.first_page * self.frame_size_bytes);

        // child receives a fresh writable frame with current parent bytes
        let frame =
            platform::copy_frame_range(&self.frames, source, byte_len, self.frame_size_bytes)?;
        let copied_run = FrameRun {
            first_page: run.first_page,
            first_frame: frame,
            page_count: run.page_count,
        };
        platform::map_frame_range_writable(
            fork.space.base(),
            run.first_page,
            self.frame_size_bytes,
            byte_len,
            &self.frames,
            frame,
        )?;

        // publish child owned states after the mappings exist
        for (page_index, frame) in copied_run.pages(self.frame_size_bytes) {
            fork.pages.set_state(page_index, PageState::Owned(frame));
        }

        Ok(())
    }

    /// Mark one mapped frame run shared.
    fn set_shared_frame_run(&self, run: FrameRun) {
        for (page_index, frame) in run.pages(self.frame_size_bytes) {
            self.pages.set_state(page_index, PageState::Shared(frame));
        }
    }

    /// Mark one mapped frame run immutable until released.
    fn set_immutable_frame_run(&self, run: FrameRun) {
        for (page_index, frame) in run.pages(self.frame_size_bytes) {
            self.pages
                .set_state(page_index, PageState::Immutable(frame));
        }
    }

    /// Return one mapped address without validating the range.
    #[inline(always)]
    fn mapped_address(&self, offset: usize) -> *mut u8 {
        // SAFETY: callers validate and materialize the range first
        unsafe { self.space.base().add(offset) }
    }

    /// Copy mapped bytes into one caller buffer.
    #[inline(always)]
    fn copy_mapped_bytes_to(&self, offset: usize, target: &mut [u8]) {
        let source = self.mapped_address(offset);

        // SAFETY: callers only copy from mapped page ranges
        unsafe {
            copy_nonoverlapping(source, target.as_mut_ptr(), target.len());
        }
    }

    /// Copy caller bytes into one mapped page range.
    #[inline(always)]
    fn copy_bytes_to_mapped(&self, offset: usize, bytes: &[u8]) {
        let target = self.mapped_address(offset);

        // SAFETY: callers prepare page protections first
        unsafe {
            copy_nonoverlapping(bytes.as_ptr(), target, bytes.len());
        }
    }

    /// Fork this memory map by copying materialized pages.
    fn fork_copied_frames(&self, fork: &Self) -> MemoryResult<()> {
        let _lock = self.lock.lock();
        let _fork_lock = fork.lock.lock();

        // wasm has no native mappings, so materialized pages are copied
        for (page_index, state) in self.pages.mapped_states() {
            let source = self.mapped_address(page_index * self.frame_size_bytes);
            let frame = platform::copy_page(&self.frames, source, self.frame_size_bytes)?;

            // copy the frame into the child linear memory
            platform::map_page_writable(
                fork.space.base(),
                page_index,
                self.frame_size_bytes,
                &self.frames,
                frame,
            )?;

            let state = if state.is_immutable() {
                PageState::Immutable(frame)
            } else {
                PageState::Owned(frame)
            };
            fork.pages.set_state(page_index, state);
        }

        Ok(())
    }

    /// Map one contiguous frame run as read only storage.
    fn map_readonly_frame_run(&self, target: &Self, run: FrameRun) -> MemoryResult<()> {
        platform::map_frame_range_readonly(
            target.space.base(),
            run.first_page,
            self.frame_size_bytes,
            run.page_count * self.frame_size_bytes,
            &self.frames,
            run.first_frame,
        )
    }

    /// Remap one writable frame run as read only storage.
    fn remap_readonly_frame_run(&self, run: FrameRun) -> MemoryResult<()> {
        platform::remap_frame_range_readonly(
            self.space.base(),
            run.first_page,
            self.frame_size_bytes,
            run.page_count * self.frame_size_bytes,
            &self.frames,
            run.first_frame,
        )
    }

    /// Return the half open page frame range touched by one byte range.
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

        // convert byte bounds to page frame bounds
        let end = offset + byte_len;
        let first_frame = offset / self.frame_size_bytes;
        let end_frame = end.div_ceil(self.frame_size_bytes);

        Ok((first_frame, end_frame))
    }

    /// Return the frame range for one validated byte range.
    fn range_frames(&self, range: Range<usize>) -> MemoryResult<(usize, usize)> {
        let start = range.start;
        let end = range.end;

        // reject reversed ranges through the normal byte range error
        if end < start {
            return Err(MemoryError::InvalidByteRange {
                start,
                len: 0,
                capacity: self.byte_len,
            });
        }

        let byte_len = end - start;
        self.frame_range(start, byte_len)
    }
}

impl Serialize for MemoryImage {
    /// Serialize this image as exact allocator state and mapped frames.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let memory = self.memory.as_ref();
        let ranges = memory.range_allocator.lock();
        let _lock = memory.lock.lock();
        let mappings = Mappings(memory);
        let mut image = serializer.serialize_tuple(5)?;

        image.serialize_element(&memory.byte_len)?;
        image.serialize_element(&memory.frame_size_bytes)?;
        image.serialize_element(&ranges.frontier)?;
        image.serialize_element(&ranges.free_ranges)?;
        image.serialize_element(&mappings)?;

        image.end()
    }
}

impl<'de> Deserialize<'de> for MemoryImage {
    /// Deserialize one exact immutable memory image.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (byte_len, frame_size_bytes, frontier, free_ranges, mappings) =
            <(
                usize,
                usize,
                usize,
                BTreeMap<usize, usize>,
                Box<[Mapping<Box<[u8]>>]>,
            )>::deserialize(deserializer)?;

        Self::decode(byte_len, frame_size_bytes, frontier, free_ranges, mappings)
            .map_err(serde::de::Error::custom)
    }
}

impl MemoryImage {
    /// Decode one serialized memory image.
    fn decode(
        byte_len: usize,
        frame_size_bytes: usize,
        frontier: usize,
        free_ranges: BTreeMap<usize, usize>,
        mappings: Box<[Mapping<Box<[u8]>>]>,
    ) -> MemoryResult<Self> {
        let memory = MemoryMap::reserve(byte_len, frame_size_bytes)?;

        // require the exact target geometry encoded by the image
        if memory.byte_len != byte_len || memory.frame_size_bytes != frame_size_bytes {
            return Err(MemoryError::invalid_image(
                "memory geometry does not match this target",
            ));
        }

        // reconstruct logical allocation state from target geometry
        let ranges = RangeAllocator::restore(
            frontier,
            free_ranges,
            memory.frame_size_bytes,
            memory.byte_len,
        )?;
        *memory.range_allocator.lock() = ranges;

        // restore ordered mappings and their protection
        let mut previous_end = 0;
        for mapping in mappings {
            let range = MemoryRange {
                offset: mapping.offset,
                byte_len: mapping.bytes.len(),
            };
            if range.is_empty()
                || !range.offset.is_multiple_of(memory.frame_size_bytes)
                || !range.byte_len.is_multiple_of(memory.frame_size_bytes)
                || range.offset > memory.byte_len
                || range.byte_len > memory.byte_len - range.offset
                || range.offset < previous_end
            {
                return Err(MemoryError::invalid_image(
                    "mapped memory ranges are malformed",
                ));
            }

            memory.write_bytes(range.offset, &mapping.bytes)?;
            if mapping.is_immutable {
                memory.freeze(range)?;
            }
            previous_end = range.end();
        }

        Ok(Self {
            memory: Arc::new(memory),
        })
    }

    /// Restore this image into one independently writable memory map.
    pub fn restore(&self) -> MemoryResult<MemoryMap> {
        self.memory.fork_lazy()
    }

    /// Return the reserved virtual byte length.
    pub fn byte_len(&self) -> usize {
        self.memory.byte_len()
    }

    /// Return the memory mapping frame width.
    pub fn frame_size_bytes(&self) -> usize {
        self.memory.frame_size_bytes()
    }

    /// Read bytes from this image into a caller-provided buffer.
    pub fn read_bytes_into(&self, offset: usize, target: &mut [u8]) -> MemoryResult<()> {
        self.memory.read_bytes_into(offset, target)
    }

    /// Return one owned byte vector from this image.
    pub fn read_bytes(&self, offset: usize, byte_len: usize) -> MemoryResult<Vec<u8>> {
        self.memory.read_bytes(offset, byte_len)
    }
}

impl Serialize for Mappings<'_> {
    /// Serialize contiguous mapped page runs without copying their bytes first.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let memory = self.0;
        let mut sequence = serializer.serialize_seq(Some(self.len()))?;
        let mut mapped = memory.pages.mapped_states().peekable();

        // serialize each contiguous mapping with one protection state
        while let Some((first_page, state)) = mapped.next() {
            let is_immutable = state.is_immutable();
            let mut end_page = first_page + 1;
            while mapped.peek().is_some_and(|(page, state)| {
                *page == end_page && state.is_immutable() == is_immutable
            }) {
                mapped.next();
                end_page += 1;
            }

            let offset = first_page * memory.frame_size_bytes;
            let byte_len = (end_page - first_page) * memory.frame_size_bytes;

            // SAFETY: mapped page iteration proves the complete virtual range is readable
            let bytes = unsafe { slice::from_raw_parts(memory.mapped_address(offset), byte_len) };
            sequence.serialize_element(&Mapping {
                offset,
                is_immutable,
                bytes,
            })?;
        }

        sequence.end()
    }
}

impl Mappings<'_> {
    /// Return the number of contiguous mapped page runs.
    fn len(&self) -> usize {
        let mut run_count = 0;
        let mut previous = None;

        // count each address or protection transition into one mapped page run
        for (page, state) in self.0.pages.mapped_states() {
            let is_immutable = state.is_immutable();
            if previous.is_none_or(|(previous_page, previous_immutable)| {
                page != previous_page + 1 || is_immutable != previous_immutable
            }) {
                run_count += 1;
            }

            previous = Some((page, is_immutable));
        }

        run_count
    }
}

impl MemoryRange {
    /// The empty memory range.
    pub const EMPTY: Self = Self {
        offset: 0,
        byte_len: 0,
    };

    /// Return the exclusive byte end.
    pub const fn end(self) -> usize {
        self.offset + self.byte_len
    }

    /// Report whether this range is empty.
    pub const fn is_empty(self) -> bool {
        self.byte_len == 0
    }
}

impl FrameRun {
    /// Create one single frame run.
    const fn new(first_page: usize, first_frame: PageFrame) -> Self {
        Self {
            first_page,
            first_frame,
            page_count: 1,
        }
    }

    /// Record one frame in coalesced run storage.
    fn record(runs: &mut Vec<Self>, page_index: usize, frame: PageFrame, frame_size_bytes: usize) {
        let is_extended = if let Some(run) = runs.last_mut() {
            run.extend(page_index, frame, frame_size_bytes)
        } else {
            false
        };
        if !is_extended {
            runs.push(Self::new(page_index, frame));
        }
    }

    /// Extend this run with one contiguous page and frame.
    fn extend(&mut self, page_index: usize, frame: PageFrame, frame_size_bytes: usize) -> bool {
        let next_page = self.first_page + self.page_count;
        let next_frame = platform::frame_at(self.first_frame, self.page_count, frame_size_bytes);
        if page_index != next_page || frame != next_frame {
            return false;
        }

        // extend the contiguous run
        self.page_count += 1;

        true
    }

    /// Iterate over virtual pages and backing frames in this run.
    fn pages(self, frame_size_bytes: usize) -> impl Iterator<Item = (usize, PageFrame)> {
        (0..self.page_count).map(move |page_offset| {
            let page_index = self.first_page + page_offset;
            let frame = platform::frame_at(self.first_frame, page_offset, frame_size_bytes);

            (page_index, frame)
        })
    }

    /// Iterate over backing frames in this run.
    fn frames(self, frame_size_bytes: usize) -> impl Iterator<Item = PageFrame> {
        self.pages(frame_size_bytes).map(|(_, frame)| frame)
    }
}

impl RangeAllocator {
    /// Create empty range allocation state at the first allocatable offset.
    fn new(first_offset: usize) -> Self {
        Self {
            first_offset,
            frontier: first_offset,
            free_ranges: BTreeMap::new(),
        }
    }

    /// Restore logical range allocation state against one memory map.
    fn restore(
        frontier: usize,
        free_ranges: BTreeMap<usize, usize>,
        first_offset: usize,
        capacity: usize,
    ) -> MemoryResult<Self> {
        // accept the canonical allocator state for an empty reservation
        let is_empty_map = capacity == 0 && frontier == first_offset && free_ranges.is_empty();
        if is_empty_map {
            return Ok(Self {
                first_offset,
                frontier,
                free_ranges,
            });
        }

        // require one bounded allocation frontier above the null range
        if frontier < first_offset || frontier > capacity {
            return Err(MemoryError::invalid_image(
                "logical range bounds are malformed",
            ));
        }

        // require ordered, nonempty, disjoint free ranges below the frontier
        let mut previous_end = first_offset;
        for (&offset, &byte_len) in &free_ranges {
            if byte_len == 0
                || offset < previous_end
                || offset > frontier
                || byte_len > frontier - offset
            {
                return Err(MemoryError::invalid_image(
                    "logical free ranges are malformed",
                ));
            }

            previous_end = offset + byte_len;
        }

        Ok(Self {
            first_offset,
            frontier,
            free_ranges,
        })
    }

    /// Allocate one aligned range from released storage or the unused tail.
    fn allocate(
        &mut self,
        byte_len: usize,
        alignment: usize,
        capacity: usize,
    ) -> MemoryResult<MemoryRange> {
        debug_assert!(alignment.is_power_of_two());
        if byte_len == 0 {
            return Ok(MemoryRange::EMPTY);
        }

        // reuse the first released range that satisfies size and alignment
        let reusable = self.free_ranges.iter().find_map(|(&offset, &available)| {
            let aligned = align_up(offset, alignment);
            let padding = aligned - offset;
            let required = padding + byte_len;

            (required <= available).then_some((offset, available, aligned, required))
        });
        let range = if let Some((offset, available, aligned, required)) = reusable {
            self.free_ranges.remove(&offset);

            // retain unused prefix and suffix ranges
            let prefix = aligned - offset;
            if prefix > 0 {
                self.free_ranges.insert(offset, prefix);
            }
            let suffix = available - required;
            if suffix > 0 {
                self.free_ranges.insert(aligned + byte_len, suffix);
            }

            MemoryRange {
                offset: aligned,
                byte_len,
            }
        } else {
            let offset = align_up(self.frontier, alignment);
            if offset > capacity || byte_len > capacity - offset {
                return Err(MemoryError::RangeExhausted {
                    byte_len,
                    alignment,
                    capacity,
                });
            }

            // retain alignment padding below the new range
            if self.frontier < offset {
                self.free_ranges
                    .insert(self.frontier, offset - self.frontier);
            }
            self.frontier = offset + byte_len;

            MemoryRange { offset, byte_len }
        };

        Ok(range)
    }

    /// Claim one exact range that is not already allocated.
    fn claim(&mut self, range: MemoryRange, capacity: usize) -> MemoryResult<()> {
        if range.is_empty() {
            return Ok(());
        }
        if range.offset > capacity || range.byte_len > capacity - range.offset {
            return Err(MemoryError::InvalidByteRange {
                start: range.offset,
                len: range.byte_len,
                capacity,
            });
        }

        // reject the reserved null range
        if range.offset < self.first_offset {
            return Err(MemoryError::RangeOccupied {
                offset: range.offset,
                byte_len: range.byte_len,
            });
        }

        // claim free storage below the allocation frontier
        if range.offset < self.frontier {
            let Some((&free_offset, &free_len)) =
                self.free_ranges.range(..=range.offset).next_back()
            else {
                return Err(MemoryError::RangeOccupied {
                    offset: range.offset,
                    byte_len: range.byte_len,
                });
            };
            let free_end = free_offset + free_len;
            let covered_end = range.end().min(self.frontier);
            if covered_end > free_end {
                return Err(MemoryError::RangeOccupied {
                    offset: range.offset,
                    byte_len: range.byte_len,
                });
            }

            self.free_ranges.remove(&free_offset);
            if free_offset < range.offset {
                self.free_ranges
                    .insert(free_offset, range.offset - free_offset);
            }
            if covered_end < free_end {
                self.free_ranges.insert(covered_end, free_end - covered_end);
            }
        }
        // retain skipped storage when restore first claims a later range
        else if self.frontier < range.offset {
            self.free_ranges
                .insert(self.frontier, range.offset - self.frontier);
        }

        self.frontier = self.frontier.max(range.end());

        Ok(())
    }

    /// Release one live range and merge adjacent free storage.
    fn release(&mut self, range: MemoryRange) -> MemoryResult<()> {
        self.require_live(range)?;
        self.release_valid(range);

        Ok(())
    }

    /// Require one range to be live.
    fn require_live(&self, range: MemoryRange) -> MemoryResult<()> {
        if range.is_empty() {
            return Ok(());
        }

        // reject ranges outside allocated storage
        if range.offset < self.first_offset || range.end() > self.frontier {
            return Err(MemoryError::InvalidRelease {
                offset: range.offset,
                byte_len: range.byte_len,
            });
        }

        // reject ranges that overlap free storage
        let overlaps_previous = self
            .free_ranges
            .range(..=range.offset)
            .next_back()
            .is_some_and(|(&offset, &byte_len)| offset + byte_len > range.offset);
        let overlaps_next = self
            .free_ranges
            .range(range.offset..)
            .next()
            .is_some_and(|(&offset, _)| offset < range.end());
        if overlaps_previous || overlaps_next {
            return Err(MemoryError::InvalidRelease {
                offset: range.offset,
                byte_len: range.byte_len,
            });
        }

        Ok(())
    }

    /// Release one range already proven live.
    fn release_valid(&mut self, range: MemoryRange) {
        if range.is_empty() {
            return;
        }

        // merge the immediately preceding free range
        let mut offset = range.offset;
        let mut byte_len = range.byte_len;
        if let Some((&previous_offset, &previous_len)) =
            self.free_ranges.range(..offset).next_back()
            && previous_offset + previous_len == offset
        {
            self.free_ranges.remove(&previous_offset);
            offset = previous_offset;
            byte_len += previous_len;
        }

        // merge the immediately following free range
        let end = offset + byte_len;
        if let Some(next_len) = self.free_ranges.remove(&end) {
            byte_len += next_len;
        }

        self.free_ranges.insert(offset, byte_len);
    }
}

/// Align one byte offset upward.
fn align_up(offset: usize, alignment: usize) -> usize {
    (offset + alignment - 1) & !(alignment - 1)
}

impl Drop for MemoryMap {
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
            .unmap(self.frame_size_bytes, self.pages.mapped_pages());

        platform::release_frames(&self.frames, frames, self.frame_size_bytes);
    }
}

#[cfg(test)]
mod tests {
    use std::ptr::copy_nonoverlapping;

    use super::*;

    /// Reserve one test memory map and return its mapping frame width.
    fn test_map(frame_count: usize) -> (MemoryMap, usize) {
        let frame_size_bytes =
            platform::system_page_size_bytes().expect("system page width should resolve");
        let map = MemoryMap::reserve(frame_size_bytes * frame_count, frame_size_bytes)
            .expect("memory map should reserve");

        (map, frame_size_bytes)
    }

    /// Read reserved frames as zeroes before materialization.
    #[test]
    fn test_read_reserved_frames_returns_zeroes() {
        let (memory, frame_size_bytes) = test_map(2);

        let bytes = memory
            .read_bytes(frame_size_bytes - 2, 4)
            .expect("bytes should read");

        assert_eq!(bytes, [0, 0, 0, 0]);
    }

    /// Isolate eagerly forked frames before native writes.
    #[test]
    fn test_fork_eager_isolates_selected_frames() {
        let (parent, frame_size_bytes) = test_map(3);
        let initial = vec![1; frame_size_bytes * 3];

        // initialize every frame before forking
        parent
            .write_bytes(0, &initial)
            .expect("parent write should succeed");

        let child = parent
            .fork_eager(frame_size_bytes..frame_size_bytes * 2)
            .expect("eager fork should succeed");
        let child_address = child
            .address(frame_size_bytes, 4)
            .expect("child address should resolve");

        // SAFETY: child_address points at four materialized bytes in the eager range
        unsafe {
            copy_nonoverlapping([9, 8, 7, 6].as_ptr(), child_address, 4);
        }

        let parent_bytes = parent
            .read_bytes(frame_size_bytes, 4)
            .expect("parent bytes should read");
        let child_bytes = child
            .read_bytes(frame_size_bytes, 4)
            .expect("child bytes should read");

        assert_eq!(parent_bytes, [1, 1, 1, 1]);
        assert_eq!(child_bytes, [9, 8, 7, 6]);
    }

    /// Isolate lazy fork writes across multiple mapping frames.
    #[test]
    fn test_fork_lazy_isolates_multi_frame_write() {
        let (parent, frame_size_bytes) = test_map(3);
        let initial = vec![1; frame_size_bytes * 3];
        let replacement = vec![7; frame_size_bytes + 8];
        let write_offset = frame_size_bytes - 4;

        // initialize every frame before forking
        parent
            .write_bytes(0, &initial)
            .expect("parent write should succeed");
        let child = parent.fork_lazy().expect("memory map should fork");

        // write across two frame boundaries in the child
        child
            .write_bytes(write_offset, &replacement)
            .expect("child write should succeed");

        let parent_bytes = parent
            .read_bytes(write_offset, replacement.len())
            .expect("parent bytes should read");
        let child_bytes = child
            .read_bytes(write_offset, replacement.len())
            .expect("child bytes should read");

        assert_eq!(parent_bytes, vec![1; replacement.len()]);
        assert_eq!(child_bytes, replacement);
    }

    /// Isolate child native writes from the parent mapping.
    #[test]
    fn test_fork_isolates_child_native_write() {
        let (parent, _) = test_map(1);
        parent
            .write_bytes(0, &[1, 2, 3, 4])
            .expect("parent write should succeed");

        let child = parent.fork_lazy().expect("memory map should fork");
        let child_address = child.address(0, 4).expect("child address should resolve");

        // SAFETY: child_address points at four materialized child bytes
        unsafe {
            copy_nonoverlapping([9, 8, 7, 6].as_ptr(), child_address, 4);
        }

        assert_eq!(
            parent.read_bytes(0, 4).expect("parent bytes should read"),
            [1, 2, 3, 4]
        );
        assert_eq!(
            child.read_bytes(0, 4).expect("child bytes should read"),
            [9, 8, 7, 6]
        );
    }

    /// Isolate parent native writes from the child mapping.
    #[test]
    fn test_fork_isolates_parent_native_write() {
        let (parent, _) = test_map(1);
        parent
            .write_bytes(0, &[1, 2, 3, 4])
            .expect("parent write should succeed");

        let child = parent.fork_lazy().expect("memory map should fork");
        let parent_address = parent.address(0, 4).expect("parent address should resolve");

        // SAFETY: parent_address points at four materialized parent bytes
        unsafe {
            copy_nonoverlapping([9, 8, 7, 6].as_ptr(), parent_address, 4);
        }

        assert_eq!(
            parent.read_bytes(0, 4).expect("parent bytes should read"),
            [9, 8, 7, 6]
        );
        assert_eq!(
            child.read_bytes(0, 4).expect("child bytes should read"),
            [1, 2, 3, 4]
        );
    }

    /// Preserve visible bytes when forking one modified child.
    #[test]
    fn test_fork_captures_modified_child_frame() {
        let (parent, _) = test_map(1);
        parent
            .write_bytes(0, &[1, 2, 3, 4])
            .expect("parent write should succeed");

        let child = parent.fork_lazy().expect("memory map should fork");
        let child_address = child.address(0, 4).expect("child address should resolve");

        // SAFETY: child_address points at four materialized child bytes
        unsafe {
            copy_nonoverlapping([9, 8, 7, 6].as_ptr(), child_address, 4);
        }

        let grandchild = child.fork_lazy().expect("child should fork");

        assert_eq!(
            grandchild
                .read_bytes(0, 4)
                .expect("grandchild bytes should read"),
            [9, 8, 7, 6]
        );
    }

    /// Isolate later writes after forking one modified child.
    #[test]
    fn test_fork_isolates_modified_child_frame() {
        let (parent, _) = test_map(1);
        parent
            .write_bytes(0, &[1, 2, 3, 4])
            .expect("parent write should succeed");

        let child = parent.fork_lazy().expect("memory map should fork");
        let child_address = child.address(0, 4).expect("child address should resolve");

        // SAFETY: child_address points at four materialized child bytes
        unsafe {
            copy_nonoverlapping([9, 8, 7, 6].as_ptr(), child_address, 4);
        }

        let grandchild = child.fork_lazy().expect("child should fork");
        let grandchild_address = grandchild
            .address(0, 4)
            .expect("grandchild address should resolve");

        // SAFETY: both addresses point at four materialized bytes in their mappings
        unsafe {
            copy_nonoverlapping([2, 2, 2, 2].as_ptr(), child_address, 4);
            copy_nonoverlapping([3, 3, 3, 3].as_ptr(), grandchild_address, 4);
        }

        assert_eq!(
            child.read_bytes(0, 4).expect("child bytes should read"),
            [2, 2, 2, 2]
        );
        assert_eq!(
            grandchild
                .read_bytes(0, 4)
                .expect("grandchild bytes should read"),
            [3, 3, 3, 3]
        );
    }

    /// Isolate writes after reforking one shared child frame.
    #[test]
    fn test_fork_isolates_reforked_shared_frame() {
        let (parent, _) = test_map(1);
        parent
            .write_bytes(0, &[1, 2, 3, 4])
            .expect("parent write should succeed");

        let child = parent.fork_lazy().expect("memory map should fork");
        let grandchild = child.fork_lazy().expect("child should fork");
        let child_address = child.address(0, 4).expect("child address should resolve");
        let grandchild_address = grandchild
            .address(0, 4)
            .expect("grandchild address should resolve");

        // SAFETY: both addresses point at four materialized bytes in their mappings
        unsafe {
            copy_nonoverlapping([9, 8, 7, 6].as_ptr(), child_address, 4);
            copy_nonoverlapping([4, 3, 2, 1].as_ptr(), grandchild_address, 4);
        }

        assert_eq!(
            parent.read_bytes(0, 4).expect("parent bytes should read"),
            [1, 2, 3, 4]
        );
        assert_eq!(
            child.read_bytes(0, 4).expect("child bytes should read"),
            [9, 8, 7, 6]
        );
        assert_eq!(
            grandchild
                .read_bytes(0, 4)
                .expect("grandchild bytes should read"),
            [4, 3, 2, 1]
        );
    }

    /// Materialize reserved storage before exposing one native address.
    #[test]
    fn test_address_materializes_reserved_frame() {
        let (memory, _) = test_map(1);
        let address = memory.address(0, 4).expect("address should resolve");

        // SAFETY: address points at four materialized bytes in the mapping
        unsafe {
            copy_nonoverlapping([5, 6, 7, 8].as_ptr(), address, 4);
        }

        assert_eq!(
            memory.read_bytes(0, 4).expect("bytes should read"),
            [5, 6, 7, 8]
        );
    }

    /// Keep the null frame unmapped when materializing the first allocated range.
    #[test]
    fn test_first_range_preserves_null_frame() {
        let (memory, frame_size_bytes) = test_map(2);
        let range = memory.allocate(16, 8).expect("first range should allocate");

        memory
            .write_bytes(range.offset, &[1; 16])
            .expect("first range should write");

        assert_eq!(range.offset, frame_size_bytes);
        assert!(!memory.pages.is_mapped(0));
        assert!(memory.pages.is_mapped(1));
    }

    /// Preserve restored ranges when images claim them out of offset order.
    #[test]
    fn test_claim_ranges_out_of_order() {
        let mut ranges = RangeAllocator::new(8);

        ranges
            .claim(
                MemoryRange {
                    offset: 128,
                    byte_len: 32,
                },
                256,
            )
            .expect("high range should claim");
        ranges
            .claim(
                MemoryRange {
                    offset: 32,
                    byte_len: 16,
                },
                256,
            )
            .expect("low range should claim from skipped storage");

        let allocated = ranges
            .allocate(16, 16, 256)
            .expect("skipped storage should remain allocatable");

        assert_eq!(
            allocated,
            MemoryRange {
                offset: 16,
                byte_len: 16,
            }
        );
        let next = ranges
            .allocate(16, 16, 256)
            .expect("remaining skipped storage should stay allocatable");

        assert_eq!(
            next,
            MemoryRange {
                offset: 48,
                byte_len: 16,
            }
        );
    }

    /// Reject restored ranges that overlap live storage.
    #[test]
    fn test_claim_rejects_overlap() {
        let mut ranges = RangeAllocator::new(8);
        let occupied = MemoryRange {
            offset: 32,
            byte_len: 32,
        };
        let overlap = MemoryRange {
            offset: 48,
            byte_len: 32,
        };

        ranges
            .claim(occupied, 256)
            .expect("first range should claim");
        let error = ranges
            .claim(overlap, 256)
            .expect_err("overlapping range should fail");

        assert_eq!(
            error,
            MemoryError::RangeOccupied {
                offset: overlap.offset,
                byte_len: overlap.byte_len,
            }
        );
    }

    /// Reuse one released range at the same aligned offset.
    #[test]
    fn test_release_reuses_range() {
        let mut ranges = RangeAllocator::new(8);
        let range = ranges.allocate(24, 16, 256).expect("range should allocate");

        ranges.release(range).expect("range should release");
        let reused = ranges
            .allocate(24, 16, 256)
            .expect("released range should allocate again");

        assert_eq!(reused, range);
    }

    /// Reuse alignment padding left below the allocation frontier.
    #[test]
    fn test_allocate_reuses_alignment_padding() {
        let mut ranges = RangeAllocator::new(8);
        let aligned = ranges
            .allocate(8, 16, 256)
            .expect("aligned range should allocate");
        let padding = ranges
            .allocate(8, 8, 256)
            .expect("alignment padding should allocate");

        assert_eq!(aligned.offset, 16);
        assert_eq!(padding.offset, 8);
    }

    /// Reject releasing one range more than once.
    #[test]
    fn test_release_rejects_free_range() {
        let mut ranges = RangeAllocator::new(8);
        let range = ranges.allocate(24, 8, 256).expect("range should allocate");

        ranges.release(range).expect("range should release");
        let error = ranges
            .release(range)
            .expect_err("free range should not release again");

        assert_eq!(
            error,
            MemoryError::InvalidRelease {
                offset: range.offset,
                byte_len: range.byte_len,
            }
        );
    }

    /// Fork logical range allocation independently at identical coordinates.
    #[test]
    fn test_fork_preserves_range_state() {
        let first_offset = (8 * 1024)
            .max(platform::system_page_size_bytes().expect("system page width should resolve"));
        let memory = MemoryMap::reserve(64 * 1024, 8 * 1024).expect("memory map should reserve");
        let first = memory
            .allocate(1024, 16)
            .expect("parent range should allocate");
        let fork = memory.fork_lazy().expect("memory map should fork");

        let parent_next = memory
            .allocate(2048, 16)
            .expect("parent range should advance");
        let fork_next = fork
            .allocate(2048, 16)
            .expect("fork range should advance independently");

        assert_eq!(first.offset, first_offset);
        assert_eq!(fork_next, parent_next);
    }

    /// Restore sparse bytes, range allocation, and copy-on-write behavior from serialization.
    #[test]
    fn test_restore_serialized_memory_image() {
        let (memory, frame_size_bytes) = test_map(5);
        let first = memory
            .allocate(frame_size_bytes, frame_size_bytes)
            .expect("first range should allocate");
        let reusable = memory
            .allocate(frame_size_bytes, frame_size_bytes)
            .expect("reusable range should allocate");
        let last = memory
            .allocate(frame_size_bytes, frame_size_bytes)
            .expect("last range should allocate");

        // retain two noncontiguous mappings and one reusable logical range
        memory
            .write_bytes(first.offset, &[1, 2, 3, 4])
            .expect("first mapping should write");
        memory
            .write_bytes(last.offset, &[5, 6, 7, 8])
            .expect("last mapping should write");
        memory
            .freeze(last)
            .expect("last mapping should become immutable");
        memory
            .release(reusable)
            .expect("middle range should release");

        // capture mutable and immutable mappings before changing live memory
        let image = memory.capture().expect("memory image should capture");
        memory
            .write_bytes(first.offset, &[9, 9, 9, 9])
            .expect("live mapping should diverge");
        let error = memory
            .write_bytes(last.offset, &[9, 9, 9, 9])
            .expect_err("immutable mapping should reject writes");
        assert_eq!(
            error,
            MemoryError::ImmutableRange {
                offset: last.offset,
                byte_len: frame_size_bytes,
            }
        );

        // restore through the exact public serialization boundary
        let bytes = tspp_serde::to_vec(&image).expect("memory image should serialize");
        let image: MemoryImage =
            tspp_serde::from_slice(&bytes).expect("memory image should deserialize");
        let restored = image.restore().expect("memory image should restore");

        assert_eq!(restored.byte_len(), memory.byte_len());
        assert_eq!(restored.frame_size_bytes(), frame_size_bytes);
        assert_eq!(
            restored
                .read_bytes(first.offset, 4)
                .expect("first mapping should read"),
            [1, 2, 3, 4]
        );
        assert_eq!(
            restored
                .read_bytes(last.offset, 4)
                .expect("last mapping should read"),
            [5, 6, 7, 8]
        );
        assert!(!restored.pages.is_mapped(reusable.offset / frame_size_bytes));

        // preserve allocator reuse and isolate later child writes
        let allocated = restored
            .allocate(frame_size_bytes, frame_size_bytes)
            .expect("restored free range should allocate");
        assert_eq!(allocated, reusable);

        let fork = restored.fork_lazy().expect("restored map should fork");
        fork.write_bytes(first.offset, &[9, 8, 7, 6])
            .expect("fork mapping should write");
        let error = fork
            .write_bytes(last.offset, &[9, 8, 7, 6])
            .expect_err("forked immutable mapping should reject writes");
        assert_eq!(
            restored
                .read_bytes(first.offset, 4)
                .expect("restored mapping should read"),
            [1, 2, 3, 4]
        );
        assert_eq!(
            fork.read_bytes(first.offset, 4)
                .expect("fork mapping should read"),
            [9, 8, 7, 6]
        );
        assert_eq!(
            error,
            MemoryError::ImmutableRange {
                offset: last.offset,
                byte_len: frame_size_bytes,
            }
        );

        // released immutable storage becomes private and reusable in one map only
        restored
            .release(last)
            .expect("immutable range should release");
        let recycled = restored
            .allocate(frame_size_bytes, frame_size_bytes)
            .expect("released immutable range should allocate");
        assert_eq!(recycled, last);
        restored
            .write_bytes(recycled.offset, &[4, 3, 2, 1])
            .expect("recycled mapping should become writable");
        assert_eq!(
            fork.read_bytes(last.offset, 4)
                .expect("fork immutable mapping should read"),
            [5, 6, 7, 8]
        );
    }
}
