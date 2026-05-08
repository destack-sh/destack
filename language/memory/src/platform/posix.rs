use std::env::temp_dir;
use std::os::unix::ffi::OsStrExt;

use crate::{MemoryError, MemoryResult};

pub(crate) use super::unix::{
    PageFrame, PageFrameAllocator, SUPPORTS_SHARED_PAGE_FRAMES, VirtualSpace,
    WriteWatchRegistration, allocate_frame_range, copy_page, fork_dirty_pages, frame_at,
    make_clean_pages_writable, map_frame_range_clean, map_frame_range_shared, map_page_shared,
    register_write_watch, release_frames, reserve_virtual_space, retain_frames, system_page_bytes,
    unregister_write_watch,
};

/// Create one page-frame allocator.
pub(crate) fn create_page_frame_allocator(byte_len: usize) -> MemoryResult<PageFrameAllocator> {
    let mut path = temp_dir();
    path.push("ds-memory-XXXXXX");

    let mut path = path.as_os_str().as_bytes().to_vec();
    path.push(0);

    let fd = unsafe { libc::mkstemp(path.as_mut_ptr().cast()) };
    if fd < 0 {
        return Err(MemoryError::AddressSpaceFailed { byte_len });
    }

    // unlink immediately so the descriptor is the only reference
    let _ = unsafe { libc::unlink(path.as_ptr().cast()) };

    super::unix::create_page_frame_allocator_from_fd(fd, byte_len)
}
