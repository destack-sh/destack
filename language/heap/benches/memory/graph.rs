use destack_heap::{
    AllocationShape, Heap, HeapReference, SharedAllocator, SharedGcWorker, SharedHeap,
    SharedHeapReference,
};
use destack_mir::ReferenceMap;

use crate::config::{LEAF_BYTES, RECORD_BYTES, REFERENCE_BYTES, WORKLOAD_OBJECTS};
use crate::heap::local_heap;

/// One allocated object graph.
pub(crate) struct ObjectGraph<R> {
    /// The allocated record references.
    pub(crate) records: Vec<R>,
}

/// One allocated fixed reference array.
pub(crate) struct ReferenceArray<R> {
    /// The allocated array reference.
    pub(crate) reference: R,
}

/// One object graph workload shaped like records pointing at leaf records.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ObjectGraphWorkload {
    /// The number of records to allocate.
    objects: usize,
    /// The record payload byte width.
    record_bytes: usize,
    /// The leaf payload byte width.
    leaf_bytes: usize,
}

impl ObjectGraphWorkload {
    /// Return the standard object graph workload.
    pub(crate) const fn standard() -> Self {
        Self {
            objects: WORKLOAD_OBJECTS,
            record_bytes: RECORD_BYTES,
            leaf_bytes: LEAF_BYTES,
        }
    }

    /// Allocate this workload in one local heap.
    pub(crate) fn allocate_local(self, heap: &mut Heap) -> ObjectGraph<HeapReference> {
        let reference_map = local_record_reference_map();

        self.allocate_local_with_map(heap, &reference_map)
    }

    /// Allocate this workload in one shared heap.
    pub(crate) fn allocate_shared(
        self,
        shared: &SharedHeap,
        worker: &SharedGcWorker,
        allocator: &mut SharedAllocator,
    ) -> ObjectGraph<SharedHeapReference> {
        let reference_map = shared_record_reference_map();

        self.allocate_shared_with_map(shared, worker, allocator, &reference_map)
    }

    /// Build one local heap with this workload ready to fork.
    pub(crate) fn local_heap(self) -> (Heap, ObjectGraph<HeapReference>) {
        let mut heap = local_heap();
        let graph = self.allocate_local(&mut heap);

        (heap, graph)
    }

    /// Allocate this workload in one local heap with a known reference map.
    fn allocate_local_with_map(
        self,
        heap: &mut Heap,
        reference_map: &ReferenceMap,
    ) -> ObjectGraph<HeapReference> {
        let leaf_map = ReferenceMap::None;
        let leaf_shape = AllocationShape::new(self.leaf_bytes, 1, &leaf_map);
        let record_shape = AllocationShape::new(self.record_bytes, REFERENCE_BYTES, reference_map);
        let leaf_layout = heap.allocation_layout(leaf_shape);
        let record_layout = heap.allocation_layout(record_shape);
        let mut records = Vec::with_capacity(self.objects);

        // allocate leaf and record pairs
        for index in 0..self.objects {
            let leaf = heap
                .allocate_zeroed(&leaf_layout)
                .expect("leaf allocation should succeed");
            let mut record = vec![0u8; self.record_bytes];
            write_word(&mut record, 0, leaf.bits());
            write_word(&mut record, REFERENCE_BYTES, index);

            let reference = heap
                .allocate_bytes(&record_layout, &record)
                .expect("record allocation should succeed");
            records.push(reference);
        }

        ObjectGraph { records }
    }

    /// Allocate this workload in one shared heap with a known reference map.
    fn allocate_shared_with_map(
        self,
        shared: &SharedHeap,
        worker: &SharedGcWorker,
        allocator: &mut SharedAllocator,
        reference_map: &ReferenceMap,
    ) -> ObjectGraph<SharedHeapReference> {
        let leaf_map = ReferenceMap::None;
        let leaf_shape = AllocationShape::new(self.leaf_bytes, 1, &leaf_map);
        let record_shape = AllocationShape::new(self.record_bytes, REFERENCE_BYTES, reference_map);
        let leaf_layout = shared.allocation_layout(leaf_shape);
        let record_layout = shared.allocation_layout(record_shape);
        let mut records = Vec::with_capacity(self.objects);

        // allocate leaf and record pairs through one worker cache
        for index in 0..self.objects {
            let leaf = shared
                .allocate_zeroed(worker, allocator, &leaf_layout)
                .expect("shared leaf allocation should succeed");
            let mut record = vec![0u8; self.record_bytes];
            write_word(&mut record, 0, leaf.bits());
            write_word(&mut record, REFERENCE_BYTES, index);

            let reference = shared
                .allocate_bytes(worker, allocator, &record_layout, &record)
                .expect("shared record allocation should succeed");
            records.push(reference);
        }

        ObjectGraph { records }
    }
}

/// One reference array workload backed by a repeated reference map.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ReferenceArrayWorkload {
    /// The number of references in the array.
    objects: usize,
    /// The leaf payload byte width.
    leaf_bytes: usize,
}

impl ReferenceArrayWorkload {
    /// Return the standard reference array workload.
    pub(crate) const fn standard() -> Self {
        Self {
            objects: WORKLOAD_OBJECTS,
            leaf_bytes: LEAF_BYTES,
        }
    }

    /// Allocate this workload in one local heap.
    pub(crate) fn allocate_local(self, heap: &mut Heap) -> ReferenceArray<HeapReference> {
        let reference_map = local_reference_array_map(self.objects);
        let leaf_map = ReferenceMap::None;
        let leaf_shape = AllocationShape::new(self.leaf_bytes, 1, &leaf_map);
        let array_shape = AllocationShape::new(
            self.objects * REFERENCE_BYTES,
            REFERENCE_BYTES,
            &reference_map,
        );
        let leaf_layout = heap.allocation_layout(leaf_shape);
        let array_layout = heap.allocation_layout(array_shape);
        let mut payload = vec![0u8; self.objects * REFERENCE_BYTES];

        // build the array payload from fresh leaf references
        for index in 0..self.objects {
            let leaf = heap
                .allocate_zeroed(&leaf_layout)
                .expect("leaf allocation should succeed");
            write_word(&mut payload, index * REFERENCE_BYTES, leaf.bits());
        }

        let reference = heap
            .allocate_bytes(&array_layout, &payload)
            .expect("reference array allocation should succeed");

        ReferenceArray { reference }
    }

    /// Allocate this workload in one shared heap.
    pub(crate) fn allocate_shared(
        self,
        shared: &SharedHeap,
        worker: &SharedGcWorker,
        allocator: &mut SharedAllocator,
    ) -> ReferenceArray<SharedHeapReference> {
        let reference_map = shared_reference_array_map(self.objects);
        let leaf_map = ReferenceMap::None;
        let leaf_shape = AllocationShape::new(self.leaf_bytes, 1, &leaf_map);
        let array_shape = AllocationShape::new(
            self.objects * REFERENCE_BYTES,
            REFERENCE_BYTES,
            &reference_map,
        );
        let leaf_layout = shared.allocation_layout(leaf_shape);
        let array_layout = shared.allocation_layout(array_shape);
        let mut payload = vec![0u8; self.objects * REFERENCE_BYTES];

        // build the array payload from fresh leaf references
        for index in 0..self.objects {
            let leaf = shared
                .allocate_zeroed(worker, allocator, &leaf_layout)
                .expect("shared leaf allocation should succeed");
            write_word(&mut payload, index * REFERENCE_BYTES, leaf.bits());
        }

        let reference = shared
            .allocate_bytes(worker, allocator, &array_layout, &payload)
            .expect("shared reference array allocation should succeed");

        ReferenceArray { reference }
    }
}

/// Encode one machine word into one payload buffer.
fn write_word(bytes: &mut [u8], offset: usize, value: usize) {
    bytes[offset..offset + REFERENCE_BYTES].copy_from_slice(&value.to_le_bytes());
}

/// Build the scan map for one record with a local reference field.
fn local_record_reference_map() -> ReferenceMap {
    ReferenceMap::Direct {
        local_offsets: Box::new([0]),
        shared_offsets: Box::new([]),
    }
}

/// Build the scan map for one record with a shared reference field.
fn shared_record_reference_map() -> ReferenceMap {
    ReferenceMap::Direct {
        local_offsets: Box::new([]),
        shared_offsets: Box::new([0]),
    }
}

/// Build the scan map for one local reference array.
fn local_reference_array_map(objects: usize) -> ReferenceMap {
    ReferenceMap::Repeat {
        count: objects as u32,
        stride: REFERENCE_BYTES as u32,
        element: Box::new(local_record_reference_map()),
    }
}

/// Build the scan map for one shared reference array.
fn shared_reference_array_map(objects: usize) -> ReferenceMap {
    ReferenceMap::Repeat {
        count: objects as u32,
        stride: REFERENCE_BYTES as u32,
        element: Box::new(shared_record_reference_map()),
    }
}
