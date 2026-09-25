use std::hint::black_box;
use std::mem::size_of;

use tspp_memory::MemoryMap;

use crate::config::{PAGE_SIZE_BYTES, SPACE_SIZE_BYTES};

/// One reserved memory map shape used by fork benchmarks.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MemoryMapShape {
    /// The reserved virtual byte width.
    reserved_size_bytes: usize,
    /// The active byte prefix to materialize.
    active_bytes: usize,
}

impl MemoryMapShape {
    /// Return the default reserved memory map shape.
    pub(crate) const fn reserved() -> Self {
        Self {
            reserved_size_bytes: SPACE_SIZE_BYTES,
            active_bytes: 0,
        }
    }

    /// Return the default memory map shape with a materialized page prefix.
    pub(crate) const fn materialized_pages(page_count: usize) -> Self {
        Self {
            reserved_size_bytes: SPACE_SIZE_BYTES,
            active_bytes: page_count * PAGE_SIZE_BYTES,
        }
    }

    /// Return a memory map shape with a materialized byte prefix.
    pub(crate) const fn materialized_bytes(
        reserved_size_bytes: usize,
        active_bytes: usize,
    ) -> Self {
        Self {
            reserved_size_bytes,
            active_bytes,
        }
    }

    /// Reserve this memory map without materializing pages.
    pub(crate) fn reserve(self) -> MemoryMap {
        MemoryMap::reserve(self.reserved_size_bytes, PAGE_SIZE_BYTES)
            .expect("memory map should reserve")
    }

    /// Reserve this memory map and materialize its active prefix.
    pub(crate) fn materialize(self) -> MemoryMap {
        let map = self.reserve();
        let page = vec![0xAB; PAGE_SIZE_BYTES];
        let page_count = self.active_bytes.div_ceil(PAGE_SIZE_BYTES);

        // materialize only the active pages requested by the benchmark
        for page_index in 0..page_count {
            let offset = page_index * PAGE_SIZE_BYTES;
            map.write_bytes(offset, &page)
                .expect("memory map page write should succeed");
        }

        map
    }

    /// Fork one materialized memory map lazily.
    pub(crate) fn fork_lazy_pair(self) -> (MemoryMap, MemoryMap) {
        let parent = self.materialize();
        let child = parent.fork_lazy().expect("memory map fork should succeed");

        (parent, child)
    }

    /// Fork one materialized memory map lazily and return its first word address.
    pub(crate) fn fork_lazy_word(self) -> (MemoryMap, MemoryMap, *mut usize) {
        let (parent, child) = self.fork_lazy_pair();
        let address = child
            .address(0, size_of::<usize>())
            .expect("forked address should resolve")
            .cast::<usize>();

        (parent, child, address)
    }

    /// Fork one materialized memory map eagerly and return its first word address.
    pub(crate) fn fork_eager_word(self) -> (MemoryMap, MemoryMap, *mut usize) {
        let parent = self.materialize();
        let child = parent
            .fork_eager(0..parent.byte_len())
            .expect("eager fork should succeed");
        let address = child
            .address(0, size_of::<usize>())
            .expect("forked address should resolve")
            .cast::<usize>();

        (parent, child, address)
    }

    /// Fork one materialized memory map lazily and return its first page-range address.
    pub(crate) fn fork_lazy_pages(self, page_count: usize) -> (MemoryMap, MemoryMap, *mut usize) {
        let (parent, child) = self.fork_lazy_pair();
        let byte_len = page_count * PAGE_SIZE_BYTES;
        let address = child
            .address(0, byte_len)
            .expect("forked address range should resolve")
            .cast::<usize>();

        (parent, child, address)
    }
}

/// One live memory map lineage used by fork benchmarks.
pub(crate) struct ForkLineage {
    /// The live maps from root to leaf.
    maps: Vec<MemoryMap>,
}

impl ForkLineage {
    /// Build one live fork chain from materialized page counts.
    pub(crate) fn with_pages(
        page_count: usize,
        ancestor_count: usize,
        dirty_page_count: usize,
    ) -> Self {
        let shape = MemoryMapShape::materialized_pages(page_count);

        Self::with_shape(shape, ancestor_count, dirty_page_count)
    }

    /// Build one live fork chain from active byte counts.
    pub(crate) fn with_bytes(
        reserved_size_bytes: usize,
        active_bytes: usize,
        ancestor_count: usize,
        dirty_bytes: usize,
    ) -> Self {
        let shape = MemoryMapShape::materialized_bytes(reserved_size_bytes, active_bytes);
        let dirty_page_count = dirty_bytes.div_ceil(PAGE_SIZE_BYTES);

        Self::with_shape(shape, ancestor_count, dirty_page_count)
    }

    /// Fork the live leaf map.
    pub(crate) fn fork_leaf(&self) -> MemoryMap {
        self.maps
            .last()
            .expect("fork lineage should keep one leaf")
            .fork_lazy()
            .expect("nested memory map fork should succeed")
    }

    /// Build one live fork chain from a materialized memory map shape.
    fn with_shape(shape: MemoryMapShape, ancestor_count: usize, dirty_page_count: usize) -> Self {
        let mut maps = Vec::with_capacity(ancestor_count + 1);
        let root = shape.materialize();
        maps.push(root);

        // keep every ancestor live so nested forks retain shared frames
        for ancestor_index in 0..ancestor_count {
            let child = maps[ancestor_index]
                .fork_lazy()
                .expect("memory map fork should succeed");
            maps.push(child);
        }

        // dirty the leaf after the chain is built
        let page = vec![0xA5; PAGE_SIZE_BYTES];
        let leaf = maps.last().expect("fork lineage should keep one leaf");
        for page_index in 0..dirty_page_count {
            let offset = page_index * PAGE_SIZE_BYTES;
            leaf.write_bytes(offset, black_box(&page))
                .expect("forked memory map write should succeed");
        }

        Self { maps }
    }
}
