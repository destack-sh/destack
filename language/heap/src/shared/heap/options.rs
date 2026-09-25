use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use tspp_mir::TraceId;

use crate::{
    AllocationClass, AllocationPlan, AllocationShape, DEFAULT_HEAP_PAGE_SIZE_BYTES,
    DEFAULT_SHARED_SMALL_SIZE_BYTES, DEFAULT_SMALL_ALLOCATION_ALIGNMENT_BYTES, DropPlan, GcOptions,
    HeapError, SizeClassTable, validate_page_size_bytes, validate_size_class_alignment,
    validate_small_span_size_bytes,
};

/// The configuration for one shared heap instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SharedHeapOptions {
    /// The collector configuration.
    pub gc: GcOptions,
    /// The configured small-block class table.
    pub size_classes: SizeClassTable,
    /// The byte size for shared heap small-block spans.
    pub heap_small_size_bytes: usize,
    /// The byte size for memory pages.
    pub page_size_bytes: usize,
    /// The required alignment for configured small-block classes.
    pub small_allocation_alignment_bytes: usize,
}

impl Default for SharedHeapOptions {
    fn default() -> Self {
        Self {
            gc: GcOptions::default(),
            size_classes: SizeClassTable::default(),
            heap_small_size_bytes: DEFAULT_SHARED_SMALL_SIZE_BYTES,
            page_size_bytes: DEFAULT_HEAP_PAGE_SIZE_BYTES,
            small_allocation_alignment_bytes: DEFAULT_SMALL_ALLOCATION_ALIGNMENT_BYTES,
        }
    }
}

impl SharedHeapOptions {
    /// Resolve one shared heap allocation plan for this allocation shape.
    #[inline(always)]
    pub fn allocation_plan(&self, shape: &AllocationShape) -> AllocationPlan {
        let class = self.classify_allocation(shape);

        AllocationPlan::new(shape, class)
    }

    /// Resolve one shared heap allocation class for this allocation shape.
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

    /// Resolve one shared heap allocation class for allocation parameters.
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

    /// Validate these options for one shared heap.
    pub fn validate(&self) -> Result<(), HeapError> {
        self.gc.validate()?;
        validate_page_size_bytes(self.page_size_bytes)?;
        validate_size_class_alignment(&self.size_classes, self.small_allocation_alignment_bytes)?;
        validate_small_span_size_bytes(self.heap_small_size_bytes, &self.size_classes)?;

        Ok(())
    }
}
