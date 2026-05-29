use std::sync::Arc;

use destack_memory::AddressSpace;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::{SharedRawBlock, SharedRawSpace};
use crate::allocator::PageSpanCache;
use crate::{AllocationUsage, Allocator, HeapResult, PageSpan};

/// One frozen shared raw-space image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedRawSpaceImage {
    /// Captured shared raw-space blocks keyed by block index.
    blocks: Box<[SharedRawBlockImage]>,

    /// The number of allocated shared raw-space blocks.
    allocated_count: usize,
    /// The number of allocated shared raw-space bytes.
    allocated_bytes: u64,
    /// The reserved virtual byte capacity for shared raw space.
    space_size_bytes: usize,
    /// The next unused byte offset in shared raw space.
    next_offset: usize,
}

impl SharedRawSpaceImage {
    /// Create one frozen shared raw-space image.
    pub fn new(
        blocks: Box<[SharedRawBlockImage]>,
        allocated_count: usize,
        allocated_bytes: u64,
        space_size_bytes: usize,
        next_offset: usize,
    ) -> Self {
        Self {
            blocks,
            allocated_count,
            allocated_bytes,
            space_size_bytes,
            next_offset,
        }
    }

    /// Return one shared block image by index.
    pub fn block(&self, index: usize) -> Option<&SharedRawBlockImage> {
        self.blocks.get(index)
    }

    /// Return the frozen shared raw-space blocks.
    pub fn blocks(&self) -> &[SharedRawBlockImage] {
        &self.blocks
    }

    /// Return the number of allocated blocks.
    pub const fn allocated_count(&self) -> usize {
        self.allocated_count
    }

    /// Return the allocated shared bytes.
    pub const fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    /// Return the reserved virtual byte capacity for shared raw space.
    pub const fn space_size_bytes(&self) -> usize {
        self.space_size_bytes
    }

    /// Return the next unused byte offset in shared raw space.
    pub const fn next_offset(&self) -> usize {
        self.next_offset
    }

    /// Return the retained frozen page count.
    pub fn page_count(&self, page_size_bytes: usize) -> usize {
        let mut page_count = 0;

        // count retained block bytes
        for block in self.blocks() {
            page_count += block.bytes.len().div_ceil(page_size_bytes);
        }

        page_count
    }
}

/// One frozen shared raw-space block image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedRawBlockImage {
    /// Whether this block is live.
    pub is_live: bool,
    /// The first byte offset inside shared raw space.
    pub first_offset: usize,
    /// The logical byte length of this block.
    pub byte_len: usize,
    /// The captured block bytes.
    pub bytes: Box<[u8]>,
}

impl SharedRawSpace {
    /// Fork one shared raw space over the same shared allocator.
    ///
    /// Call this only from a safepoint where shared raw mutators are stopped.
    pub fn fork(&self) -> HeapResult<Self> {
        let blocks = self.blocks.read();
        let state = self.state.lock();
        let mapping = self.mapping.write().fork_lazy()?;
        let base_address = mapping.base_address();

        let forked = Self {
            allocator: self.allocator.clone(),
            base_address,
            state: parking_lot::Mutex::new(super::space::SharedRawState {
                page_span_cache: PageSpanCache::new(self.allocator.pages_per_chunk()),
                usage: state.usage,
                live_retained_bytes: state.live_retained_bytes,
                next_offset: state.next_offset,
            }),
            page_map: parking_lot::RwLock::new(Vec::new()),
            blocks: parking_lot::RwLock::new(
                blocks
                    .iter()
                    .map(|block: &Arc<RwLock<SharedRawBlock>>| -> HeapResult<_> {
                        let block = block.read();
                        let pages = if block.is_vacant() {
                            PageSpan::empty()
                        } else {
                            self.allocator.share_page_span(block.pages)?
                        };
                        let mut block = block.clone();
                        block.pages = pages;

                        Ok(Arc::new(RwLock::new(block)))
                    })
                    .collect::<HeapResult<Vec<_>>>()?,
            ),
            mapping: parking_lot::RwLock::new(mapping),
        };

        drop(state);
        drop(blocks);

        rebuild_page_map(&forked);

        Ok(forked)
    }

    /// Create one shared raw space from one frozen image over one shared allocator.
    pub fn from_image_with_allocator(
        allocator: Arc<Allocator>,
        image: &SharedRawSpaceImage,
    ) -> HeapResult<Self> {
        let mapping = AddressSpace::reserve(image.space_size_bytes(), allocator.page_size_bytes())?;
        let base_address = mapping.base_address();

        let blocks = image
            .blocks()
            .iter()
            .map(|block| -> HeapResult<_> {
                let block = if block.is_live {
                    let pages = allocator.allocate_pages(block.bytes.len())?;

                    mapping.write_bytes(block.first_offset, &block.bytes)?;

                    Arc::new(RwLock::new(SharedRawBlock::new(
                        block.first_offset,
                        block.byte_len,
                        pages,
                    )))
                } else {
                    Arc::new(RwLock::new(SharedRawBlock::vacant()))
                };

                Ok(block)
            })
            .collect::<HeapResult<Vec<_>>>()?;
        let live_retained_bytes = live_retained_bytes(&blocks, allocator.page_size_bytes());
        let restored = Self {
            allocator: allocator.clone(),
            base_address,
            state: parking_lot::Mutex::new(super::space::SharedRawState {
                page_span_cache: PageSpanCache::new(allocator.pages_per_chunk()),
                usage: AllocationUsage::new(image.allocated_count(), image.allocated_bytes()),
                live_retained_bytes,
                next_offset: image.next_offset(),
            }),
            page_map: parking_lot::RwLock::new(Vec::new()),
            blocks: parking_lot::RwLock::new(blocks),
            mapping: parking_lot::RwLock::new(mapping),
        };

        rebuild_page_map(&restored);

        Ok(restored)
    }

    /// Return one frozen shared raw-space image.
    pub fn image(&self) -> HeapResult<SharedRawSpaceImage> {
        let blocks = self.blocks.read();
        let state = self.state.lock();

        Ok(SharedRawSpaceImage::new(
            blocks
                .iter()
                .map(|block: &Arc<RwLock<SharedRawBlock>>| -> HeapResult<_> {
                    let block = block.read();
                    let bytes = if block.is_vacant() {
                        Box::new([])
                    } else {
                        let byte_len = block.pages.len() * self.allocator.page_size_bytes();
                        self.mapping
                            .read()
                            .read_bytes(block.first_offset, byte_len)?
                            .into_boxed_slice()
                    };

                    Ok(SharedRawBlockImage {
                        is_live: !block.is_vacant(),
                        first_offset: block.first_offset,
                        byte_len: block.byte_len,
                        bytes,
                    })
                })
                .collect::<HeapResult<Vec<_>>>()?
                .into_boxed_slice(),
            state.usage.allocation_count(),
            state.usage.allocated_bytes(),
            self.mapping.read().byte_len(),
            state.next_offset,
        ))
    }
}

/// Rebuild the address space entries for one live shared raw space.
fn rebuild_page_map(raw: &SharedRawSpace) {
    let blocks = raw.blocks.read();

    for (block_index, block) in blocks.iter().enumerate() {
        let block = block.read();

        if block.is_vacant() {
            continue;
        }

        raw.map_page_span(block.first_offset, &block.pages, block_index);
    }
}

/// Return the live retained bytes for restored raw blocks.
fn live_retained_bytes(blocks: &[Arc<RwLock<SharedRawBlock>>], page_size_bytes: usize) -> u64 {
    let mut retained_bytes = 0;

    for block in blocks {
        let block = block.read();

        if block.is_vacant() {
            continue;
        }

        retained_bytes += block.pages.len() as u64 * page_size_bytes as u64;
    }

    retained_bytes
}
