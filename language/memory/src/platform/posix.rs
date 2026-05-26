use std::env::temp_dir;
use std::io::Error as IoError;
use std::os::unix::ffi::OsStrExt;

use crate::{MemoryError, MemoryOperation, MemoryResult};

pub(crate) use super::unix::{
    PageFrame, PageFrameAllocator, SUPPORTS_SHARED_PAGE_FRAMES, VirtualSpace,
    WriteWatchRegistration, allocate_frame_range, copy_frame_range, copy_page, frame_at,
    make_shared_pages_writable, map_frame_range_cow, map_frame_range_writable, map_page_writable,
    register_write_watch, release_frames, reserve_virtual_space, retain_frames, system_frame_bytes,
    unregister_write_watch,
};

/// Create one page frame allocator.
pub(crate) fn create_page_frame_allocator(byte_len: usize) -> MemoryResult<PageFrameAllocator> {
    let mut path = temp_dir();
    path.push("ds-memory-XXXXXX");

    let mut path = path.as_os_str().as_bytes().to_vec();
    path.push(0);

    // SAFETY: path is a writable nul terminated mkstemp template
    let fd = unsafe { libc::mkstemp(path.as_mut_ptr().cast()) };
    if fd < 0 {
        return Err(MemoryError::system_bytes(
            MemoryOperation::CreateFrameAllocator,
            IoError::last_os_error().raw_os_error(),
            byte_len,
        ));
    }

    // unlink immediately so the descriptor is the only reference
    // SAFETY: path remains nul terminated after mkstemp returns
    let _ = unsafe { libc::unlink(path.as_ptr().cast()) };

    super::unix::create_page_frame_allocator_from_fd(fd, byte_len)
}
