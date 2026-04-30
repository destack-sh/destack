use std::ptr::null_mut;

use parking_lot::Mutex;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Memory::{
    CreateFileMappingW, MEM_PRESERVE_PLACEHOLDER, MEM_RELEASE, MEM_REPLACE_PLACEHOLDER,
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

/// One page-sized backing frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PageFrame {
    /// The section that owns this page frame.
    pub(crate) section_index: usize,
    /// The byte offset inside the owning section.
    pub(crate) offset: u64,
}

/// Return one page frame inside a contiguous frame range.
pub(crate) fn frame_at(frame: PageFrame, page_offset: usize, page_bytes: usize) -> PageFrame {
    PageFrame {
        section_index: frame.section_index,
        offset: frame.offset + (page_offset * page_bytes) as u64,
    }
}

/// One platform page-frame allocator.
#[derive(Debug)]
pub(crate) struct PageFrameAllocator {
    /// The page-frame sections.
    sections: Mutex<Vec<HANDLE>>,
}

impl Drop for PageFrameAllocator {
    fn drop(&mut self) {
        let sections = self.sections.get_mut();

        // cleanup cannot report errors from Drop
        for section in sections.drain(..) {
            let _ = unsafe { CloseHandle(section) };
        }
    }
}

/// Create one page-frame allocator.
pub(crate) fn create_page_frame_allocator(_byte_len: usize) -> HeapResult<PageFrameAllocator> {
    Ok(PageFrameAllocator {
        sections: Mutex::new(Vec::new()),
    })
}

/// Allocate one zeroed page frame.
pub(crate) fn allocate_frame(
    allocator: &PageFrameAllocator,
    page_bytes: usize,
) -> HeapResult<PageFrame> {
    allocate_frame_range(allocator, page_bytes, page_bytes)
}

/// Allocate one zeroed page-frame range.
pub(crate) fn allocate_frame_range(
    allocator: &PageFrameAllocator,
    byte_len: usize,
    _page_bytes: usize,
) -> HeapResult<PageFrame> {
    let mut sections = allocator.sections.lock();
    let section = create_section(byte_len)?;
    let frame = PageFrame {
        section_index: sections.len(),
        offset: 0,
    };

    sections.push(section);

    Ok(frame)
}

/// Copy one mapped page into a fresh page frame.
pub(crate) fn copy_page(
    allocator: &PageFrameAllocator,
    source: *mut u8,
    page_bytes: usize,
) -> HeapResult<PageFrame> {
    let frame = allocate_frame(allocator, page_bytes)?;
    let target = map_frame_anywhere(allocator, frame, page_bytes)?;

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

/// Map one page frame as shared writable memory.
pub(crate) fn map_page_shared(
    base: *mut u8,
    page_index: usize,
    page_bytes: usize,
    allocator: &PageFrameAllocator,
    frame: PageFrame,
) -> HeapResult<()> {
    map_page(
        base,
        page_index,
        page_bytes,
        allocator,
        frame,
        PAGE_READWRITE,
    )
}

/// Map one page-frame range privately into reserved virtual pages.
pub(crate) fn map_frame_range_private(
    base: *mut u8,
    first_page: usize,
    page_bytes: usize,
    byte_len: usize,
    allocator: &PageFrameAllocator,
    frame: PageFrame,
) -> HeapResult<()> {
    map_frame_range(
        base,
        first_page,
        page_bytes,
        byte_len,
        allocator,
        frame,
        PAGE_WRITECOPY,
    )
}

/// Map one page-frame range as shared writable memory.
pub(crate) fn map_frame_range_shared(
    base: *mut u8,
    first_page: usize,
    page_bytes: usize,
    byte_len: usize,
    allocator: &PageFrameAllocator,
    frame: PageFrame,
) -> HeapResult<()> {
    map_frame_range(
        base,
        first_page,
        page_bytes,
        byte_len,
        allocator,
        frame,
        PAGE_READWRITE,
    )
}

/// Map one page-frame range into reserved virtual pages.
fn map_frame_range(
    base: *mut u8,
    first_page: usize,
    page_bytes: usize,
    byte_len: usize,
    allocator: &PageFrameAllocator,
    frame: PageFrame,
    protection: u32,
) -> HeapResult<()> {
    let page_count = byte_len / page_bytes;

    for page_offset in 0..page_count {
        let page_index = first_page + page_offset;
        let frame = PageFrame {
            section_index: frame.section_index,
            offset: frame.offset + (page_offset * page_bytes) as u64,
        };

        map_page(base, page_index, page_bytes, allocator, frame, protection)?;
    }

    Ok(())
}

/// Map one page frame into a reserved virtual page.
fn map_page(
    base: *mut u8,
    page_index: usize,
    page_bytes: usize,
    allocator: &PageFrameAllocator,
    frame: PageFrame,
    protection: u32,
) -> HeapResult<()> {
    let address = unsafe { base.add(page_index * page_bytes) };
    let section = allocator.section(frame)?;

    split_placeholder(address, page_bytes);
    unmap_page_view(address);

    let data = unsafe {
        MapViewOfFile3(
            section,
            GetCurrentProcess(),
            address.cast(),
            frame.offset,
            page_bytes,
            MEM_REPLACE_PLACEHOLDER,
            protection,
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

/// Map one frame at any available address.
fn map_frame_anywhere(
    allocator: &PageFrameAllocator,
    frame: PageFrame,
    page_bytes: usize,
) -> HeapResult<*mut u8> {
    let section = allocator.section(frame)?;

    let data = unsafe {
        MapViewOfFile3(
            section,
            GetCurrentProcess(),
            null_mut(),
            frame.offset,
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

impl PageFrameAllocator {
    /// Return the Windows section that owns one page frame.
    fn section(&self, frame: PageFrame) -> HeapResult<HANDLE> {
        let sections = self.sections.lock();
        let Some(section) = sections.get(frame.section_index).copied() else {
            return Err(HeapError::AddressSpaceFailed { byte_len: 0 });
        };

        Ok(section)
    }
}

/// Create one page-file backed section.
fn create_section(byte_len: usize) -> HeapResult<HANDLE> {
    let max_size = byte_len as u64;
    let max_size_high = (max_size >> 32) as u32;
    let max_size_low = max_size as u32;
    let section = unsafe {
        CreateFileMappingW(
            INVALID_HANDLE_VALUE,
            null_mut(),
            PAGE_READWRITE,
            max_size_high,
            max_size_low,
            null_mut(),
        )
    };
    if section == 0 {
        return Err(HeapError::AddressSpaceFailed { byte_len });
    }

    Ok(section)
}
