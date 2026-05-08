use std::mem::zeroed;
use std::os::fd::RawFd;
#[cfg(not(any(target_os = "macos", target_os = "ios")))]
use std::ptr::copy_nonoverlapping;
use std::ptr::{null_mut, write_bytes};
use std::sync::OnceLock;

use parking_lot::Mutex;

use crate::address::watch_page_write;
pub(crate) use crate::core::{WriteWatchRegistration, pages, register, unregister};
use crate::{MemoryError, MemoryResult};

/// Whether mapped spaces can share page frames directly.
pub(crate) const SUPPORTS_SHARED_PAGE_FRAMES: bool = true;
/// The previously installed Unix memory fault handlers.
static SIGNAL_HANDLERS: OnceLock<SignalHandlers> = OnceLock::new();

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

/// One platform page-frame allocator.
#[derive(Debug)]
pub(crate) struct PageFrameAllocator {
    /// The page-frame file descriptor.
    fd: RawFd,
    /// The reusable frame state.
    state: Mutex<PageFrameAllocatorState>,
}

impl Drop for PageFrameAllocator {
    fn drop(&mut self) {
        // cleanup cannot report errors from Drop
        let _ = unsafe { libc::close(self.fd) };
    }
}

/// The page-frame file frontier, free ranges, and live references.
#[derive(Debug)]
struct PageFrameAllocatorState {
    /// The next never-allocated byte offset.
    next_offset: u64,
    /// The free page-frame byte ranges.
    free_ranges: Vec<PageFrameRange>,
    /// The live references keyed by page-frame index.
    references: Vec<u32>,
}

/// One reusable range inside the page-frame file.
#[derive(Debug, Clone, Copy)]
struct PageFrameRange {
    /// The first byte offset.
    offset: u64,
    /// The byte length.
    byte_len: u64,
}

/// The signal handlers replaced by the memory write handler.
#[derive(Debug)]
struct SignalHandlers {
    /// The previous segmentation fault handler.
    segmentation: libc::sigaction,
    /// The previous bus fault handler.
    bus: libc::sigaction,
}

// signal actions are immutable after installation
unsafe impl Send for SignalHandlers {}

// signal actions are immutable after installation
unsafe impl Sync for SignalHandlers {}

/// Create one page-frame allocator from an owned descriptor.
pub(crate) fn create_page_frame_allocator_from_fd(
    fd: RawFd,
    _byte_len: usize,
) -> MemoryResult<PageFrameAllocator> {
    Ok(PageFrameAllocator {
        fd,
        state: Mutex::new(PageFrameAllocatorState {
            next_offset: 0,
            free_ranges: Vec::new(),
            references: Vec::new(),
        }),
    })
}

/// Return one page frame inside a contiguous frame range.
pub(crate) fn frame_at(frame: PageFrame, page_offset: usize, page_bytes: usize) -> PageFrame {
    PageFrame {
        offset: frame.offset + (page_offset * page_bytes) as u64,
    }
}

/// Allocate one zeroed page frame range.
pub(crate) fn allocate_frame_range(
    allocator: &PageFrameAllocator,
    byte_len: usize,
    page_bytes: usize,
) -> MemoryResult<PageFrame> {
    let (frame, is_reused) = allocate_frame_storage(allocator, byte_len, page_bytes)?;

    // reused frame-file ranges must regain sparse-page zero semantics
    if is_reused {
        zero_frame_range(allocator, frame, byte_len)?;
    }

    Ok(frame)
}

/// Retain mapped page frames.
pub(crate) fn retain_frames<I>(allocator: &PageFrameAllocator, frames: I, page_bytes: usize)
where
    I: IntoIterator<Item = PageFrame>,
{
    let mut state = allocator.state.lock();

    for frame in frames {
        let index = frame_index(frame, page_bytes);
        let references = &mut state.references[index];

        *references += 1;
    }
}

/// Release mapped page frames.
pub(crate) fn release_frames<I>(allocator: &PageFrameAllocator, frames: I, page_bytes: usize)
where
    I: IntoIterator<Item = PageFrame>,
{
    let mut state = allocator.state.lock();

    for frame in frames {
        let index = frame_index(frame, page_bytes);
        let references = &mut state.references[index];

        // keep shared frames live until the last mapping drops
        *references -= 1;
        if *references != 0 {
            continue;
        }

        state.free_ranges.push(PageFrameRange {
            offset: frame.offset,
            byte_len: page_bytes as u64,
        });
    }

    // merge once after the whole release batch
    merge_free_frame_ranges(&mut state.free_ranges);
}

/// Copy one mapped page into a fresh page frame.
pub(crate) fn copy_page(
    allocator: &PageFrameAllocator,
    source: *mut u8,
    page_bytes: usize,
) -> MemoryResult<PageFrame> {
    copy_frame_range(allocator, source, page_bytes, page_bytes)
}

/// Copy one mapped byte range into a fresh page-frame range.
pub(crate) fn copy_frame_range(
    allocator: &PageFrameAllocator,
    source: *mut u8,
    byte_len: usize,
    page_bytes: usize,
) -> MemoryResult<PageFrame> {
    let (frame, _) = allocate_frame_storage(allocator, byte_len, page_bytes)?;

    // copy current bytes directly into the backing frame file
    write_frame_range(allocator.fd, frame, source, byte_len)?;

    Ok(frame)
}

/// Return the operating-system page byte width.
pub(crate) fn system_page_bytes() -> MemoryResult<usize> {
    let page_bytes = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if page_bytes <= 0 {
        return Err(MemoryError::InvariantViolation {
            context: "system page size",
        });
    }

    Ok(page_bytes as usize)
}

/// Reserve one inaccessible virtual address range.
pub(crate) fn reserve_virtual_space(byte_len: usize) -> MemoryResult<VirtualSpace> {
    if byte_len == 0 {
        return Ok(VirtualSpace {
            base: null_mut(),
            byte_len,
        });
    }

    // reserve address space without committing mapped pages
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
        return Err(MemoryError::AddressSpaceFailed { byte_len });
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
) -> MemoryResult<()> {
    map_frame_range_shared(base, page_index, page_bytes, page_bytes, allocator, frame)
}

/// Map one page-frame range as clean private memory.
pub(crate) fn map_frame_range_clean(
    base: *mut u8,
    first_page: usize,
    page_bytes: usize,
    byte_len: usize,
    allocator: &PageFrameAllocator,
    frame: PageFrame,
) -> MemoryResult<()> {
    map_frame_range(
        base,
        first_page,
        page_bytes,
        byte_len,
        allocator,
        frame,
        libc::PROT_READ,
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
) -> MemoryResult<()> {
    map_frame_range(
        base,
        first_page,
        page_bytes,
        byte_len,
        allocator,
        frame,
        libc::PROT_READ | libc::PROT_WRITE,
        libc::MAP_SHARED,
    )
}

/// Fork current dirty bytes into one child byte range.
pub(crate) fn fork_dirty_pages(
    base: *mut u8,
    first_page: usize,
    page_bytes: usize,
    byte_len: usize,
    source: *mut u8,
) -> MemoryResult<()> {
    fork_dirty_pages_platform(base, first_page, page_bytes, byte_len, source)
}

/// Make clean private pages writable after a watched write.
pub(crate) fn make_clean_pages_writable(
    base: *mut u8,
    first_page: usize,
    page_bytes: usize,
    byte_len: usize,
) -> MemoryResult<()> {
    let address = unsafe { base.add(first_page * page_bytes) };
    let result =
        unsafe { libc::mprotect(address.cast(), byte_len, libc::PROT_READ | libc::PROT_WRITE) };
    if result == 0 {
        return Ok(());
    }

    Err(MemoryError::AddressSpaceFailed { byte_len })
}

/// Register one write watched virtual range.
pub(crate) fn register_write_watch(
    base: *mut u8,
    byte_len: usize,
    context: *const (),
) -> MemoryResult<WriteWatchRegistration> {
    let registration = register(base, byte_len, context)?;

    install_write_fault_handler();

    Ok(registration)
}

/// Unregister one write watched virtual range.
pub(crate) fn unregister_write_watch(registration: &WriteWatchRegistration) {
    unregister(registration);
}

/// Allocate backing storage for one page-frame range.
fn allocate_frame_storage(
    allocator: &PageFrameAllocator,
    byte_len: usize,
    page_bytes: usize,
) -> MemoryResult<(PageFrame, bool)> {
    let mut state = allocator.state.lock();

    // reuse a returned frame range before extending the frame file
    let (frame, is_reused) = if let Some(frame) = allocate_free_frame_range(&mut state, byte_len) {
        (frame, true)
    } else {
        let frame = PageFrame {
            offset: state.next_offset,
        };
        let next_offset = frame.offset + byte_len as u64;

        extend_frame_file(allocator.fd, next_offset, page_bytes)?;
        state.next_offset = next_offset;

        (frame, false)
    };

    // each page starts with one owning mapping
    initialize_frame_references(&mut state, frame, byte_len, page_bytes);

    Ok((frame, is_reused))
}

/// Allocate one free frame range when a large enough range exists.
fn allocate_free_frame_range(
    state: &mut PageFrameAllocatorState,
    byte_len: usize,
) -> Option<PageFrame> {
    let byte_len = byte_len as u64;
    let range_index = state
        .free_ranges
        .iter()
        .position(|range| range.byte_len >= byte_len)?;
    let range = &mut state.free_ranges[range_index];
    let frame = PageFrame {
        offset: range.offset,
    };

    // consume the front of the free range
    range.offset += byte_len;
    range.byte_len -= byte_len;

    if range.byte_len == 0 {
        state.free_ranges.swap_remove(range_index);
    }

    Some(frame)
}

/// Initialize one reference count for every page in a frame range.
fn initialize_frame_references(
    state: &mut PageFrameAllocatorState,
    frame: PageFrame,
    byte_len: usize,
    page_bytes: usize,
) {
    let page_count = byte_len / page_bytes;

    for page_offset in 0..page_count {
        let frame = frame_at(frame, page_offset, page_bytes);
        let index = frame_index(frame, page_bytes);

        if index >= state.references.len() {
            state.references.resize(index + 1, 0);
        }

        state.references[index] = 1;
    }
}

/// Return the dense index for one page frame.
fn frame_index(frame: PageFrame, page_bytes: usize) -> usize {
    (frame.offset as usize) / page_bytes
}

/// Merge adjacent free page-frame ranges.
fn merge_free_frame_ranges(ranges: &mut Vec<PageFrameRange>) {
    ranges.sort_by_key(|range| range.offset);

    let mut range_index = 0;
    while range_index + 1 < ranges.len() {
        let current_end = ranges[range_index].offset + ranges[range_index].byte_len;

        // adjacent ranges become one reusable range
        if current_end == ranges[range_index + 1].offset {
            let next = ranges.remove(range_index + 1);
            ranges[range_index].byte_len += next.byte_len;
            continue;
        }

        range_index += 1;
    }
}

/// Map one page-frame range into reserved virtual pages.
fn map_frame_range(
    base: *mut u8,
    first_page: usize,
    page_bytes: usize,
    byte_len: usize,
    allocator: &PageFrameAllocator,
    frame: PageFrame,
    protection: libc::c_int,
    flags: libc::c_int,
) -> MemoryResult<()> {
    // replace the reserved range with a file-backed view
    let address = unsafe { base.add(first_page * page_bytes) };
    let data = unsafe {
        libc::mmap(
            address.cast(),
            byte_len,
            protection,
            flags | libc::MAP_FIXED,
            allocator.fd,
            frame.offset as libc::off_t,
        )
    };
    if data == libc::MAP_FAILED {
        return Err(MemoryError::AddressSpaceFailed { byte_len });
    }

    Ok(())
}

/// Fork current dirty bytes into one child byte range.
#[cfg(any(target_os = "macos", target_os = "ios"))]
fn fork_dirty_pages_platform(
    base: *mut u8,
    first_page: usize,
    page_bytes: usize,
    byte_len: usize,
    source: *mut u8,
) -> MemoryResult<()> {
    let target = unsafe { base.add(first_page * page_bytes) };
    let task = unsafe { MACH_TASK_SELF };

    // mach copy installs the current bytes into the child mapping
    map_private_writable(base, first_page, page_bytes, byte_len)?;
    let result = unsafe {
        mach_vm_copy(
            task,
            source as libc::mach_vm_address_t,
            byte_len as libc::mach_vm_size_t,
            target as libc::mach_vm_address_t,
        )
    };
    if result != libc::KERN_SUCCESS {
        return Err(MemoryError::AddressSpaceFailed { byte_len });
    }

    Ok(())
}

/// Fork current dirty bytes into one child byte range.
#[cfg(not(any(target_os = "macos", target_os = "ios")))]
fn fork_dirty_pages_platform(
    base: *mut u8,
    first_page: usize,
    page_bytes: usize,
    byte_len: usize,
    source: *mut u8,
) -> MemoryResult<()> {
    let address = map_private_writable(base, first_page, page_bytes, byte_len)?;

    unsafe {
        copy_nonoverlapping(source, address, byte_len);
    }

    Ok(())
}

/// Map one private writable byte range.
fn map_private_writable(
    base: *mut u8,
    first_page: usize,
    page_bytes: usize,
    byte_len: usize,
) -> MemoryResult<*mut u8> {
    let address = unsafe { base.add(first_page * page_bytes) };

    // replace the reserved range with private committed pages
    let data = unsafe {
        libc::mmap(
            address.cast(),
            byte_len,
            libc::PROT_READ | libc::PROT_WRITE,
            mmap_private_anonymous_flags() | libc::MAP_FIXED,
            -1,
            0,
        )
    };
    if data == libc::MAP_FAILED {
        return Err(MemoryError::AddressSpaceFailed { byte_len });
    }

    Ok(address)
}

/// Map one frame range at any available address.
fn map_frame_range_anywhere(
    allocator: &PageFrameAllocator,
    frame: PageFrame,
    byte_len: usize,
) -> MemoryResult<*mut u8> {
    let data = unsafe {
        libc::mmap(
            null_mut(),
            byte_len,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            allocator.fd,
            frame.offset as libc::off_t,
        )
    };
    if data == libc::MAP_FAILED {
        return Err(MemoryError::AddressSpaceFailed { byte_len });
    }

    Ok(data.cast())
}

/// Zero one reusable frame range.
fn zero_frame_range(
    allocator: &PageFrameAllocator,
    frame: PageFrame,
    byte_len: usize,
) -> MemoryResult<()> {
    let target = map_frame_range_anywhere(allocator, frame, byte_len)?;

    unsafe {
        write_bytes(target, 0, byte_len);
    }

    // cleanup cannot report errors after the frame is zeroed
    let _ = unsafe { libc::munmap(target.cast(), byte_len) };

    Ok(())
}

/// Write one mapped byte range into the backing frame file.
fn write_frame_range(
    fd: RawFd,
    frame: PageFrame,
    source: *mut u8,
    byte_len: usize,
) -> MemoryResult<()> {
    let mut written = 0;

    // pwrite copies directly into the frame file without scratch mapping
    while written < byte_len {
        let source = unsafe { source.add(written) };
        let offset = frame.offset + written as u64;
        let remaining = byte_len - written;
        let result = unsafe { libc::pwrite(fd, source.cast(), remaining, offset as libc::off_t) };

        if result <= 0 {
            return Err(MemoryError::AddressSpaceFailed { byte_len });
        }

        written += result as usize;
    }

    Ok(())
}

/// Extend one page-frame file to the requested byte length.
fn extend_frame_file(fd: RawFd, byte_len: u64, page_bytes: usize) -> MemoryResult<()> {
    if byte_len > libc::off_t::MAX as u64 {
        return Err(MemoryError::AddressSpaceFailed {
            byte_len: page_bytes,
        });
    }

    let result = unsafe { libc::ftruncate(fd, byte_len as libc::off_t) };
    if result == 0 {
        return Ok(());
    }

    Err(MemoryError::AddressSpaceFailed {
        byte_len: page_bytes,
    })
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

/// Install the write fault handler once.
fn install_write_fault_handler() {
    SIGNAL_HANDLERS.get_or_init(|| {
        // install one process-level handler for protected pages
        let mut action = unsafe { zeroed::<libc::sigaction>() };
        let mut segmentation = unsafe { zeroed::<libc::sigaction>() };
        let mut bus = unsafe { zeroed::<libc::sigaction>() };
        action.sa_flags = libc::SA_SIGINFO;
        action.sa_sigaction = handle_write_watch as *const () as usize;

        unsafe {
            libc::sigemptyset(&mut action.sa_mask);
            libc::sigaction(libc::SIGSEGV, &action, &mut segmentation);
            libc::sigaction(libc::SIGBUS, &action, &mut bus);
        }

        SignalHandlers { segmentation, bus }
    });
}

/// Restore the previous signal handler and raise the signal again.
fn raise_unhandled_signal(signal: libc::c_int) {
    let Some(handlers) = SIGNAL_HANDLERS.get() else {
        unsafe {
            libc::signal(signal, libc::SIG_DFL);
            libc::raise(signal);
        }

        return;
    };

    let previous = match signal {
        libc::SIGSEGV => &handlers.segmentation,
        libc::SIGBUS => &handlers.bus,
        _ => {
            unsafe {
                libc::signal(signal, libc::SIG_DFL);
                libc::raise(signal);
            }

            return;
        }
    };

    unsafe {
        libc::sigaction(signal, previous, null_mut());
        libc::raise(signal);
    }
}

/// Handle one watched page write.
unsafe extern "C" fn handle_write_watch(
    signal: libc::c_int,
    signal_info: *mut libc::siginfo_t,
    _context: *mut libc::c_void,
) {
    let address = unsafe { (*signal_info).si_addr() as usize };

    // scan watched ranges without signal-unsafe locks
    for page in pages() {
        let Some(entries) = page.entries() else {
            continue;
        };

        for entry in entries {
            let Some(context) = entry.context(address) else {
                continue;
            };

            if unsafe { watch_page_write(context, address) } {
                return;
            }
        }
    }

    // raise unrelated signals through the previous platform handler
    raise_unhandled_signal(signal);
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
unsafe extern "C" {
    /// The current Mach task port.
    #[link_name = "mach_task_self_"]
    static MACH_TASK_SELF: libc::mach_port_t;

    /// Copy one virtual memory range inside one Mach task.
    fn mach_vm_copy(
        target_task: libc::vm_map_t,
        source_address: libc::mach_vm_address_t,
        size: libc::mach_vm_size_t,
        target_address: libc::mach_vm_address_t,
    ) -> libc::kern_return_t;
}
