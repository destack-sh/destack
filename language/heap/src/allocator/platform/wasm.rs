#![cfg(target_arch = "wasm32")]

use parking_lot::Mutex;

use crate::{HeapError, HeapResult};

/// The WebAssembly linear-memory page width.
const WASM_PAGE_BYTES: usize = 64 * 1024;

/// Whether mapped spaces can share page frames directly.
pub(crate) const SUPPORTS_SHARED_PAGE_FRAMES: bool = false;

/// One reserved virtual byte space.
#[derive(Debug)]
pub(crate) struct VirtualSpace {
    /// The owned byte range.
    data: Box<[u8]>,
}

impl VirtualSpace {
    /// Return the reserved base address.
    pub(crate) fn base(&self) -> *mut u8 {
        self.data.as_ptr().cast_mut()
    }

    /// Unmap this virtual byte space.
    pub(crate) fn unmap<I>(&mut self, _page_bytes: usize, _mapped_pages: I)
    where
        I: IntoIterator<Item = usize>,
    {
        self.data = Box::new([]);
    }
}

/// One page-sized backing frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PageFrame {
    /// The frame index inside the page-frame allocator.
    pub(crate) index: usize,
}

/// Return one page frame inside a contiguous frame range.
pub(crate) fn frame_at(frame: PageFrame, page_offset: usize, _page_bytes: usize) -> PageFrame {
    PageFrame {
        index: frame.index + page_offset,
    }
}

/// One platform page-frame allocator.
#[derive(Debug)]
pub(crate) struct PageFrameAllocator {
    /// The owned page frames.
    frames: Mutex<Vec<Box<[u8]>>>,
}

/// Create one page-frame allocator.
pub(crate) fn create_page_frame_allocator(_byte_len: usize) -> HeapResult<PageFrameAllocator> {
    Ok(PageFrameAllocator {
        frames: Mutex::new(Vec::new()),
    })
}

/// Allocate one zeroed page-frame range.
pub(crate) fn allocate_frame_range(
    allocator: &PageFrameAllocator,
    byte_len: usize,
    page_bytes: usize,
) -> HeapResult<PageFrame> {
    let mut frames = allocator.frames.lock();
    let frame = PageFrame {
        index: frames.len(),
    };
    let page_count = byte_len / page_bytes;

    for _ in 0..page_count {
        frames.push(vec![0; page_bytes].into_boxed_slice());
    }

    Ok(frame)
}

/// Copy one mapped page into a fresh page frame.
pub(crate) fn copy_page(
    allocator: &PageFrameAllocator,
    source: *mut u8,
    page_bytes: usize,
) -> HeapResult<PageFrame> {
    let mut frame = vec![0; page_bytes].into_boxed_slice();

    unsafe {
        std::ptr::copy_nonoverlapping(source, frame.as_mut_ptr(), page_bytes);
    }

    let mut frames = allocator.frames.lock();
    let page_frame = PageFrame {
        index: frames.len(),
    };
    frames.push(frame);

    Ok(page_frame)
}

/// Return the operating-system page byte width.
pub(crate) const fn system_page_bytes() -> HeapResult<usize> {
    Ok(WASM_PAGE_BYTES)
}

/// Reserve one virtual byte range.
pub(crate) fn reserve_virtual_space(byte_len: usize) -> HeapResult<VirtualSpace> {
    Ok(VirtualSpace {
        data: vec![0; byte_len].into_boxed_slice(),
    })
}

/// Map one page frame as shared writable memory.
pub(crate) fn map_page_shared(
    base: *mut u8,
    page_index: usize,
    page_bytes: usize,
    allocator: &PageFrameAllocator,
    frame: PageFrame,
) -> HeapResult<()> {
    map_page(base, page_index, page_bytes, allocator, frame)
}

/// Copy one page-frame range into linear memory.
pub(crate) fn map_frame_range_private(
    base: *mut u8,
    first_page: usize,
    page_bytes: usize,
    byte_len: usize,
    allocator: &PageFrameAllocator,
    frame: PageFrame,
) -> HeapResult<()> {
    map_frame_range(base, first_page, page_bytes, byte_len, allocator, frame)
}

/// Copy one page-frame range into linear memory.
pub(crate) fn map_frame_range_shared(
    base: *mut u8,
    first_page: usize,
    page_bytes: usize,
    byte_len: usize,
    allocator: &PageFrameAllocator,
    frame: PageFrame,
) -> HeapResult<()> {
    map_frame_range(base, first_page, page_bytes, byte_len, allocator, frame)
}

/// Copy one page-frame range into linear memory.
fn map_frame_range(
    base: *mut u8,
    first_page: usize,
    page_bytes: usize,
    byte_len: usize,
    allocator: &PageFrameAllocator,
    frame: PageFrame,
) -> HeapResult<()> {
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
) -> HeapResult<()> {
    let source = {
        let frames = allocator.frames.lock();
        let Some(frame) = frames.get(frame.index) else {
            return Err(HeapError::AddressSpaceFailed {
                byte_len: page_bytes,
            });
        };

        frame.as_ptr()
    };

    let target = unsafe { base.add(page_index * page_bytes) };
    unsafe {
        std::ptr::copy_nonoverlapping(source, target, page_bytes);
    }

    Ok(())
}
