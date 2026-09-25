use std::io::Error as IoError;

use crate::{MemoryError, MemoryOperation, MemoryResult};

pub(crate) use super::unix::{
    PageFrame, PageFrameAllocator, SUPPORTS_SHARED_PAGE_FRAMES, VirtualSpace,
    WriteWatchRegistration, allocate_frame_range, copy_frame_range, copy_page, frame_at,
    make_shared_pages_writable, map_frame_range_readonly, map_frame_range_writable,
    map_page_writable, register_write_watch, release_frames, remap_frame_range_readonly,
    reserve_virtual_space, retain_frames, system_page_size_bytes, unregister_write_watch,
};

/// Create one page frame allocator.
pub(crate) fn create_page_frame_allocator(byte_len: usize) -> MemoryResult<PageFrameAllocator> {
    let name = c"tspp-memory";

    // SAFETY: name is a static nul terminated C string
    let fd = unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC) };
    if fd < 0 {
        return Err(MemoryError::system_bytes(
            MemoryOperation::CreateFrameAllocator,
            IoError::last_os_error().raw_os_error(),
            byte_len,
        ));
    }

    super::unix::create_page_frame_allocator_from_fd(fd, byte_len)
}
