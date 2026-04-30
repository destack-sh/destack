#![cfg(any(target_os = "linux", target_os = "android"))]

use crate::{HeapError, HeapResult};

pub(crate) use super::unix::{
    PageFrame, PageFrameAllocator, SUPPORTS_SHARED_PAGE_FRAMES, VirtualSpace, allocate_frame_range,
    copy_page, frame_at, map_frame_range_private, map_frame_range_shared, map_page_shared,
    reserve_virtual_space, system_page_bytes,
};

/// Create one page-frame allocator.
pub(crate) fn create_page_frame_allocator(byte_len: usize) -> HeapResult<PageFrameAllocator> {
    let name = c"destack-heap";
    let fd = unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC) };
    if fd < 0 {
        return Err(HeapError::AddressSpaceFailed { byte_len });
    }

    super::unix::create_page_frame_allocator_from_fd(fd, byte_len)
}
