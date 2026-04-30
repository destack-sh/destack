#![cfg(all(unix, not(any(target_os = "linux", target_os = "android"))))]

use std::os::unix::ffi::OsStrExt;

use crate::{HeapError, HeapResult};

pub(crate) use super::unix::{
    PageFrame, PageFrameAllocator, SUPPORTS_SHARED_PAGE_FRAMES, VirtualSpace, allocate_frame_range,
    copy_page, frame_at, map_frame_range_private, map_frame_range_shared, map_page_shared,
    reserve_virtual_space, system_page_bytes,
};

/// Create one page-frame allocator.
pub(crate) fn create_page_frame_allocator(byte_len: usize) -> HeapResult<PageFrameAllocator> {
    let mut path = std::env::temp_dir();
    path.push("ds-heap-XXXXXX");

    let mut path = path.as_os_str().as_bytes().to_vec();
    path.push(0);

    let fd = unsafe { libc::mkstemp(path.as_mut_ptr().cast()) };
    if fd < 0 {
        return Err(HeapError::AddressSpaceFailed { byte_len });
    }

    // unlink immediately so the descriptor is the only reference
    let _ = unsafe { libc::unlink(path.as_ptr().cast()) };

    super::unix::create_page_frame_allocator_from_fd(fd, byte_len)
}
