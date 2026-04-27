#![cfg(all(unix, not(any(target_os = "linux", target_os = "android"))))]

use std::os::unix::ffi::OsStrExt;

use crate::{HeapError, HeapResult};

pub(crate) use super::unix::{
    PageFrame, PageStoreHandle, SUPPORTS_SHARED_PAGE_FRAMES, VirtualSpace, allocate_frame,
    commit_chunk_space, copy_page, map_page, reserve_chunk_space, reserve_virtual_space,
    system_page_bytes, unmap_chunk_space,
};

/// Create one page store.
pub(crate) fn create_page_store(byte_len: usize) -> HeapResult<PageStoreHandle> {
    let mut path = std::env::temp_dir();
    path.push("ds-heap-XXXXXX");

    let mut path = path.as_os_str().as_bytes().to_vec();
    path.push(0);

    let fd = unsafe { libc::mkstemp(path.as_mut_ptr().cast()) };
    if fd < 0 {
        return Err(HeapError::AddressSpaceFailed { byte_len });
    }

    // unlink immediately so the descriptor is the only owner
    let _ = unsafe { libc::unlink(path.as_ptr().cast()) };

    super::unix::create_page_store_from_fd(fd, byte_len)
}
