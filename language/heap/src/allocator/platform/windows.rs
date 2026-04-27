use std::ptr::null_mut;

use parking_lot::Mutex;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Memory::{
    CreateFileMappingW, MEM_COMMIT, MEM_PRESERVE_PLACEHOLDER, MEM_RELEASE, MEM_REPLACE_PLACEHOLDER,
    MEM_RESERVE, MEM_RESERVE_PLACEHOLDER, MEMORY_MAPPED_VIEW_ADDRESS, MapViewOfFile3,
    PAGE_NOACCESS, PAGE_READWRITE, PAGE_WRITECOPY, UnmapViewOfFile2, VirtualAlloc2, VirtualFree,
};
use windows_sys::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

use crate::{HeapError, HeapResult};

/// Whether mapped spaces can share page frames directly.
pub(crate) const SUPPORTS_SHARED_PAGE_FRAMES: bool = true;

/// One reserved virtual byte space.
#[derive(Debug)]
pub(crate) struct VirtualSpace {
    /// The reserved base address.
    base: *mut u8,
    /// The reserved byte length.
    byte_len: usize,
}

impl VirtualSpace {
    /// Return the reserved base address.
    pub(crate) const fn base(&self) -> *mut u8 {
        self.base
    }

    /// Unmap this virtual byte space.
    pub(crate) fn unmap<I>(&mut self, page_bytes: usize, mapped_pages: I)
    where
        I: IntoIterator<Item = usize>,
    {
        if self.base.is_null() || self.byte_len == 0 {
            return;
        }

        let mut next_page = 0;
        let page_count = self.byte_len / page_bytes;

        // release sparse gaps and mapped page views
        for page_index in mapped_pages {
            release_virtual_gap(self.base, next_page, page_index, page_bytes);

            let address = unsafe { self.base.add(page_index * page_bytes) };
            unmap_page_view(address);
            release_placeholder(address);

            next_page = page_index + 1;
        }

        release_virtual_gap(self.base, next_page, page_count, page_bytes);
        self.base = null_mut();
        self.byte_len = 0;
    }
}

/// One page-sized frame in the page store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PageFrame {
    /// The logical page-frame byte offset.
    pub(crate) offset: u64,
}

/// One platform page-store handle.
#[derive(Debug)]
pub(crate) struct PageStoreHandle {
    /// The page-frame section handles.
    frames: Mutex<Vec<HANDLE>>,
}

impl Drop for PageStoreHandle {
    fn drop(&mut self) {
        for frame in self.frames.get_mut() {
            // cleanup cannot report errors from Drop
            let _ = unsafe { CloseHandle(*frame) };
        }
    }
}

/// Create one page store.
pub(crate) fn create_page_store(_byte_len: usize) -> HeapResult<PageStoreHandle> {
    Ok(PageStoreHandle {
        frames: Mutex::new(Vec::new()),
    })
}

/// Allocate one zeroed page frame.
pub(crate) fn allocate_frame(handle: &PageStoreHandle, page_bytes: usize) -> HeapResult<PageFrame> {
    let section = unsafe {
        CreateFileMappingW(
            INVALID_HANDLE_VALUE,
            null_mut(),
            PAGE_READWRITE,
            0,
            page_bytes as u32,
            null_mut(),
        )
    };
    if section == 0 {
        return Err(HeapError::AddressSpaceFailed {
            byte_len: page_bytes,
        });
    }

    let mut frames = handle.frames.lock();
    let frame = PageFrame {
        offset: frames.len() as u64 * page_bytes as u64,
    };
    frames.push(section);

    Ok(frame)
}

/// Copy one mapped page into a fresh page frame.
pub(crate) fn copy_page(
    handle: &PageStoreHandle,
    source: *mut u8,
    page_bytes: usize,
) -> HeapResult<PageFrame> {
    let frame = allocate_frame(handle, page_bytes)?;
    let frame_handle = frame_handle(handle, frame, page_bytes)?;
    let target = map_frame_anywhere(frame_handle, page_bytes)?;

    unsafe {
        std::ptr::copy_nonoverlapping(source, target, page_bytes);
    }

    unmap_page_view(target);

    Ok(frame)
}

/// Return the operating-system page byte width.
pub(crate) fn system_page_bytes() -> HeapResult<usize> {
    let mut system = std::mem::MaybeUninit::<SYSTEM_INFO>::uninit();

    unsafe {
        GetSystemInfo(system.as_mut_ptr());
    }

    let system = unsafe { system.assume_init() };
    let page_bytes = system.dwPageSize as usize;
    if page_bytes == 0 {
        return Err(HeapError::InvariantViolation {
            context: "system page size",
        });
    }

    Ok(page_bytes)
}

/// Reserve one inaccessible virtual address range.
pub(crate) fn reserve_virtual_space(byte_len: usize) -> HeapResult<VirtualSpace> {
    if byte_len == 0 {
        return Ok(VirtualSpace {
            base: null_mut(),
            byte_len,
        });
    }

    let data = reserve_placeholder(byte_len)?;

    Ok(VirtualSpace {
        base: data,
        byte_len,
    })
}

/// Reserve one inaccessible virtual address range for allocator chunks.
pub(crate) fn reserve_chunk_space(byte_len: usize) -> HeapResult<*mut u8> {
    reserve_placeholder(byte_len)
        .map_err(|_| HeapError::AllocatorChunkAllocationFailed { byte_len })
}

/// Commit one chunk range for allocator payloads.
pub(crate) fn commit_chunk_space(data: *mut u8, byte_len: usize) -> HeapResult<()> {
    split_placeholder(data, byte_len);

    let data = unsafe {
        VirtualAlloc2(
            GetCurrentProcess(),
            data.cast(),
            byte_len,
            MEM_RESERVE | MEM_COMMIT | MEM_REPLACE_PLACEHOLDER,
            PAGE_READWRITE,
            null_mut(),
            0,
        )
    };
    if data.is_null() {
        return Err(HeapError::AllocatorChunkAllocationFailed { byte_len });
    }

    Ok(())
}

/// Unmap one chunk range and report unexpected OS failure.
pub(crate) fn unmap_chunk_space(data: *mut u8, byte_len: usize) -> HeapResult<()> {
    if byte_len == 0 {
        return Ok(());
    }

    split_placeholder(data, byte_len);
    if release_placeholder(data) {
        return Ok(());
    }

    Err(HeapError::InvariantViolation {
        context: "allocator chunk unmap",
    })
}

/// Map one page-store frame into a reserved virtual page.
pub(crate) fn map_page(
    base: *mut u8,
    page_index: usize,
    page_bytes: usize,
    handle: &PageStoreHandle,
    frame: PageFrame,
) -> HeapResult<()> {
    let address = unsafe { base.add(page_index * page_bytes) };
    let frame = frame_handle(handle, frame, page_bytes)?;

    split_placeholder(address, page_bytes);
    unmap_page_view(address);

    let data = unsafe {
        MapViewOfFile3(
            frame,
            GetCurrentProcess(),
            address.cast(),
            0,
            page_bytes,
            MEM_REPLACE_PLACEHOLDER,
            PAGE_WRITECOPY,
            null_mut(),
            0,
        )
    };
    if data.Value.is_null() {
        return Err(HeapError::AddressSpaceFailed {
            byte_len: page_bytes,
        });
    }

    Ok(())
}

/// Reserve one Windows placeholder range.
fn reserve_placeholder(byte_len: usize) -> HeapResult<*mut u8> {
    let data = unsafe {
        VirtualAlloc2(
            GetCurrentProcess(),
            null_mut(),
            byte_len,
            MEM_RESERVE | MEM_RESERVE_PLACEHOLDER,
            PAGE_NOACCESS,
            null_mut(),
            0,
        )
    };
    if data.is_null() {
        return Err(HeapError::AddressSpaceFailed { byte_len });
    }

    Ok(data.cast())
}

/// Split out one exact placeholder range when it is part of a larger placeholder.
fn split_placeholder(data: *mut u8, byte_len: usize) {
    // the call fails when the range is already exact or not a placeholder
    let _ = unsafe {
        VirtualFree(
            data.cast(),
            byte_len,
            MEM_RELEASE | MEM_PRESERVE_PLACEHOLDER,
        )
    };
}

/// Release one exact placeholder range.
fn release_placeholder(data: *mut u8) -> bool {
    let result = unsafe { VirtualFree(data.cast(), 0, MEM_RELEASE) };

    result != 0
}

/// Release one placeholder gap between mapped pages.
fn release_virtual_gap(base: *mut u8, first_page: usize, end_page: usize, page_bytes: usize) {
    if first_page == end_page {
        return;
    }

    let address = unsafe { base.add(first_page * page_bytes) };
    release_placeholder(address);
}

/// Unmap one page view back into an exact placeholder.
fn unmap_page_view(address: *mut u8) {
    let _ = unsafe {
        UnmapViewOfFile2(
            GetCurrentProcess(),
            MEMORY_MAPPED_VIEW_ADDRESS {
                Value: address.cast(),
            },
            MEM_PRESERVE_PLACEHOLDER,
        )
    };
}

/// Return one frame handle by page-frame id.
fn frame_handle(
    handle: &PageStoreHandle,
    frame: PageFrame,
    page_bytes: usize,
) -> HeapResult<HANDLE> {
    let frame_index = (frame.offset / page_bytes as u64) as usize;
    let frames = handle.frames.lock();
    let Some(frame) = frames.get(frame_index) else {
        return Err(HeapError::AddressSpaceFailed {
            byte_len: page_bytes,
        });
    };

    Ok(*frame)
}

/// Map one frame at any available address.
fn map_frame_anywhere(frame: HANDLE, page_bytes: usize) -> HeapResult<*mut u8> {
    let data = unsafe {
        MapViewOfFile3(
            frame,
            GetCurrentProcess(),
            null_mut(),
            0,
            page_bytes,
            0,
            PAGE_READWRITE,
            null_mut(),
            0,
        )
    };
    if data.Value.is_null() {
        return Err(HeapError::AddressSpaceFailed {
            byte_len: page_bytes,
        });
    }

    Ok(data.Value.cast())
}
