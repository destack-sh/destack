use std::mem::MaybeUninit;
use std::ptr::{copy_nonoverlapping, null_mut, write_bytes};
use std::sync::OnceLock;

use parking_lot::Mutex;
use windows_sys::Win32::Foundation::{
    CloseHandle, EXCEPTION_ACCESS_VIOLATION, GetLastError, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::System::Diagnostics::Debug::{
    AddVectoredExceptionHandler, EXCEPTION_POINTERS,
};
use windows_sys::Win32::System::Memory::{
    CreateFileMappingW, MEM_PRESERVE_PLACEHOLDER, MEM_RELEASE, MEM_REPLACE_PLACEHOLDER,
    MEM_RESERVE, MEM_RESERVE_PLACEHOLDER, MEMORY_MAPPED_VIEW_ADDRESS, MapViewOfFile3,
    PAGE_NOACCESS, PAGE_READONLY, PAGE_READWRITE, PAGE_WRITECOPY, UnmapViewOfFile2, VirtualAlloc2,
    VirtualFree, VirtualProtect,
};
use windows_sys::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

use crate::address::watch_page_write;
pub(crate) use crate::core::{WriteWatchRegistration, WriteWatchTable};
use crate::{MemoryError, MemoryOperation, MemoryResult};

/// Whether mapped spaces can share page frames directly.
pub(crate) const SUPPORTS_SHARED_PAGE_FRAMES: bool = true;
/// Continue after handling one vectored exception.
const EXCEPTION_CONTINUE_EXECUTION: i32 = -1;
/// Search the next vectored exception handler.
const EXCEPTION_CONTINUE_SEARCH: i32 = 0;
/// The minimum frame count for one Windows page frame section.
const MIN_PAGE_FRAME_SECTION_FRAMES: usize = 256;
/// The Windows access violation code for writes.
const ACCESS_VIOLATION_WRITE: usize = 1;
/// The one time Windows write fault handler installation.
static WRITE_FAULT_HANDLER: OnceLock<MemoryResult<()>> = OnceLock::new();

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
    pub(crate) fn unmap<I>(&mut self, page_size_bytes: usize, mapped_pages: I)
    where
        I: IntoIterator<Item = usize>,
    {
        if self.base.is_null() || self.byte_len == 0 {
            return;
        }

        let mut next_page = 0;
        let page_count = self.byte_len / page_size_bytes;

        // release reserved gaps and mapped page views
        for page_index in mapped_pages {
            release_virtual_gap(self.base, next_page, page_index, page_size_bytes);

            // SAFETY: page_index comes from this address space page table
            let address = unsafe { self.base.add(page_index * page_size_bytes) };
            unmap_page_view(address);
            release_placeholder(address);

            next_page = page_index + 1;
        }

        release_virtual_gap(self.base, next_page, page_count, page_size_bytes);
        self.base = null_mut();
        self.byte_len = 0;
    }
}

/// One page sized backing frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PageFrame {
    /// The section that owns this page frame.
    pub(crate) section_index: usize,
    /// The byte offset inside the owning section.
    pub(crate) offset: u64,
}

/// One platform page frame allocator.
#[derive(Debug)]
pub(crate) struct PageFrameAllocator {
    /// The reusable frame state.
    state: Mutex<PageFrameAllocatorState>,
}

impl Drop for PageFrameAllocator {
    fn drop(&mut self) {
        let state = self.state.get_mut();

        // cleanup cannot report errors from Drop
        for section in state.sections.drain(..) {
            // SAFETY: section handles are owned by this allocator
            let _ = unsafe { CloseHandle(section.handle) };
        }
    }
}

impl PageFrameAllocator {
    /// Return the Windows section that owns one page frame.
    fn section(&self, frame: PageFrame) -> MemoryResult<HANDLE> {
        let state = self.state.lock();
        let Some(section) = state.sections.get(frame.section_index) else {
            return Err(MemoryError::Internal {
                context: "page frame section",
            });
        };

        Ok(section.handle)
    }
}

/// The page frame sections, free ranges, and live reference counts.
#[derive(Debug)]
struct PageFrameAllocatorState {
    /// The page frame sections.
    sections: Vec<PageFrameSection>,
    /// The free page frame byte ranges.
    free_ranges: Vec<PageFrameRange>,
}

/// One page frame section and its live reference counts.
#[derive(Debug)]
struct PageFrameSection {
    /// The Windows section handle.
    handle: HANDLE,
    /// The byte length of the section.
    byte_len: u64,
    /// The next never allocated byte offset.
    next_offset: u64,
    /// The live reference counts keyed by page frame index.
    frame_ref_counts: Vec<u32>,
}

impl PageFrameSection {
    /// Reserve one contiguous byte range inside this section.
    fn reserve(&mut self, byte_len: usize) -> Option<u64> {
        let byte_len = byte_len as u64;
        if byte_len > self.byte_len - self.next_offset {
            return None;
        }

        let offset = self.next_offset;
        self.next_offset += byte_len;

        Some(offset)
    }
}

/// One reusable range inside a page frame section.
#[derive(Debug, Clone, Copy)]
struct PageFrameRange {
    /// The first page frame.
    frame: PageFrame,
    /// The byte length.
    byte_len: u64,
}

/// Create one page frame allocator.
pub(crate) fn create_page_frame_allocator(_byte_len: usize) -> MemoryResult<PageFrameAllocator> {
    Ok(PageFrameAllocator {
        state: Mutex::new(PageFrameAllocatorState {
            sections: Vec::new(),
            free_ranges: Vec::new(),
        }),
    })
}

/// Return one page frame inside a contiguous frame range.
pub(crate) fn frame_at(frame: PageFrame, page_offset: usize, page_size_bytes: usize) -> PageFrame {
    PageFrame {
        section_index: frame.section_index,
        offset: frame.offset + (page_offset * page_size_bytes) as u64,
    }
}

/// Allocate one zeroed page frame range.
pub(crate) fn allocate_frame_range(
    allocator: &PageFrameAllocator,
    byte_len: usize,
    page_size_bytes: usize,
) -> MemoryResult<PageFrame> {
    let (frame, is_reused) = allocate_frame_storage(allocator, byte_len, page_size_bytes)?;

    // reused page file ranges must regain reserved page zero semantics
    if is_reused {
        zero_frame_range(allocator, frame, byte_len)?;
    }

    Ok(frame)
}

/// Retain mapped page frames.
pub(crate) fn retain_frames<I>(allocator: &PageFrameAllocator, frames: I, page_size_bytes: usize)
where
    I: IntoIterator<Item = PageFrame>,
{
    let mut state = allocator.state.lock();

    for frame in frames {
        let section = &mut state.sections[frame.section_index];
        let index = frame_index(frame, page_size_bytes);
        let ref_count = &mut section.frame_ref_counts[index];

        *ref_count += 1;
    }
}

/// Release mapped page frames.
pub(crate) fn release_frames<I>(allocator: &PageFrameAllocator, frames: I, page_size_bytes: usize)
where
    I: IntoIterator<Item = PageFrame>,
{
    let mut state = allocator.state.lock();

    for frame in frames {
        let section = &mut state.sections[frame.section_index];
        let index = frame_index(frame, page_size_bytes);
        let ref_count = &mut section.frame_ref_counts[index];

        // keep shared frames live until the last mapping drops
        *ref_count -= 1;
        if *ref_count != 0 {
            continue;
        }

        state.free_ranges.push(PageFrameRange {
            frame,
            byte_len: page_size_bytes as u64,
        });
    }

    // merge once after the whole release batch
    merge_free_frame_ranges(&mut state.free_ranges);
}

/// Copy one mapped page into a fresh page frame.
pub(crate) fn copy_page(
    allocator: &PageFrameAllocator,
    source: *mut u8,
    page_size_bytes: usize,
) -> MemoryResult<PageFrame> {
    copy_frame_range(allocator, source, page_size_bytes, page_size_bytes)
}

/// Copy one mapped byte range into a fresh page frame range.
pub(crate) fn copy_frame_range(
    allocator: &PageFrameAllocator,
    source: *mut u8,
    byte_len: usize,
    page_size_bytes: usize,
) -> MemoryResult<PageFrame> {
    let (frame, _) = allocate_frame_storage(allocator, byte_len, page_size_bytes)?;
    let target = map_frame_range_anywhere(allocator, frame, byte_len)?;

    // SAFETY: source and target are mapped for byte_len bytes
    unsafe {
        copy_nonoverlapping(source, target, byte_len);
    }

    unmap_scratch_view(target);

    Ok(frame)
}

/// Return the platform frame byte width for fixed address mappings.
pub(crate) fn system_frame_size_bytes() -> MemoryResult<usize> {
    let mut system = MaybeUninit::<SYSTEM_INFO>::uninit();

    // SAFETY: GetSystemInfo initializes the provided SYSTEM_INFO storage
    unsafe {
        GetSystemInfo(system.as_mut_ptr());
    }

    // SAFETY: GetSystemInfo initialized the structure above
    let system = unsafe { system.assume_init() };
    let frame_size_bytes = system.dwPageSize as usize;
    if frame_size_bytes == 0 {
        return Err(MemoryError::Internal {
            context: "system frame size",
        });
    }

    Ok(frame_size_bytes)
}

/// Reserve one inaccessible virtual address range.
pub(crate) fn reserve_virtual_space(byte_len: usize) -> MemoryResult<VirtualSpace> {
    if byte_len == 0 {
        return Ok(VirtualSpace {
            base: null_mut(),
            byte_len,
        });
    }

    let address = reserve_placeholder(byte_len)?;

    Ok(VirtualSpace {
        base: address,
        byte_len,
    })
}

/// Map one page frame as writable memory.
pub(crate) fn map_page_writable(
    base: *mut u8,
    page_index: usize,
    page_size_bytes: usize,
    allocator: &PageFrameAllocator,
    frame: PageFrame,
) -> MemoryResult<()> {
    map_page(
        base,
        page_index,
        page_size_bytes,
        allocator,
        frame,
        PAGE_READWRITE,
    )
}

/// Map one page frame range as copy on write memory.
pub(crate) fn map_frame_range_cow(
    base: *mut u8,
    first_page: usize,
    page_size_bytes: usize,
    byte_len: usize,
    allocator: &PageFrameAllocator,
    frame: PageFrame,
) -> MemoryResult<()> {
    map_frame_range(
        base,
        first_page,
        page_size_bytes,
        byte_len,
        allocator,
        frame,
        PAGE_READONLY,
    )
}

/// Map one page frame range as writable memory.
pub(crate) fn map_frame_range_writable(
    base: *mut u8,
    first_page: usize,
    page_size_bytes: usize,
    byte_len: usize,
    allocator: &PageFrameAllocator,
    frame: PageFrame,
) -> MemoryResult<()> {
    map_frame_range(
        base,
        first_page,
        page_size_bytes,
        byte_len,
        allocator,
        frame,
        PAGE_READWRITE,
    )
}

/// Make shared pages writable after a watched write.
pub(crate) fn make_shared_pages_writable(
    base: *mut u8,
    first_page: usize,
    page_size_bytes: usize,
    byte_len: usize,
) -> MemoryResult<()> {
    let page_count = byte_len / page_size_bytes;

    // each mapped view is currently page granular on Windows
    for page_offset in 0..page_count {
        let page_index = first_page + page_offset;
        // SAFETY: page_index is inside the reserved address space
        let address = unsafe { base.add(page_index * page_size_bytes) };
        let mut old_protection = 0;
        // SAFETY: address and length describe one mapped page view
        let result = unsafe {
            VirtualProtect(
                address.cast(),
                page_size_bytes,
                PAGE_WRITECOPY,
                &mut old_protection,
            )
        };
        if result == 0 {
            return Err(last_system_error(
                MemoryOperation::ProtectPages,
                Some(byte_len),
            ));
        }
    }

    Ok(())
}

/// Register one write watched virtual range.
pub(crate) fn register_write_watch(
    base: *mut u8,
    byte_len: usize,
    context: *const (),
) -> MemoryResult<WriteWatchRegistration> {
    let registration = WriteWatchTable::register(base, byte_len, context)?;

    if let Err(error) = install_write_fault_handler() {
        WriteWatchTable::unregister(&registration);

        return Err(error);
    }

    Ok(registration)
}

/// Unregister one write watched virtual range.
pub(crate) fn unregister_write_watch(registration: &WriteWatchRegistration) {
    WriteWatchTable::unregister(registration);
}

/// Allocate backing storage for one page frame range.
fn allocate_frame_storage(
    allocator: &PageFrameAllocator,
    byte_len: usize,
    page_size_bytes: usize,
) -> MemoryResult<(PageFrame, bool)> {
    let mut state = allocator.state.lock();

    // reuse a returned frame range before extending section storage
    let (frame, is_reused) = if let Some(frame) = allocate_free_frame_range(&mut state, byte_len) {
        (frame, true)
    } else {
        let frame = allocate_new_frame_range(&mut state, byte_len, page_size_bytes)?;

        (frame, false)
    };

    // each page starts with one owning mapping
    initialize_frame_references(&mut state, frame, byte_len, page_size_bytes);

    Ok((frame, is_reused))
}

/// Map one page frame range into reserved virtual pages.
fn map_frame_range(
    base: *mut u8,
    first_page: usize,
    page_size_bytes: usize,
    byte_len: usize,
    allocator: &PageFrameAllocator,
    frame: PageFrame,
    protection: u32,
) -> MemoryResult<()> {
    let page_count = byte_len / page_size_bytes;

    // windows maps section views one page at a time
    for page_offset in 0..page_count {
        let page_index = first_page + page_offset;
        let frame = PageFrame {
            section_index: frame.section_index,
            offset: frame.offset + (page_offset * page_size_bytes) as u64,
        };

        map_page(
            base,
            page_index,
            page_size_bytes,
            allocator,
            frame,
            protection,
        )?;
    }

    Ok(())
}

/// Install the write fault handler once.
fn install_write_fault_handler() -> MemoryResult<()> {
    WRITE_FAULT_HANDLER
        .get_or_init(|| {
            // register before regular handlers so watched writes are handled first
            // SAFETY: the handler has the system calling convention and remains loaded
            let handler = unsafe { AddVectoredExceptionHandler(1, Some(handle_write_watch)) };

            if handler.is_null() {
                return Err(last_system_error(MemoryOperation::InstallWriteWatch, None));
            }

            Ok(())
        })
        .clone()
}

/// Handle one watched page write.
unsafe extern "system" fn handle_write_watch(exception: *mut EXCEPTION_POINTERS) -> i32 {
    // SAFETY: vectored exception handlers receive a valid exception pointer
    let record = unsafe { (*exception).ExceptionRecord };

    // only access violations can come from protected shared pages
    // SAFETY: record is owned by the exception currently being handled
    if unsafe { (*record).ExceptionCode } != EXCEPTION_ACCESS_VIOLATION {
        return EXCEPTION_CONTINUE_SEARCH;
    }

    // SAFETY: access violation records carry access kind at index 0
    let access = unsafe { (*record).ExceptionInformation[0] };
    if access != ACCESS_VIOLATION_WRITE {
        return EXCEPTION_CONTINUE_SEARCH;
    }

    // SAFETY: access violation records carry fault address at index 1
    let address = unsafe { (*record).ExceptionInformation[1] };

    // handle writes inside registered memory spaces
    if let Some(context) = WriteWatchTable::context(address) {
        // SAFETY: context comes from the write watch table registration
        if unsafe { watch_page_write(context, address) } {
            return EXCEPTION_CONTINUE_EXECUTION;
        }
    }

    EXCEPTION_CONTINUE_SEARCH
}

/// Map one page frame into a reserved virtual page.
fn map_page(
    base: *mut u8,
    page_index: usize,
    page_size_bytes: usize,
    allocator: &PageFrameAllocator,
    frame: PageFrame,
    protection: u32,
) -> MemoryResult<()> {
    // replace one placeholder page with one section view
    // SAFETY: page_index is inside the reserved address space
    let address = unsafe { base.add(page_index * page_size_bytes) };
    let section = allocator.section(frame)?;

    split_placeholder(address, page_size_bytes);
    unmap_page_view(address);

    // SAFETY: the placeholder was split to the exact page before replacement
    let view = unsafe {
        MapViewOfFile3(
            section,
            GetCurrentProcess(),
            address.cast(),
            frame.offset,
            page_size_bytes,
            MEM_REPLACE_PLACEHOLDER,
            protection,
            null_mut(),
            0,
        )
    };
    if view.Value.is_null() {
        return Err(last_system_error(
            MemoryOperation::MapFrameRange,
            Some(page_size_bytes),
        ));
    }

    Ok(())
}

/// Reserve one Windows placeholder range.
fn reserve_placeholder(byte_len: usize) -> MemoryResult<*mut u8> {
    // SAFETY: null base lets the kernel choose the placeholder reservation
    let address = unsafe {
        VirtualAlloc2(
            GetCurrentProcess(),
            null_mut(),
            byte_len,
            MEM_RESERVE | MEM_RESERVE_PLACEHOLDER,
            PAGE_NOACCESS,
            null_mut(),
            0,
        )
    };
    if address.is_null() {
        return Err(last_system_error(
            MemoryOperation::ReserveAddressSpace,
            Some(byte_len),
        ));
    }

    Ok(address.cast())
}

/// Split out one exact placeholder range when it is part of a larger placeholder.
fn split_placeholder(address: *mut u8, byte_len: usize) {
    // the call fails when the range is already exact or not a placeholder
    // SAFETY: address and length are within the placeholder reservation
    let _ = unsafe {
        VirtualFree(
            address.cast(),
            byte_len,
            MEM_RELEASE | MEM_PRESERVE_PLACEHOLDER,
        )
    };
}

/// Release one exact placeholder range.
fn release_placeholder(address: *mut u8) {
    // cleanup cannot report errors from Drop
    // SAFETY: address is an exact placeholder range
    let _ = unsafe { VirtualFree(address.cast(), 0, MEM_RELEASE) };
}

/// Release one placeholder gap between mapped pages.
fn release_virtual_gap(base: *mut u8, first_page: usize, end_page: usize, page_size_bytes: usize) {
    if first_page == end_page {
        return;
    }

    // SAFETY: first_page and end_page describe a gap in the placeholder range
    let address = unsafe { base.add(first_page * page_size_bytes) };
    release_placeholder(address);
}

/// Unmap one page view back into an exact placeholder.
fn unmap_page_view(address: *mut u8) {
    // SAFETY: address is either a mapped page view or this cleanup is a no op failure
    let _ = unsafe {
        UnmapViewOfFile2(
            GetCurrentProcess(),
            MEMORY_MAPPED_VIEW_ADDRESS {
                Value: address.cast(),
            },
            MEM_PRESERVE_PLACEHOLDER,
        )
    };
}

/// Unmap one scratch view that was not mapped over a placeholder.
fn unmap_scratch_view(address: *mut u8) {
    // SAFETY: address is a scratch mapping returned by MapViewOfFile3
    let _ = unsafe {
        UnmapViewOfFile2(
            GetCurrentProcess(),
            MEMORY_MAPPED_VIEW_ADDRESS {
                Value: address.cast(),
            },
            0,
        )
    };
}

/// Map one frame range at any available address.
fn map_frame_range_anywhere(
    allocator: &PageFrameAllocator,
    frame: PageFrame,
    byte_len: usize,
) -> MemoryResult<*mut u8> {
    let section = allocator.section(frame)?;

    // SAFETY: null base asks the kernel for a scratch mapping
    let view = unsafe {
        MapViewOfFile3(
            section,
            GetCurrentProcess(),
            null_mut(),
            frame.offset,
            byte_len,
            0,
            PAGE_READWRITE,
            null_mut(),
            0,
        )
    };
    if view.Value.is_null() {
        return Err(last_system_error(
            MemoryOperation::MapFrameRange,
            Some(byte_len),
        ));
    }

    Ok(view.Value.cast())
}

/// Zero one reusable frame range.
fn zero_frame_range(
    allocator: &PageFrameAllocator,
    frame: PageFrame,
    byte_len: usize,
) -> MemoryResult<()> {
    let target = map_frame_range_anywhere(allocator, frame, byte_len)?;

    // SAFETY: target is a writable scratch mapping for byte_len bytes
    unsafe {
        write_bytes(target, 0, byte_len);
    }

    unmap_scratch_view(target);

    Ok(())
}

/// Allocate one never used frame range from section storage.
fn allocate_new_frame_range(
    state: &mut PageFrameAllocatorState,
    byte_len: usize,
    page_size_bytes: usize,
) -> MemoryResult<PageFrame> {
    // carve from an existing section when it has tail capacity
    for (section_index, section) in state.sections.iter_mut().enumerate().rev() {
        let Some(offset) = section.reserve(byte_len) else {
            continue;
        };

        return Ok(PageFrame {
            section_index,
            offset,
        });
    }

    // create a larger section so following ranges stay contiguous
    let section_byte_len = page_frame_section_bytes(byte_len, page_size_bytes);
    let section = create_section(section_byte_len)?;
    let section_index = state.sections.len();
    let mut section = PageFrameSection {
        handle: section,
        byte_len: section_byte_len as u64,
        next_offset: 0,
        frame_ref_counts: Vec::new(),
    };
    let Some(offset) = section.reserve(byte_len) else {
        return Err(MemoryError::Internal {
            context: "page frame section reserve",
        });
    };

    state.sections.push(section);

    Ok(PageFrame {
        section_index,
        offset,
    })
}

/// Return the section byte length for one allocation request.
fn page_frame_section_bytes(byte_len: usize, page_size_bytes: usize) -> usize {
    let section_bytes = byte_len.max(page_size_bytes * MIN_PAGE_FRAME_SECTION_FRAMES);

    section_bytes.next_multiple_of(page_size_bytes)
}

/// Create one page file backed section.
fn create_section(byte_len: usize) -> MemoryResult<HANDLE> {
    let max_size = byte_len as u64;
    let max_size_high = (max_size >> 32) as u32;
    let max_size_low = max_size as u32;
    // SAFETY: INVALID_HANDLE_VALUE requests page file backed storage
    let section = unsafe {
        CreateFileMappingW(
            INVALID_HANDLE_VALUE,
            null_mut(),
            PAGE_READWRITE,
            max_size_high,
            max_size_low,
            null_mut(),
        )
    };
    if section == 0 {
        return Err(last_system_error(
            MemoryOperation::CreateFrameAllocator,
            Some(byte_len),
        ));
    }

    Ok(section)
}

/// Return one system error from the last platform error code.
fn last_system_error(operation: MemoryOperation, byte_len: Option<usize>) -> MemoryError {
    // SAFETY: GetLastError reads thread-local Windows error state
    let code = unsafe { GetLastError() } as i32;

    MemoryError::system(operation, Some(code), byte_len)
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
    let frame = range.frame;

    // consume the front of the free range
    range.frame.offset += byte_len;
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
    page_size_bytes: usize,
) {
    let page_count = byte_len / page_size_bytes;

    for page_offset in 0..page_count {
        let frame = frame_at(frame, page_offset, page_size_bytes);
        let index = frame_index(frame, page_size_bytes);
        let section = &mut state.sections[frame.section_index];

        if index >= section.frame_ref_counts.len() {
            section.frame_ref_counts.resize(index + 1, 0);
        }

        section.frame_ref_counts[index] = 1;
    }
}

/// Return the dense index for one page frame.
fn frame_index(frame: PageFrame, page_size_bytes: usize) -> usize {
    (frame.offset as usize) / page_size_bytes
}

/// Merge adjacent free page frame ranges.
fn merge_free_frame_ranges(ranges: &mut Vec<PageFrameRange>) {
    ranges.sort_by_key(|range| (range.frame.section_index, range.frame.offset));

    let mut range_index = 0;
    while range_index + 1 < ranges.len() {
        let current = ranges[range_index];
        let next = ranges[range_index + 1];
        let current_end = current.frame.offset + current.byte_len;

        // adjacent ranges inside the same section become one reusable range
        if current.frame.section_index == next.frame.section_index
            && current_end == next.frame.offset
        {
            ranges[range_index].byte_len += next.byte_len;
            ranges.remove(range_index + 1);
            continue;
        }

        range_index += 1;
    }
}
