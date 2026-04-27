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

/// One page-sized frame in the page store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PageFrame {
    /// The frame index inside the page store.
    index: usize,
}

/// One platform page-store handle.
#[derive(Debug)]
pub(crate) struct PageStoreHandle {
    /// The owned page frames.
    frames: Mutex<Vec<Box<[u8]>>>,
}

/// Create one page store.
pub(crate) fn create_page_store(_byte_len: usize) -> HeapResult<PageStoreHandle> {
    Ok(PageStoreHandle {
        frames: Mutex::new(Vec::new()),
    })
}

/// Allocate one zeroed page frame.
pub(crate) fn allocate_frame(handle: &PageStoreHandle, page_bytes: usize) -> HeapResult<PageFrame> {
    let mut frames = handle.frames.lock();
    let frame = PageFrame {
        index: frames.len(),
    };

    frames.push(vec![0; page_bytes].into_boxed_slice());

    Ok(frame)
}

/// Copy one mapped page into a fresh page frame.
pub(crate) fn copy_page(
    handle: &PageStoreHandle,
    source: *mut u8,
    page_bytes: usize,
) -> HeapResult<PageFrame> {
    let mut frame = vec![0; page_bytes].into_boxed_slice();

    unsafe {
        std::ptr::copy_nonoverlapping(source, frame.as_mut_ptr(), page_bytes);
    }

    let mut frames = handle.frames.lock();
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

/// Reserve one virtual byte range for allocator chunks.
pub(crate) fn reserve_chunk_space(byte_len: usize) -> HeapResult<*mut u8> {
    let data = vec![0; byte_len].into_boxed_slice();
    let data = Box::into_raw(data);

    Ok(data.cast())
}

/// Commit one chunk range for allocator payloads.
pub(crate) const fn commit_chunk_space(_data: *mut u8, _byte_len: usize) -> HeapResult<()> {
    Ok(())
}

/// Unmap one chunk range.
pub(crate) fn unmap_chunk_space(data: *mut u8, byte_len: usize) -> HeapResult<()> {
    if byte_len == 0 {
        return Ok(());
    }

    let data = std::ptr::slice_from_raw_parts_mut(data, byte_len);
    unsafe {
        drop(Box::from_raw(data));
    }

    Ok(())
}

/// Map one page-store frame into a reserved virtual page.
pub(crate) fn map_page(
    base: *mut u8,
    page_index: usize,
    page_bytes: usize,
    handle: &PageStoreHandle,
    frame: PageFrame,
) -> HeapResult<()> {
    let source = {
        let frames = handle.frames.lock();
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
