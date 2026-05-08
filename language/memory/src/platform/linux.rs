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
    let name = c"destack-memory";
    let fd = unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC) };
    if fd < 0 {
        return Err(MemoryError::AddressSpaceFailed { byte_len });
    }

    super::unix::create_page_frame_allocator_from_fd(fd, byte_len)
}
