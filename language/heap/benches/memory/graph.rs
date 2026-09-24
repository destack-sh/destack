use std::sync::Arc;

use destack_heap::{
    AllocationCache, AllocationPlan, AllocationShape, Heap, HeapLimits, HeapOptions, HeapReference,
    SharedHeap, SharedHeapReference, SharedMarkWorker,
};
use destack_memory::MemoryMap;
use destack_mir::TraceMap;

use crate::config::{LEAF_BYTES, RECORD_BYTES, REFERENCE_BYTES, WORKLOAD_OBJECTS};
use crate::heap::local_memory;
use crate::trace::BenchTraceTable;

/// One allocated object graph.
pub(crate) struct ObjectGraph<R> {
    /// The allocated record references.
    pub(crate) records: Vec<R>,
    /// The trace table required to scan this workload.
    pub(crate) trace_table: BenchTraceTable,
}

/// One allocated fixed reference array.
pub(crate) struct ReferenceArray<R> {
    /// The allocated array reference.
    pub(crate) reference: R,
    /// The trace table required to scan this workload.
    pub(crate) trace_table: BenchTraceTable,
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
        let trace_map = local_record_trace_map();

        self.allocate_local_with_map(heap, &trace_map)
    }

    /// Allocate this workload in one shared heap.
    pub(crate) fn allocate_shared(
        self,
        shared: &SharedHeap,
        worker: &SharedMarkWorker,
        cache: &mut AllocationCache,
    ) -> ObjectGraph<SharedHeapReference> {
        let trace_map = shared_record_trace_map();

        self.allocate_shared_with_map(shared, worker, cache, &trace_map)
    }

    /// Build one local heap with this workload ready to fork.
    pub(crate) fn local_heap(self) -> (Arc<MemoryMap>, Heap, ObjectGraph<HeapReference>) {
        let memory = local_memory();
        let mut heap = Heap::new(memory.clone(), HeapLimits::default(), HeapOptions::local())
            .expect("heap should build");
        let graph = self.allocate_local(&mut heap);

        (memory, heap, graph)
    }

    /// Allocate this workload in one local heap with a known trace map.
    fn allocate_local_with_map(
        self,
        heap: &mut Heap,
        trace_map: &TraceMap,
    ) -> ObjectGraph<HeapReference> {
        let mut source_traces = destack_mir::TraceTable::new();
        let record_trace_id = source_traces.insert(trace_map.clone());
        let trace_table = BenchTraceTable::from_mir(&source_traces);
        let leaf_map = TraceMap::Empty;
        let leaf_shape = AllocationShape::new(self.leaf_bytes, 1, None, leaf_map);
        let record_shape = AllocationShape::new(
            self.record_bytes,
            REFERENCE_BYTES,
            Some(record_trace_id),
            trace_map.clone(),
        );
        let leaf_plan = local_allocation_plan(heap, &leaf_shape);
        let record_plan = local_allocation_plan(heap, &record_shape);
        let mut records = Vec::with_capacity(self.objects);

        // allocate leaf and record pairs
        for index in 0..self.objects {
            let leaf = heap
                .allocate_zeroed(leaf_plan, &leaf_shape.trace_map)
                .expect("leaf allocation should succeed");
            let mut record = vec![0u8; self.record_bytes];
            write_word(&mut record, 0, leaf.bits());
            write_word(&mut record, REFERENCE_BYTES, index);

            let reference = heap
                .allocate_bytes(record_plan, &record_shape.trace_map, &record)
                .expect("record allocation should succeed");
            records.push(reference);
        }

        ObjectGraph {
            records,
            trace_table,
        }
    }

    /// Allocate this workload in one shared heap with a known trace map.
    fn allocate_shared_with_map(
        self,
        shared: &SharedHeap,
        worker: &SharedMarkWorker,
        cache: &mut AllocationCache,
        trace_map: &TraceMap,
    ) -> ObjectGraph<SharedHeapReference> {
        let mut source_traces = destack_mir::TraceTable::new();
        let record_trace_id = source_traces.insert(trace_map.clone());
        let trace_table = BenchTraceTable::from_mir(&source_traces);
        let trace_view = trace_table.view();
        let leaf_map = TraceMap::Empty;
        let leaf_shape = AllocationShape::new(self.leaf_bytes, 1, None, leaf_map);
        let record_shape = AllocationShape::new(
            self.record_bytes,
            REFERENCE_BYTES,
            Some(record_trace_id),
            trace_map.clone(),
        );
        let leaf_plan = shared_allocation_plan(shared, &leaf_shape);
        let record_plan = shared_allocation_plan(shared, &record_shape);
        let mut records = Vec::with_capacity(self.objects);

        // allocate leaf and record pairs through one worker cache
        for index in 0..self.objects {
            let leaf = shared
                .allocate_zeroed(worker, cache, leaf_plan, &leaf_shape.trace_map, trace_view)
                .expect("shared leaf allocation should succeed");
            let mut record = vec![0u8; self.record_bytes];
            write_word(&mut record, 0, leaf.bits());
            write_word(&mut record, REFERENCE_BYTES, index);

            let reference = shared
                .allocate_bytes(
                    worker,
                    cache,
                    record_plan,
                    &record_shape.trace_map,
                    &record,
                    trace_view,
                )
                .expect("shared record allocation should succeed");
            records.push(reference);
        }

        ObjectGraph {
            records,
            trace_table,
        }
    }
}

/// One reference array workload backed by a repeated trace map.
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
        let mut source_traces = destack_mir::TraceTable::new();
        let trace_map = local_reference_array_map(self.objects);
        let trace_id = source_traces.insert(trace_map.clone());
        let trace_table = BenchTraceTable::from_mir(&source_traces);
        let leaf_map = TraceMap::Empty;
        let leaf_shape = AllocationShape::new(self.leaf_bytes, 1, None, leaf_map);
        let array_shape = AllocationShape::new(
            self.objects * REFERENCE_BYTES,
            REFERENCE_BYTES,
            Some(trace_id),
            trace_map,
        );
        let leaf_plan = local_allocation_plan(heap, &leaf_shape);
        let array_plan = local_allocation_plan(heap, &array_shape);
        let mut payload = vec![0u8; self.objects * REFERENCE_BYTES];

        // build the array payload from fresh leaf references
        for index in 0..self.objects {
            let leaf = heap
                .allocate_zeroed(leaf_plan, &leaf_shape.trace_map)
                .expect("leaf allocation should succeed");
            write_word(&mut payload, index * REFERENCE_BYTES, leaf.bits());
        }

        let reference = heap
            .allocate_bytes(array_plan, &array_shape.trace_map, &payload)
            .expect("reference array allocation should succeed");

        ReferenceArray {
            reference,
            trace_table,
        }
    }

    /// Allocate this workload in one shared heap.
    pub(crate) fn allocate_shared(
        self,
        shared: &SharedHeap,
        worker: &SharedMarkWorker,
        cache: &mut AllocationCache,
    ) -> ReferenceArray<SharedHeapReference> {
        let mut source_traces = destack_mir::TraceTable::new();
        let trace_map = shared_reference_array_map(self.objects);
        let trace_id = source_traces.insert(trace_map.clone());
        let trace_table = BenchTraceTable::from_mir(&source_traces);
        let trace_view = trace_table.view();
        let leaf_map = TraceMap::Empty;
        let leaf_shape = AllocationShape::new(self.leaf_bytes, 1, None, leaf_map);
        let array_shape = AllocationShape::new(
            self.objects * REFERENCE_BYTES,
            REFERENCE_BYTES,
            Some(trace_id),
            trace_map,
        );
        let leaf_plan = shared_allocation_plan(shared, &leaf_shape);
        let array_plan = shared_allocation_plan(shared, &array_shape);
        let mut payload = vec![0u8; self.objects * REFERENCE_BYTES];

        // build the array payload from fresh leaf references
        for index in 0..self.objects {
            let leaf = shared
                .allocate_zeroed(worker, cache, leaf_plan, &leaf_shape.trace_map, trace_view)
                .expect("shared leaf allocation should succeed");
            write_word(&mut payload, index * REFERENCE_BYTES, leaf.bits());
        }

        let reference = shared
            .allocate_bytes(
                worker,
                cache,
                array_plan,
                &array_shape.trace_map,
                &payload,
                trace_view,
            )
            .expect("shared reference array allocation should succeed");

        ReferenceArray {
            reference,
            trace_table,
        }
    }
}

/// Encode one machine word into one payload buffer.
fn write_word(bytes: &mut [u8], offset: usize, value: usize) {
    bytes[offset..offset + REFERENCE_BYTES].copy_from_slice(&value.to_le_bytes());
}

/// Build one explicit local allocation plan for graph workloads.
#[inline(always)]
fn local_allocation_plan(heap: &Heap, shape: &AllocationShape) -> AllocationPlan {
    heap.options().allocation_plan(shape)
}

/// Build one explicit shared allocation plan for graph workloads.
#[inline(always)]
fn shared_allocation_plan(shared: &SharedHeap, shape: &AllocationShape) -> AllocationPlan {
    shared.options().allocation_plan(shape)
}

/// Build the scan map for one record with a local reference field.
fn local_record_trace_map() -> TraceMap {
    TraceMap::Fixed {
        local_offsets: Box::new([0]),
        shared_offsets: Box::new([]),
        frame_offsets: Box::new([]),
        borrow_offsets: Box::default(),
    }
}

/// Build the scan map for one record with a shared reference field.
fn shared_record_trace_map() -> TraceMap {
    TraceMap::Fixed {
        local_offsets: Box::new([]),
        shared_offsets: Box::new([0]),
        frame_offsets: Box::new([]),
        borrow_offsets: Box::default(),
    }
}

/// Build the scan map for one local reference array.
fn local_reference_array_map(objects: usize) -> TraceMap {
    TraceMap::Repeated {
        count: objects as u32,
        stride: REFERENCE_BYTES as u32,
        element: Box::new(local_record_trace_map()),
    }
}

/// Build the scan map for one shared reference array.
fn shared_reference_array_map(objects: usize) -> TraceMap {
    TraceMap::Repeated {
        count: objects as u32,
        stride: REFERENCE_BYTES as u32,
        element: Box::new(shared_record_trace_map()),
    }
}
