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

/// One page-sized frame in the page store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PageFrame {
    /// The byte offset of this frame inside the page store.
    pub(crate) offset: u64,
}

/// One platform page-store handle.
#[derive(Debug)]
pub(crate) struct PageStoreHandle {
    /// The page-store file descriptor.
    fd: RawFd,
    /// The next unused page-store byte offset.
    next_offset: Mutex<u64>,
}

impl Drop for PageStoreHandle {
    fn drop(&mut self) {
        // cleanup cannot report errors from Drop
        let _ = unsafe { libc::close(self.fd) };
    }
}

/// Create one page store from an owned descriptor.
pub(crate) fn create_page_store_from_fd(
    fd: RawFd,
    _byte_len: usize,
) -> HeapResult<PageStoreHandle> {
    Ok(PageStoreHandle {
        fd,
        next_offset: Mutex::new(0),
    })
}

/// Allocate one zeroed page frame.
pub(crate) fn allocate_frame(handle: &PageStoreHandle, page_bytes: usize) -> HeapResult<PageFrame> {
    let mut next_offset = handle.next_offset.lock();
    let frame = PageFrame {
        offset: *next_offset,
    };
    let next = frame.offset + page_bytes as u64;

    extend_page_store(handle.fd, next, page_bytes)?;

    *next_offset = next;

    Ok(frame)
}

/// Copy one mapped page into a fresh page frame.
pub(crate) fn copy_page(
    handle: &PageStoreHandle,
    source: *mut u8,
    page_bytes: usize,
) -> HeapResult<PageFrame> {
    let frame = allocate_frame(handle, page_bytes)?;
    let target = map_frame_anywhere(handle, frame, page_bytes)?;

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

/// Reserve one inaccessible virtual address range for allocator chunks.
pub(crate) fn reserve_chunk_space(byte_len: usize) -> HeapResult<*mut u8> {
    reserve_virtual_space(byte_len)
        .map(|space| space.base())
        .map_err(|_| HeapError::AllocatorChunkAllocationFailed { byte_len })
}

/// Commit one chunk range for allocator payloads.
pub(crate) fn commit_chunk_space(data: *mut u8, byte_len: usize) -> HeapResult<()> {
    let result =
        unsafe { libc::mprotect(data.cast(), byte_len, libc::PROT_READ | libc::PROT_WRITE) };
    if result == 0 {
        return Ok(());
    }

    Err(HeapError::AllocatorChunkAllocationFailed { byte_len })
}

/// Unmap one chunk range and report unexpected OS failure.
pub(crate) fn unmap_chunk_space(data: *mut u8, byte_len: usize) -> HeapResult<()> {
    if byte_len == 0 {
        return Ok(());
    }

    let result = unsafe { libc::munmap(data.cast(), byte_len) };
    if result == 0 {
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
    let data = unsafe {
        libc::mmap(
            address.cast(),
            page_bytes,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_FIXED,
            handle.fd,
            frame.offset as libc::off_t,
        )
    };
    if data == libc::MAP_FAILED {
        return Err(HeapError::AddressSpaceFailed {
            byte_len: page_bytes,
        });
    }

    Ok(())
}

/// Map one page-store frame at any available address.
fn map_frame_anywhere(
    handle: &PageStoreHandle,
    frame: PageFrame,
    page_bytes: usize,
) -> HeapResult<*mut u8> {
    let data = unsafe {
        libc::mmap(
            null_mut(),
            page_bytes,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            handle.fd,
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

/// Extend one page store to the requested byte length.
fn extend_page_store(fd: RawFd, byte_len: u64, page_bytes: usize) -> HeapResult<()> {
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
