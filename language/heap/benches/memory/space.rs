use std::hint::black_box;
use std::mem::size_of;

use destack_memory::AddressSpace;

use crate::config::{PAGE_BYTES, SPACE_BYTES};

/// One reserved address-space shape used by fork benchmarks.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AddressSpaceShape {
    /// The reserved virtual byte width.
    space_bytes: usize,
    /// The active byte prefix to materialize.
    active_bytes: usize,
}

impl AddressSpaceShape {
    /// Return the default reserved address-space shape.
    pub(crate) const fn reserved() -> Self {
        Self {
            space_bytes: SPACE_BYTES,
            active_bytes: 0,
        }
    }

    /// Return the default space shape with a materialized page prefix.
    pub(crate) const fn materialized_pages(page_count: usize) -> Self {
        Self {
            space_bytes: SPACE_BYTES,
            active_bytes: page_count * PAGE_BYTES,
        }
    }

    /// Return an address-space shape with a materialized byte prefix.
    pub(crate) const fn materialized_bytes(space_bytes: usize, active_bytes: usize) -> Self {
        Self {
            space_bytes,
            active_bytes,
        }
    }

    /// Reserve this address space without materializing pages.
    pub(crate) fn reserve(self) -> AddressSpace {
        AddressSpace::reserve(self.space_bytes, PAGE_BYTES).expect("address space should reserve")
    }

    /// Reserve this address space and materialize its active prefix.
    pub(crate) fn materialize(self) -> AddressSpace {
        let space = self.reserve();
        let page = vec![0xAB; PAGE_BYTES];
        let page_count = self.active_bytes.div_ceil(PAGE_BYTES);

        // materialize only the active pages requested by the benchmark
        for page_index in 0..page_count {
            let offset = page_index * PAGE_BYTES;
            space
                .write_bytes(offset, &page)
                .expect("address space page write should succeed");
        }

        space
    }

    /// Fork one materialized address space lazily.
    pub(crate) fn fork_lazy_pair(self) -> (AddressSpace, AddressSpace) {
        let parent = self.materialize();
        let child = parent
            .fork_lazy()
            .expect("address space fork should succeed");

        (parent, child)
    }

    /// Fork one materialized address space lazily and return its first word address.
    pub(crate) fn fork_lazy_word(self) -> (AddressSpace, AddressSpace, *mut usize) {
        let (parent, child) = self.fork_lazy_pair();
        let address = child
            .address(0, size_of::<usize>())
            .expect("forked address should resolve")
            .cast::<usize>();

        (parent, child, address)
    }

    /// Fork one materialized address space eagerly and return its first word address.
    pub(crate) fn fork_eager_word(self) -> (AddressSpace, AddressSpace, *mut usize) {
        let parent = self.materialize();
        let child = parent.fork_eager(..).expect("eager fork should succeed");
        let address = child
            .address(0, size_of::<usize>())
            .expect("forked address should resolve")
            .cast::<usize>();

        (parent, child, address)
    }

    /// Fork one materialized address space lazily and return its first page-range address.
    pub(crate) fn fork_lazy_pages(
        self,
        page_count: usize,
    ) -> (AddressSpace, AddressSpace, *mut usize) {
        let (parent, child) = self.fork_lazy_pair();
        let byte_len = page_count * PAGE_BYTES;
        let address = child
            .address(0, byte_len)
            .expect("forked address range should resolve")
            .cast::<usize>();

        (parent, child, address)
    }
}

/// One live address-space lineage used by fork benchmarks.
pub(crate) struct ForkLineage {
    /// The live spaces from root to leaf.
    spaces: Vec<AddressSpace>,
}

impl ForkLineage {
    /// Build one live fork chain from materialized page counts.
    pub(crate) fn with_pages(
        page_count: usize,
        ancestor_count: usize,
        dirty_page_count: usize,
    ) -> Self {
        let shape = AddressSpaceShape::materialized_pages(page_count);

        Self::with_shape(shape, ancestor_count, dirty_page_count)
    }

    /// Build one live fork chain from active byte counts.
    pub(crate) fn with_bytes(
        space_bytes: usize,
        active_bytes: usize,
        ancestor_count: usize,
        dirty_bytes: usize,
    ) -> Self {
        let shape = AddressSpaceShape::materialized_bytes(space_bytes, active_bytes);
        let dirty_page_count = dirty_bytes.div_ceil(PAGE_BYTES);

        Self::with_shape(shape, ancestor_count, dirty_page_count)
    }

    /// Fork the live leaf space.
    pub(crate) fn fork_leaf(&self) -> AddressSpace {
        self.spaces
            .last()
            .expect("fork lineage should keep one leaf")
            .fork_lazy()
            .expect("nested address space fork should succeed")
    }

    /// Build one live fork chain from a materialized address-space shape.
    fn with_shape(
        shape: AddressSpaceShape,
        ancestor_count: usize,
        dirty_page_count: usize,
    ) -> Self {
        let mut spaces = Vec::with_capacity(ancestor_count + 1);
        let root = shape.materialize();
        spaces.push(root);

        // keep every ancestor live so nested forks retain shared frames
        for ancestor_index in 0..ancestor_count {
            let child = spaces[ancestor_index]
                .fork_lazy()
                .expect("address space fork should succeed");
            spaces.push(child);
        }

        // dirty the leaf after the chain is built
        let page = vec![0xA5; PAGE_BYTES];
        let leaf = spaces.last().expect("fork lineage should keep one leaf");
        for page_index in 0..dirty_page_count {
            let offset = page_index * PAGE_BYTES;
            leaf.write_bytes(offset, black_box(&page))
                .expect("forked address space write should succeed");
        }

        Self { spaces }
    }
}
