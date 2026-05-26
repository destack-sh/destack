use std::mem::zeroed;
use std::os::fd::RawFd;
use std::ptr::{null_mut, write_bytes};
use std::sync::OnceLock;

use parking_lot::Mutex;

use crate::address::watch_page_write;
pub(crate) use crate::core::{WriteWatchRegistration, WriteWatchTable};
use crate::{MemoryError, MemoryOperation, MemoryResult};

/// Whether mapped spaces can share page frames directly.
pub(crate) const SUPPORTS_SHARED_PAGE_FRAMES: bool = true;
/// Anonymous mapping flag on Darwin targets.
#[cfg(target_os = "macos")]
const MAP_ANONYMOUS: libc::c_int = libc::MAP_ANON;
/// Anonymous mapping flag on non-Darwin Unix targets.
#[cfg(not(target_os = "macos"))]
const MAP_ANONYMOUS: libc::c_int = libc::MAP_ANONYMOUS;
/// Private anonymous mapping flags for reserved address ranges.
const MAP_PRIVATE_ANONYMOUS: libc::c_int = libc::MAP_PRIVATE | MAP_ANONYMOUS;
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
        let _ = unmap_virtual(self.base, self.byte_len);
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
        let _ = close_fd(self.fd);
    }
}

/// The page-frame file frontier, free ranges, and live reference counts.
#[derive(Debug)]
struct PageFrameAllocatorState {
    /// The next never-allocated byte offset.
    next_offset: u64,
    /// The free page-frame byte ranges.
    free_ranges: Vec<PageFrameRange>,
    /// The live reference counts keyed by page-frame index.
    frame_ref_counts: Vec<u32>,
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

// SAFETY: signal actions are immutable after installation
unsafe impl Send for SignalHandlers {}

// SAFETY: signal actions are immutable after installation
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
            frame_ref_counts: Vec::new(),
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

    // reused frame-file ranges must regain reserved-page zero semantics
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
        let ref_count = &mut state.frame_ref_counts[index];

        *ref_count += 1;
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
        let ref_count = &mut state.frame_ref_counts[index];

        // keep shared frames live until the last mapping drops
        *ref_count -= 1;
        if *ref_count != 0 {
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

/// Return the platform frame byte width for fixed-address mappings.
pub(crate) fn system_frame_bytes() -> MemoryResult<usize> {
    let page_bytes = system_page_bytes();
    if page_bytes <= 0 {
        return Err(MemoryError::InvariantViolation {
            context: "system frame size",
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
    let address = map_anonymous(byte_len, libc::PROT_NONE);
    if address == libc::MAP_FAILED {
        return Err(last_system_error(
            MemoryOperation::ReserveAddressSpace,
            byte_len,
        ));
    }

    Ok(VirtualSpace {
        base: address.cast(),
        byte_len,
    })
}

/// Map one page frame as writable memory.
pub(crate) fn map_page_writable(
    base: *mut u8,
    page_index: usize,
    page_bytes: usize,
    allocator: &PageFrameAllocator,
    frame: PageFrame,
) -> MemoryResult<()> {
    map_frame_range_writable(base, page_index, page_bytes, page_bytes, allocator, frame)
}

/// Map one page-frame range as copy-on-write memory.
pub(crate) fn map_frame_range_cow(
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

/// Map one page-frame range as writable memory.
pub(crate) fn map_frame_range_writable(
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

/// Make shared pages writable after a watched write.
pub(crate) fn make_shared_pages_writable(
    base: *mut u8,
    first_page: usize,
    page_bytes: usize,
    byte_len: usize,
) -> MemoryResult<()> {
    let address = page_address(base, first_page, page_bytes);
    let result = protect_pages(address, byte_len, libc::PROT_READ | libc::PROT_WRITE);
    if result == 0 {
        return Ok(());
    }

    Err(last_system_error(MemoryOperation::ProtectPages, byte_len))
}

/// Register one write watched virtual range.
pub(crate) fn register_write_watch(
    base: *mut u8,
    byte_len: usize,
    context: *const (),
) -> MemoryResult<WriteWatchRegistration> {
    let registration = WriteWatchTable::register(base, byte_len, context)?;

    install_write_fault_handler();

    Ok(registration)
}

/// Unregister one write watched virtual range.
pub(crate) fn unregister_write_watch(registration: &WriteWatchRegistration) {
    WriteWatchTable::unregister(registration);
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

        if index >= state.frame_ref_counts.len() {
            state.frame_ref_counts.resize(index + 1, 0);
        }

        state.frame_ref_counts[index] = 1;
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

/// Return one byte address inside a raw byte range.
fn byte_address(base: *mut u8, byte_offset: usize) -> *mut u8 {
    // SAFETY: callers pass offsets inside a reserved virtual range
    unsafe { base.add(byte_offset) }
}

/// Return one page address inside a reserved range.
fn page_address(base: *mut u8, page_index: usize, page_bytes: usize) -> *mut u8 {
    byte_address(base, page_index * page_bytes)
}

/// Return the platform page byte width.
fn system_page_bytes() -> libc::c_long {
    // SAFETY: sysconf reads process configuration and does not retain pointers
    unsafe { libc::sysconf(libc::_SC_PAGESIZE) }
}

/// Close one file descriptor.
fn close_fd(fd: RawFd) -> libc::c_int {
    // SAFETY: fd is owned by the page-frame allocator
    unsafe { libc::close(fd) }
}

/// Reserve one anonymous virtual address range.
fn map_anonymous(byte_len: usize, protection: libc::c_int) -> *mut libc::c_void {
    // SAFETY: null address lets the kernel choose a range, result is checked by caller
    unsafe {
        libc::mmap(
            null_mut(),
            byte_len,
            protection,
            MAP_PRIVATE_ANONYMOUS,
            -1,
            0,
        )
    }
}

/// Map one frame range into one fixed address.
fn map_frame_fixed(
    address: *mut u8,
    byte_len: usize,
    protection: libc::c_int,
    flags: libc::c_int,
    fd: RawFd,
    offset: u64,
) -> *mut libc::c_void {
    // SAFETY: address is reserved by this address space and the fd owns frame storage
    unsafe {
        libc::mmap(
            address.cast(),
            byte_len,
            protection,
            flags | libc::MAP_FIXED,
            fd,
            offset as libc::off_t,
        )
    }
}

/// Map one frame range at any available address.
fn map_frame_anywhere(fd: RawFd, offset: u64, byte_len: usize) -> *mut libc::c_void {
    // SAFETY: fd owns frame storage and result is checked by caller
    unsafe {
        libc::mmap(
            null_mut(),
            byte_len,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            fd,
            offset as libc::off_t,
        )
    }
}

/// Update page protection for one range.
fn protect_pages(address: *mut u8, byte_len: usize, protection: libc::c_int) -> libc::c_int {
    // SAFETY: address and length describe mapped pages owned by this address space
    unsafe { libc::mprotect(address.cast(), byte_len, protection) }
}

/// Unmap one virtual range.
fn unmap_virtual(address: *mut u8, byte_len: usize) -> libc::c_int {
    // SAFETY: address and length describe a mapping owned by this address space
    unsafe { libc::munmap(address.cast(), byte_len) }
}

/// Fill one byte range with zero.
fn write_zero_bytes(address: *mut u8, byte_len: usize) {
    // SAFETY: caller maps the scratch frame range writable before zeroing
    unsafe {
        write_bytes(address, 0, byte_len);
    }
}

/// Write bytes at one file offset.
fn write_at(fd: RawFd, source: *mut u8, byte_len: usize, offset: u64) -> isize {
    // SAFETY: source is a mapped byte range and fd owns frame storage
    unsafe { libc::pwrite(fd, source.cast(), byte_len, offset as libc::off_t) }
}

/// Truncate one file to the given byte length.
fn truncate_file(fd: RawFd, byte_len: u64) -> libc::c_int {
    // SAFETY: fd is owned by the page-frame allocator
    unsafe { libc::ftruncate(fd, byte_len as libc::off_t) }
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
    let address = page_address(base, first_page, page_bytes);
    let mapped = map_frame_fixed(
        address,
        byte_len,
        protection,
        flags,
        allocator.fd,
        frame.offset,
    );
    if mapped == libc::MAP_FAILED {
        return Err(last_system_error(MemoryOperation::MapFrameRange, byte_len));
    }

    Ok(())
}

/// Map one frame range at any available address.
fn map_frame_range_anywhere(
    allocator: &PageFrameAllocator,
    frame: PageFrame,
    byte_len: usize,
) -> MemoryResult<*mut u8> {
    let address = map_frame_anywhere(allocator.fd, frame.offset, byte_len);
    if address == libc::MAP_FAILED {
        return Err(last_system_error(MemoryOperation::MapFrameRange, byte_len));
    }

    Ok(address.cast())
}

/// Zero one reusable frame range.
fn zero_frame_range(
    allocator: &PageFrameAllocator,
    frame: PageFrame,
    byte_len: usize,
) -> MemoryResult<()> {
    let target = map_frame_range_anywhere(allocator, frame, byte_len)?;

    write_zero_bytes(target, byte_len);

    // cleanup cannot report errors after the frame is zeroed
    let _ = unmap_virtual(target, byte_len);

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
        let source = byte_address(source, written);
        let offset = frame.offset + written as u64;
        let remaining = byte_len - written;
        let result = write_at(fd, source, remaining, offset);

        if result <= 0 {
            return Err(last_system_error(
                MemoryOperation::CopyFrameStorage,
                byte_len,
            ));
        }

        written += result as usize;
    }

    Ok(())
}

/// Extend one page-frame file to the requested byte length.
fn extend_frame_file(fd: RawFd, byte_len: u64, page_bytes: usize) -> MemoryResult<()> {
    if byte_len > libc::off_t::MAX as u64 {
        return Err(MemoryError::system(
            MemoryOperation::ExtendFrameAllocator,
            page_bytes,
        ));
    }

    let result = truncate_file(fd, byte_len);
    if result == 0 {
        return Ok(());
    }

    Err(last_system_error(
        MemoryOperation::ExtendFrameAllocator,
        page_bytes,
    ))
}

/// Return one system error from the last platform error code.
fn last_system_error(operation: MemoryOperation, byte_len: usize) -> MemoryError {
    MemoryError::system_with_code(
        operation,
        std::io::Error::last_os_error().raw_os_error(),
        byte_len,
    )
}

/// Install the write fault handler once.
fn install_write_fault_handler() {
    SIGNAL_HANDLERS.get_or_init(|| {
        // install one process-level handler for protected pages
        // SAFETY: zeroed sigaction is filled before installation
        let mut action = unsafe { zeroed::<libc::sigaction>() };
        // SAFETY: zeroed storage is passed to sigaction as an out parameter
        let mut segmentation = unsafe { zeroed::<libc::sigaction>() };
        // SAFETY: zeroed storage is passed to sigaction as an out parameter
        let mut bus = unsafe { zeroed::<libc::sigaction>() };
        action.sa_flags = libc::SA_SIGINFO;
        action.sa_sigaction = handle_write_watch as *const () as usize;

        // SAFETY: action contains a valid SA_SIGINFO handler
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
        // SAFETY: restoring the default handler before re-raising delegates the fault
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
            // SAFETY: restoring the default handler before re-raising delegates the fault
            unsafe {
                libc::signal(signal, libc::SIG_DFL);
                libc::raise(signal);
            }

            return;
        }
    };

    // SAFETY: previous was captured from sigaction during handler installation
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
    // SAFETY: SA_SIGINFO delivers a valid siginfo pointer for this handler
    let address = unsafe { (*signal_info).si_addr() as usize };

    // scan watched ranges without signal-unsafe locks
    for page in WriteWatchTable::pages() {
        let Some(entries) = page.entries() else {
            continue;
        };

        for entry in entries {
            let Some(context) = entry.context(address) else {
                continue;
            };

            // SAFETY: context comes from the write-watch table registration
            if unsafe { watch_page_write(context, address) } {
                return;
            }
        }
    }

    // raise unrelated signals through the previous platform handler
    raise_unhandled_signal(signal);
}
