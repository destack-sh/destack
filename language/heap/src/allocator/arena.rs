use std::ptr::{NonNull, null_mut};
use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};

use super::{PageId, PageRun};
use crate::{HeapError, HeapResult};

/// One fixed arena in the allocator.
#[derive(Debug)]
pub(super) struct Arena {
    /// The logical arena index.
    index: usize,
    /// The mapped arena base.
    base: NonNull<u8>,
    /// The next never-allocated page inside this arena.
    pub(super) next_unused_page: AtomicU32,
    /// The run refcounts keyed by run-start page index inside this arena.
    run_refcounts: Box<[AtomicU32]>,
}

impl Arena {
    /// Create one arena metadata record.
    pub(super) fn new(index: usize, base: NonNull<u8>, pages_per_arena: usize) -> Self {
        Self {
            index,
            base,
            next_unused_page: AtomicU32::new(0),
            run_refcounts: std::iter::repeat_with(|| AtomicU32::new(0))
                .take(pages_per_arena)
                .collect(),
        }
    }

    /// Return the logical arena index.
    pub(super) const fn index(&self) -> usize {
        self.index
    }

    /// Return the mapped arena base.
    pub(super) const fn base(&self) -> NonNull<u8> {
        self.base
    }

    /// Return one page pointer inside this arena.
    pub(super) fn page_ptr(&self, arena_page_index: usize, page_bytes: usize) -> Option<*mut u8> {
        let data = self.base.as_ptr();
        let page_offset = arena_page_index.checked_mul(page_bytes)?;

        Some(unsafe { data.add(page_offset) })
    }

    /// Allocate one fresh run from this arena.
    pub(super) fn allocate_run(
        &self,
        arena_index: usize,
        page_count: usize,
        pages_per_arena: usize,
    ) -> HeapResult<Option<PageRun>> {
        let page_count = u32::try_from(page_count)
            .map_err(|_| HeapError::InvalidPageId { index: page_count })?;

        let start_page = loop {
            let start_page = self.next_unused_page.load(Ordering::Acquire);
            let Some(end_page) = start_page.checked_add(page_count) else {
                return Err(HeapError::InvalidPageId {
                    index: start_page as usize,
                });
            };

            // stop once this arena is exhausted
            if end_page as usize > pages_per_arena {
                return Ok(None);
            }

            if self
                .next_unused_page
                .compare_exchange(start_page, end_page, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                break start_page as usize;
            }
        };

        let first_page = arena_index
            .checked_mul(pages_per_arena)
            .and_then(|first_page| first_page.checked_add(start_page))
            .ok_or(HeapError::InvalidPageId { index: arena_index })?;
        let first_page = PageId::new(first_page)?;

        Ok(Some(PageRun::new(first_page, page_count as usize)?))
    }

    /// Report whether this arena still has capacity for one fresh run.
    pub(super) fn has_capacity(
        &self,
        page_count: usize,
        pages_per_arena: usize,
    ) -> HeapResult<bool> {
        let start_page = self.next_unused_page.load(Ordering::Acquire) as usize;
        let Some(end_page) = start_page.checked_add(page_count) else {
            return Err(HeapError::InvalidPageId { index: start_page });
        };

        Ok(end_page <= pages_per_arena)
    }

    /// Return one run refcount by arena-local page index.
    pub(super) fn run_refcount(&self, arena_page_index: usize) -> Option<&AtomicU32> {
        self.run_refcounts.get(arena_page_index)
    }

    /// Raise the fresh-allocation watermark to the given page index.
    pub(super) fn raise_watermark(&self, next_unused_page: usize) {
        self.next_unused_page
            .fetch_max(next_unused_page as u32, Ordering::AcqRel);
    }
}

/// One dense logical arena table.
#[derive(Debug)]
pub(super) struct ArenaTable {
    /// The arena pointers stored by logical arena index.
    arenas: Box<[AtomicPtr<Arena>]>,
}

impl ArenaTable {
    /// Create one empty arena table for the given arena count.
    pub(super) fn new(arena_count: usize) -> Self {
        let mut arenas = Vec::with_capacity(arena_count);

        for _ in 0..arena_count {
            arenas.push(AtomicPtr::new(null_mut()));
        }

        Self {
            arenas: arenas.into_boxed_slice(),
        }
    }

    /// Return one arena by logical arena index.
    pub(super) fn get(&self, arena_index: usize) -> Option<&Arena> {
        let arena = self.arenas.get(arena_index)?.load(Ordering::Acquire);
        if arena.is_null() {
            return None;
        }

        Some(unsafe { &*arena })
    }

    /// Insert one arena pointer at one logical arena index.
    pub(super) fn insert(&self, arena_index: usize, arena: *mut Arena) {
        let Some(slot) = self.arenas.get(arena_index) else {
            panic!("arena table index should stay in bounds: arena_index={arena_index}");
        };
        let previous = slot.swap(arena, Ordering::AcqRel);

        if !previous.is_null() {
            panic!("arena table entry should be installed once: arena_index={arena_index}");
        }
    }
}

/// One sparse address-map block.
#[derive(Debug)]
struct ArenaAddressBlock {
    /// The arena pointers stored in this block.
    arenas: Box<[AtomicPtr<Arena>]>,
}

impl ArenaAddressBlock {
    /// The number of arena pointers stored in one block.
    const LEN: usize = 1 << 12;

    /// Create one empty address-map block.
    fn new() -> Self {
        let mut arenas = Vec::with_capacity(Self::LEN);

        for _ in 0..Self::LEN {
            arenas.push(AtomicPtr::new(null_mut()));
        }

        Self {
            arenas: arenas.into_boxed_slice(),
        }
    }
}

/// One sparse arena address map.
#[derive(Debug)]
pub(super) struct ArenaAddressMap {
    /// The maximum index stored in this map.
    entry_count: usize,
    /// The sparse block pointers.
    blocks: Box<[AtomicPtr<ArenaAddressBlock>]>,
}

impl ArenaAddressMap {
    /// Create one sparse arena address map for the given entry count.
    pub(super) fn new(entry_count: usize) -> Self {
        let block_count = entry_count.div_ceil(ArenaAddressBlock::LEN);
        let mut blocks = Vec::with_capacity(block_count);

        for _ in 0..block_count {
            blocks.push(AtomicPtr::new(null_mut()));
        }

        Self {
            entry_count,
            blocks: blocks.into_boxed_slice(),
        }
    }

    /// Return one arena by sparse address-map index.
    pub(super) fn get(&self, index: usize) -> Option<&Arena> {
        if index >= self.entry_count {
            return None;
        }

        let block_index = index / ArenaAddressBlock::LEN;
        let slot_index = index % ArenaAddressBlock::LEN;
        let block = self.blocks.get(block_index)?.load(Ordering::Acquire);
        if block.is_null() {
            return None;
        }

        let arena = unsafe { &*block }.arenas[slot_index].load(Ordering::Acquire);
        if arena.is_null() {
            return None;
        }

        Some(unsafe { &*arena })
    }

    /// Insert one arena pointer at one sparse address-map index.
    pub(super) fn insert(&self, index: usize, arena: *mut Arena) {
        let block_index = index / ArenaAddressBlock::LEN;
        let slot_index = index % ArenaAddressBlock::LEN;
        let block = self.ensure_block(block_index);
        let previous = block.arenas[slot_index].swap(arena, Ordering::AcqRel);

        if !previous.is_null() {
            panic!("arena address map entry should be installed once: index={index}");
        }
    }

    /// Ensure one sparse address-map block exists.
    fn ensure_block(&self, block_index: usize) -> &ArenaAddressBlock {
        let slot = &self.blocks[block_index];
        let current = slot.load(Ordering::Acquire);
        if !current.is_null() {
            return unsafe { &*current };
        }

        let block = Box::into_raw(Box::new(ArenaAddressBlock::new()));
        match slot.compare_exchange(null_mut(), block, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => unsafe { &*block },
            Err(current) => {
                unsafe {
                    drop(Box::from_raw(block));
                }

                unsafe { &*current }
            }
        }
    }
}

impl Drop for ArenaAddressMap {
    fn drop(&mut self) {
        for block in &self.blocks {
            let block = block.load(Ordering::Acquire);
            if block.is_null() {
                continue;
            }

            unsafe {
                drop(Box::from_raw(block));
            }
        }
    }
}

/// Return the maximum allocator arena count addressable by page ids.
pub(super) fn max_arena_count(page_bytes: usize, arena_bytes: usize) -> usize {
    ((u32::MAX as usize) + 1).div_ceil(arena_bytes / page_bytes)
}

/// Return the number of sparse arena-map entries addressable for this arena size.
pub(super) fn max_arena_frame_count(arena_bytes: usize) -> HeapResult<usize> {
    let address_bits = arena_address_bits();
    let arena_shift = arena_bytes.trailing_zeros() as usize;
    if arena_shift >= address_bits {
        return Err(HeapError::InvalidAllocatorArenaBytes { bytes: arena_bytes });
    }

    1usize
        .checked_shl((address_bits - arena_shift) as u32)
        .ok_or(HeapError::InvariantOverflow {
            context: "allocator arena-map entry count",
        })
}

/// Return one sparse arena-map index for one live arena address.
pub(super) fn arena_frame_index(address: usize, arena_bytes: usize) -> Option<usize> {
    if address == 0 {
        return None;
    }

    let address_bits = arena_address_bits();
    let address = if address_bits == usize::BITS as usize {
        address
    } else {
        address & ((1usize << address_bits) - 1)
    };

    Some(address / arena_bytes)
}

/// Return the sparse arena-map indexes covered by one mapped arena.
pub(super) fn arena_frame_indices(
    base_address: usize,
    byte_len: usize,
) -> HeapResult<std::ops::RangeInclusive<usize>> {
    let first_index = arena_frame_index(base_address, byte_len).ok_or(
        HeapError::AllocatorAddressUnsupported {
            address: base_address,
        },
    )?;
    let last_address =
        base_address
            .checked_add(byte_len - 1)
            .ok_or(HeapError::InvariantOverflow {
                context: "allocator arena end address",
            })?;
    let last_index = arena_frame_index(last_address, byte_len).ok_or(
        HeapError::AllocatorAddressUnsupported {
            address: last_address,
        },
    )?;

    Ok(first_index..=last_index)
}

/// Return the supported user virtual address width for arena-map indexing.
pub(super) const fn arena_address_bits() -> usize {
    if cfg!(target_pointer_width = "64") {
        48
    } else {
        usize::BITS as usize
    }
}

/// Allocate one mapped allocator arena.
pub(super) fn allocate_arena_bytes(byte_len: usize) -> HeapResult<NonNull<u8>> {
    let flags = libc::MAP_PRIVATE
        | if cfg!(any(target_os = "macos", target_os = "ios")) {
            libc::MAP_ANON
        } else {
            libc::MAP_ANONYMOUS
        };
    let data = unsafe {
        libc::mmap(
            null_mut(),
            byte_len,
            libc::PROT_READ | libc::PROT_WRITE,
            flags,
            -1,
            0,
        )
    };

    if data == libc::MAP_FAILED {
        return Err(HeapError::AllocatorArenaAllocationFailed { byte_len });
    }

    NonNull::new(data.cast()).ok_or(HeapError::AllocatorAddressUnsupported {
        address: data as usize,
    })
}

/// Free one mapped allocator arena.
pub(super) fn free_arena_bytes(data: *mut u8, byte_len: usize) {
    if data.is_null() {
        return;
    }

    unsafe {
        libc::munmap(data.cast(), byte_len);
    }
}

/// One allocator arena location resolved from one live address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ArenaLocation {
    /// The owning arena index.
    pub(crate) arena_index: usize,
    /// The byte offset inside the arena.
    pub(crate) arena_offset: usize,
}
