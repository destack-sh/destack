#![cfg(any(target_os = "linux", target_os = "android"))]

use crate::{HeapError, HeapResult};

pub(crate) use super::unix::{
    PageFrame, PageStoreHandle, SUPPORTS_SHARED_PAGE_FRAMES, VirtualSpace, allocate_frame,
    commit_chunk_space, copy_page, map_page, reserve_chunk_space, reserve_virtual_space,
    system_page_bytes, unmap_chunk_space,
};

/// Create one page store.
pub(crate) fn create_page_store(byte_len: usize) -> HeapResult<PageStoreHandle> {
    let name = c"destack-heap";
    let fd = unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC) };
    if fd < 0 {
        return Err(HeapError::AddressSpaceFailed { byte_len });
    }

    super::unix::create_page_store_from_fd(fd, byte_len)
}
