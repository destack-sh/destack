use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use destack_mir::TraceId;

use crate::{
    AllocationClass, AllocationPlan, AllocationShape, DEFAULT_HEAP_PAGE_SIZE_BYTES,
    DEFAULT_SMALL_ALLOCATION_ALIGNMENT_BYTES, DEFAULT_SMALL_SIZE_BYTES, DropPlan, GcOptions,
    HeapError, SizeClassTable, validate_page_size_bytes, validate_size_class_alignment,
    validate_small_span_size_bytes,
};

/// The configuration for one heap instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct HeapOptions {
    /// The collector configuration.
    pub gc: GcOptions,
    /// The configured small-allocation class table.
    pub size_classes: SizeClassTable,
    /// The byte size for heap small-block spans.
    pub heap_small_size_bytes: usize,
    /// The byte size for memory pages.
    pub page_size_bytes: usize,
    /// The required alignment for configured small-allocation classes.
    pub small_allocation_alignment_bytes: usize,
}

impl HeapOptions {
    /// Resolve one heap allocation plan for this allocation shape.
    #[inline(always)]
    pub fn allocation_plan(&self, shape: &AllocationShape) -> AllocationPlan {
        let class = self.classify_allocation(shape);

        AllocationPlan::new(shape, class)
    }

    /// Resolve one heap allocation class for this allocation shape.
    #[inline(always)]
    fn classify_allocation(&self, shape: &AllocationShape) -> AllocationClass {
        // keep allocation-specific metadata out of table-backed small spans
        if shape.requires_individual_metadata() {
            return AllocationClass::large();
        }

        self.allocation_class(
            shape.byte_len,
            shape.alignment,
            shape.trace_id,
            shape.drop,
            shape.is_noscan,
        )
    }

    /// Resolve one heap allocation class for allocation parameters.
    #[inline(always)]
    pub(crate) fn allocation_class(
        &self,
        byte_len: usize,
        alignment: usize,
        trace_id: Option<TraceId>,
        drop: Option<DropPlan>,
        is_noscan: bool,
    ) -> AllocationClass {
        AllocationClass::select(
            byte_len,
            alignment,
            trace_id,
            drop,
            is_noscan,
            &self.size_classes,
            self.page_size_bytes,
            self.heap_small_size_bytes,
        )
    }

    /// Build the default option set for one heap.
    pub fn local() -> Self {
        Self {
            gc: GcOptions::default(),
            size_classes: SizeClassTable::default(),
            heap_small_size_bytes: DEFAULT_SMALL_SIZE_BYTES,
            page_size_bytes: DEFAULT_HEAP_PAGE_SIZE_BYTES,
            small_allocation_alignment_bytes: DEFAULT_SMALL_ALLOCATION_ALIGNMENT_BYTES,
        }
    }

    /// Validate these options for one heap.
    pub fn validate_local(&self) -> Result<(), HeapError> {
        self.gc.validate()?;
        validate_page_size_bytes(self.page_size_bytes)?;
        validate_size_class_alignment(&self.size_classes, self.small_allocation_alignment_bytes)?;
        validate_small_span_size_bytes(self.heap_small_size_bytes, &self.size_classes)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use destack_memory::MemoryMap;
    use destack_mir::TraceMap;

    use crate::{
        AllocationShape, DropId, GcOptions, Heap, HeapConfigurationError, HeapError, HeapLimits,
        HeapOptions, SharedHeapOptions, SizeClassTable,
    };

    /// Route repeated drops to exact-sized large blocks.
    #[test]
    fn test_route_repeated_drop_to_large_blocks() {
        let drop = DropId::from_index(0);
        let element = AllocationShape::new(8, 8, None, TraceMap::empty())
            .with_drop(drop)
            .expect("Drop plan should build");
        let element = HeapOptions::local().allocation_plan(&element);
        let shape = element
            .repeat(&TraceMap::empty(), 2)
            .expect("repeated allocation should build");

        let local = HeapOptions::local().allocation_plan(&shape);
        let shared = SharedHeapOptions::default().allocation_plan(&shape);

        assert!(local.small_allocation().is_none());
        assert!(shared.small_allocation().is_none());
    }

    /// Route allocation-specific trace maps away from table-backed small spans.
    #[test]
    fn test_route_dynamic_trace_to_individual_metadata() {
        let trace_map = TraceMap::Fixed {
            local_offsets: vec![0, 8].into(),
            shared_offsets: vec![].into(),
            frame_offsets: vec![].into(),
            borrow_offsets: vec![].into(),
        };
        let shape = AllocationShape::new(16, 8, None, trace_map);

        let local = HeapOptions::local().allocation_plan(&shape);
        let shared = SharedHeapOptions::default().allocation_plan(&shape);

        assert!(local.small_allocation().is_none());
        assert!(shared.small_allocation().is_none());
    }

    /// Reject invalid GC trigger percentages at heap construction.
    #[test]
    fn test_heap_rejects_invalid_gc_trigger_percent() {
        let options = HeapOptions {
            gc: GcOptions {
                trigger_percent: 101,
                ..HeapOptions::local().gc
            },
            ..HeapOptions::local()
        };

        let memory = Arc::new(
            MemoryMap::reserve(1024 * 1024 * 1024, options.page_size_bytes)
                .expect("test World memory should reserve"),
        );
        let error = Heap::new(memory, HeapLimits::default(), options)
            .expect_err("invalid heap options should fail loudly");

        assert_eq!(
            error,
            HeapError::configuration(HeapConfigurationError::InvalidGcTriggerPercent {
                percent: 101,
            })
        );
    }

    /// Reject misaligned size classes under one explicit heap alignment.
    #[test]
    fn test_heap_rejects_misaligned_size_class_table() {
        let options = HeapOptions {
            size_classes: SizeClassTable::new([16, 24, 32])
                .expect("size classes should validate structurally"),
            small_allocation_alignment_bytes: 16,
            ..HeapOptions::local()
        };

        let memory = Arc::new(
            MemoryMap::reserve(1024 * 1024 * 1024, options.page_size_bytes)
                .expect("test World memory should reserve"),
        );
        let error = Heap::new(memory, HeapLimits::default(), options)
            .expect_err("invalid heap options should fail loudly");

        assert_eq!(
            error,
            HeapError::configuration(HeapConfigurationError::MisalignedSizeClass {
                alignment_bytes: 16,
                class_bytes: 24,
            })
        );
    }
}
