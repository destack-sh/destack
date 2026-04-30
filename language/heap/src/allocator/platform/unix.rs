#![cfg(unix)]

use std::os::fd::RawFd;
use std::ptr::null_mut;

use parking_lot::Mutex;

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
    pub(crate) fn unmap<I>(&mut self, _page_bytes: usize, _mapped_pages: I)
    where
        I: IntoIterator<Item = usize>,
    {
        if self.base.is_null() || self.byte_len == 0 {
            return;
        }

        // cleanup cannot report errors from Drop
        let _ = unsafe { libc::munmap(self.base.cast(), self.byte_len) };
        self.base = null_mut();
        self.byte_len = 0;
    }
}

/// One page-sized backing frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PageFrame {
    /// The byte offset of this frame inside the page-frame allocator.
    pub(crate) offset: u64,
}

/// Return one page frame inside a contiguous frame range.
pub(crate) fn frame_at(frame: PageFrame, page_offset: usize, page_bytes: usize) -> PageFrame {
    PageFrame {
        offset: frame.offset + (page_offset * page_bytes) as u64,
    }
}

/// One platform page-frame allocator.
#[derive(Debug)]
pub(crate) struct PageFrameAllocator {
    /// The page-frame file descriptor.
    fd: RawFd,
    /// The next unused page-frame byte offset.
    next_offset: Mutex<u64>,
}

impl Drop for PageFrameAllocator {
    fn drop(&mut self) {
        // cleanup cannot report errors from Drop
        let _ = unsafe { libc::close(self.fd) };
    }
}

/// Create one page-frame allocator from an owned descriptor.
pub(crate) fn create_page_frame_allocator_from_fd(
    fd: RawFd,
    _byte_len: usize,
) -> HeapResult<PageFrameAllocator> {
    Ok(PageFrameAllocator {
        fd,
        next_offset: Mutex::new(0),
    })
}

/// Allocate one zeroed page frame.
pub(crate) fn allocate_frame(
    allocator: &PageFrameAllocator,
    page_bytes: usize,
) -> HeapResult<PageFrame> {
    allocate_frame_range(allocator, page_bytes, page_bytes)
}

/// Allocate one zeroed page frame range.
pub(crate) fn allocate_frame_range(
    allocator: &PageFrameAllocator,
    byte_len: usize,
    page_bytes: usize,
) -> HeapResult<PageFrame> {
    let mut next_offset = allocator.next_offset.lock();
    let frame = PageFrame {
        offset: *next_offset,
    };
    let next = frame.offset + byte_len as u64;

    extend_frame_file(allocator.fd, next, page_bytes)?;

    *next_offset = next;

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

    // cleanup cannot report errors after the frame is initialized
    let _ = unsafe { libc::munmap(target.cast(), page_bytes) };

    Ok(frame)
}

/// Return the operating-system page byte width.
pub(crate) fn system_page_bytes() -> HeapResult<usize> {
    let page_bytes = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if page_bytes <= 0 {
        return Err(HeapError::InvariantViolation {
            context: "system page size",
        });
    }

    Ok(page_bytes as usize)
}

/// Reserve one inaccessible virtual address range.
pub(crate) fn reserve_virtual_space(byte_len: usize) -> HeapResult<VirtualSpace> {
    if byte_len == 0 {
        return Ok(VirtualSpace {
            base: null_mut(),
            byte_len,
        });
    }

    let data = unsafe {
        libc::mmap(
            null_mut(),
            byte_len,
            libc::PROT_NONE,
            mmap_private_anonymous_flags(),
            -1,
            0,
        )
    };
    if data == libc::MAP_FAILED {
        return Err(HeapError::AddressSpaceFailed { byte_len });
    }

    Ok(VirtualSpace {
        base: data.cast(),
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
    map_frame_range_shared(base, page_index, page_bytes, page_bytes, allocator, frame)
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
        libc::MAP_PRIVATE,
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
        libc::MAP_SHARED,
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
    flags: libc::c_int,
) -> HeapResult<()> {
    let address = unsafe { base.add(first_page * page_bytes) };
    let data = unsafe {
        libc::mmap(
            address.cast(),
            byte_len,
            libc::PROT_READ | libc::PROT_WRITE,
            flags | libc::MAP_FIXED,
            allocator.fd,
            frame.offset as libc::off_t,
        )
    };
    if data == libc::MAP_FAILED {
        return Err(HeapError::AddressSpaceFailed { byte_len });
    }

    Ok(())
}

/// Map one page frame at any available address.
fn map_frame_anywhere(
    allocator: &PageFrameAllocator,
    frame: PageFrame,
    page_bytes: usize,
) -> HeapResult<*mut u8> {
    let data = unsafe {
        libc::mmap(
            null_mut(),
            page_bytes,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            allocator.fd,
            frame.offset as libc::off_t,
        )
    };
    if data == libc::MAP_FAILED {
        return Err(HeapError::AddressSpaceFailed {
            byte_len: page_bytes,
        });
    }

    Ok(data.cast())
}

/// Return anonymous private mmap flags.
fn mmap_private_anonymous_flags() -> i32 {
    libc::MAP_PRIVATE
        | if cfg!(any(target_os = "macos", target_os = "ios")) {
            libc::MAP_ANON
        } else {
            libc::MAP_ANONYMOUS
        }
}

/// Extend one page-frame file to the requested byte length.
fn extend_frame_file(fd: RawFd, byte_len: u64, page_bytes: usize) -> HeapResult<()> {
    if byte_len > libc::off_t::MAX as u64 {
        return Err(HeapError::AddressSpaceFailed {
            byte_len: page_bytes,
        });
    }

    let result = unsafe { libc::ftruncate(fd, byte_len as libc::off_t) };
    if result == 0 {
        return Ok(());
    }

    Err(HeapError::AddressSpaceFailed {
        byte_len: page_bytes,
    })
}
