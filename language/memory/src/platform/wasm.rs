use std::ptr::copy_nonoverlapping;

use parking_lot::Mutex;

use crate::{MemoryError, MemoryResult};

/// The WebAssembly linear memory page width.
const WASM_PAGE_BYTES: usize = 64 * 1024;

/// Whether mapped spaces can share page frames directly.
pub(crate) const SUPPORTS_SHARED_PAGE_FRAMES: bool = false;

/// One reserved virtual byte space.
#[derive(Debug)]
pub(crate) struct VirtualSpace {
    /// The owned byte range.
    bytes: Box<[u8]>,
}

impl VirtualSpace {
    /// Return the reserved base address.
    pub(crate) fn base(&self) -> *mut u8 {
        self.bytes.as_ptr().cast_mut()
    }

    /// Unmap this virtual byte space.
    pub(crate) fn unmap<I>(&mut self, _page_bytes: usize, _mapped_pages: I)
    where
        I: IntoIterator<Item = usize>,
    {
        self.bytes = Box::new([]);
    }
}

/// One page sized backing frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PageFrame {
    /// The frame index inside the page frame allocator.
    pub(crate) index: usize,
}

/// One platform page frame allocator.
#[derive(Debug)]
pub(crate) struct PageFrameAllocator {
    /// The reusable frame state.
    state: Mutex<PageFrameAllocatorState>,
}

/// One registered write watched virtual range.
#[derive(Debug)]
pub(crate) struct WriteWatchRegistration;

/// The linear memory frame store, free ranges, and live reference counts.
#[derive(Debug)]
struct PageFrameAllocatorState {
    /// The owned page frames.
    frames: Vec<Box<[u8]>>,
    /// The free frame index ranges.
    free_ranges: Vec<PageFrameRange>,
    /// The live reference counts keyed by frame index.
    frame_ref_counts: Vec<u32>,
}

/// One reusable page frame range.
#[derive(Debug, Clone, Copy)]
struct PageFrameRange {
    /// The first page frame index.
    index: usize,
    /// The frame count.
    frame_count: usize,
}

/// Return one page frame inside a contiguous frame range.
pub(crate) fn frame_at(frame: PageFrame, page_offset: usize, _page_bytes: usize) -> PageFrame {
    PageFrame {
        index: frame.index + page_offset,
    }
}

/// Create one page frame allocator.
pub(crate) fn create_page_frame_allocator(_byte_len: usize) -> MemoryResult<PageFrameAllocator> {
    Ok(PageFrameAllocator {
        state: Mutex::new(PageFrameAllocatorState {
            frames: Vec::new(),
            free_ranges: Vec::new(),
            frame_ref_counts: Vec::new(),
        }),
    })
}

/// Allocate one zeroed page frame range.
pub(crate) fn allocate_frame_range(
    allocator: &PageFrameAllocator,
    byte_len: usize,
    page_bytes: usize,
) -> MemoryResult<PageFrame> {
    let page_count = byte_len / page_bytes;
    let (frame, is_reused) = allocate_frame_storage(allocator, page_count, page_bytes);

    // reused linear memory frames must regain fresh page zero semantics
    if is_reused {
        zero_frame_range(allocator, frame, page_count);
    }

    Ok(frame)
}

/// Retain mapped page frames.
pub(crate) fn retain_frames<I>(allocator: &PageFrameAllocator, frames: I, _page_bytes: usize)
where
    I: IntoIterator<Item = PageFrame>,
{
    let mut state = allocator.state.lock();

    for frame in frames {
        let ref_count = &mut state.frame_ref_counts[frame.index];

        *ref_count += 1;
    }
}

/// Release mapped page frames.
pub(crate) fn release_frames<I>(allocator: &PageFrameAllocator, frames: I, _page_bytes: usize)
where
    I: IntoIterator<Item = PageFrame>,
{
    let mut state = allocator.state.lock();

    for frame in frames {
        let ref_count = &mut state.frame_ref_counts[frame.index];

        // keep copied frames live until the last map drops
        *ref_count -= 1;
        if *ref_count != 0 {
            continue;
        }

        state.free_ranges.push(PageFrameRange {
            index: frame.index,
            frame_count: 1,
        });
    }

    // merge once after the whole release batch
    merge_free_frame_ranges(&mut state.free_ranges);
}

/// Copy one mapped page into a fresh page frame.
pub(crate) fn copy_page(
    allocator: &PageFrameAllocator,
    source: *mut u8,
    page_bytes: usize,
) -> MemoryResult<PageFrame> {
    copy_frame_range(allocator, source, page_bytes, page_bytes)
}

/// Copy one mapped byte range into a fresh page frame range.
pub(crate) fn copy_frame_range(
    allocator: &PageFrameAllocator,
    source: *mut u8,
    byte_len: usize,
    page_bytes: usize,
) -> MemoryResult<PageFrame> {
    let page_count = byte_len / page_bytes;
    let (page_frame, _) = allocate_frame_storage(allocator, page_count, page_bytes);
    let mut state = allocator.state.lock();

    // copy each linear memory page into the contiguous frame range
    for page_offset in 0..page_count {
        // SAFETY: caller passes a mapped source range covering byte_len bytes
        let source = unsafe { source.add(page_offset * page_bytes) };
        let frame = &mut state.frames[page_frame.index + page_offset];

        // SAFETY: source and frame both cover one full page
        unsafe {
            copy_nonoverlapping(source, frame.as_mut_ptr(), page_bytes);
        }
    }

    Ok(page_frame)
}

/// Allocate backing storage for one page frame range.
fn allocate_frame_storage(
    allocator: &PageFrameAllocator,
    frame_count: usize,
    page_bytes: usize,
) -> (PageFrame, bool) {
    let mut state = allocator.state.lock();

    // reuse returned linear memory frames before extending the frame store
    let (frame, is_reused) = if let Some(frame) = allocate_free_frame_range(&mut state, frame_count)
    {
        (frame, true)
    } else {
        let frame = PageFrame {
            index: state.frames.len(),
        };

        for _ in 0..frame_count {
            state.frames.push(vec![0; page_bytes].into_boxed_slice());
        }

        (frame, false)
    };

    // each page starts with one owning mapping
    initialize_frame_references(&mut state, frame, frame_count);

    (frame, is_reused)
}

/// Return the platform frame byte width for linear memory mappings.
pub(crate) const fn system_frame_bytes() -> MemoryResult<usize> {
    Ok(WASM_PAGE_BYTES)
}

/// Reserve one virtual byte range.
pub(crate) fn reserve_virtual_space(byte_len: usize) -> MemoryResult<VirtualSpace> {
    Ok(VirtualSpace {
        bytes: vec![0; byte_len].into_boxed_slice(),
    })
}

/// Map one page frame as writable memory.
pub(crate) fn map_page_writable(
    base: *mut u8,
    page_index: usize,
    page_bytes: usize,
    allocator: &PageFrameAllocator,
    frame: PageFrame,
) -> MemoryResult<()> {
    map_page(base, page_index, page_bytes, allocator, frame)
}

/// Copy one page frame range into linear memory.
pub(crate) fn map_frame_range_cow(
    base: *mut u8,
    first_page: usize,
    page_bytes: usize,
    byte_len: usize,
    allocator: &PageFrameAllocator,
    frame: PageFrame,
) -> MemoryResult<()> {
    map_frame_range(base, first_page, page_bytes, byte_len, allocator, frame)
}

/// Copy one page frame range into linear memory.
pub(crate) fn map_frame_range_writable(
    base: *mut u8,
    first_page: usize,
    page_bytes: usize,
    byte_len: usize,
    allocator: &PageFrameAllocator,
    frame: PageFrame,
) -> MemoryResult<()> {
    map_frame_range(base, first_page, page_bytes, byte_len, allocator, frame)
}

/// Prepare copied linear memory pages for writes.
pub(crate) fn make_shared_pages_writable(
    _base: *mut u8,
    _first_page: usize,
    _page_bytes: usize,
    _byte_len: usize,
) -> MemoryResult<()> {
    Ok(())
}

/// Register one write watched virtual range.
pub(crate) fn register_write_watch(
    _base: *mut u8,
    _byte_len: usize,
    _context: *const (),
) -> MemoryResult<WriteWatchRegistration> {
    Ok(WriteWatchRegistration)
}

/// Unregister one write watched virtual range.
pub(crate) fn unregister_write_watch(_registration: &WriteWatchRegistration) {}

/// Copy one page frame range into linear memory.
fn map_frame_range(
    base: *mut u8,
    first_page: usize,
    page_bytes: usize,
    byte_len: usize,
    allocator: &PageFrameAllocator,
    frame: PageFrame,
) -> MemoryResult<()> {
    let page_count = byte_len / page_bytes;

    for page_offset in 0..page_count {
        let page_index = first_page + page_offset;
        let frame = PageFrame {
            index: frame.index + page_offset,
        };

        map_page(base, page_index, page_bytes, allocator, frame)?;
    }

    Ok(())
}

/// Copy one page frame into linear memory.
fn map_page(
    base: *mut u8,
    page_index: usize,
    page_bytes: usize,
    allocator: &PageFrameAllocator,
    frame: PageFrame,
) -> MemoryResult<()> {
    let source = {
        let state = allocator.state.lock();
        let Some(frame) = state.frames.get(frame.index) else {
            return Err(MemoryError::InvariantViolation {
                context: "linear memory frame",
            });
        };

        frame.as_ptr()
    };

    // SAFETY: page_index is inside the owned linear memory reservation
    let target = unsafe { base.add(page_index * page_bytes) };
    // SAFETY: source and target both cover one full page
    unsafe {
        copy_nonoverlapping(source, target, page_bytes);
    }

    Ok(())
}

/// Allocate one free frame range when a large enough range exists.
fn allocate_free_frame_range(
    state: &mut PageFrameAllocatorState,
    frame_count: usize,
) -> Option<PageFrame> {
    let range_index = state
        .free_ranges
        .iter()
        .position(|range| range.frame_count >= frame_count)?;
    let frame_index = state.free_ranges[range_index].index;
    let frame = PageFrame { index: frame_index };

    // consume the front of the free range
    state.free_ranges[range_index].index += frame_count;
    state.free_ranges[range_index].frame_count -= frame_count;

    if state.free_ranges[range_index].frame_count == 0 {
        state.free_ranges.swap_remove(range_index);
    }

    Some(frame)
}

/// Zero one reusable frame range.
fn zero_frame_range(allocator: &PageFrameAllocator, frame: PageFrame, frame_count: usize) {
    let mut state = allocator.state.lock();

    for page_offset in 0..frame_count {
        state.frames[frame.index + page_offset].fill(0);
    }
}

/// Initialize one reference count for every page in a frame range.
fn initialize_frame_references(
    state: &mut PageFrameAllocatorState,
    frame: PageFrame,
    frame_count: usize,
) {
    for page_offset in 0..frame_count {
        let index = frame.index + page_offset;

        if index >= state.frame_ref_counts.len() {
            state.frame_ref_counts.resize(index + 1, 0);
        }

        state.frame_ref_counts[index] = 1;
    }
}

/// Merge adjacent free frame ranges.
fn merge_free_frame_ranges(ranges: &mut Vec<PageFrameRange>) {
    ranges.sort_by_key(|range| range.index);

    let mut range_index = 0;
    while range_index + 1 < ranges.len() {
        let current_end = ranges[range_index].index + ranges[range_index].frame_count;

        // adjacent ranges become one reusable range
        if current_end == ranges[range_index + 1].index {
            let next = ranges.remove(range_index + 1);
            ranges[range_index].frame_count += next.frame_count;
            continue;
        }

        range_index += 1;
    }
}
