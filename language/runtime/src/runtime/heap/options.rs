use crate::diagnostic::{RuntimeError, RuntimeResult};
use destack_heap::{
    DEFAULT_ADDRESS_SPACE_SIZE_BYTES, DEFAULT_ALLOCATOR_CHUNK_SIZE_BYTES,
    DEFAULT_ALLOCATOR_PAGE_SIZE_BYTES, DEFAULT_GC_MINIMUM_HEAP_BYTES,
    DEFAULT_GC_MINIMUM_WORK_BYTES, DEFAULT_GC_TRIGGER_PERCENT,
    DEFAULT_MAX_HEAP_YOUNG_ALLOCATION_SIZE_BYTES, DEFAULT_SHARED_SMALL_SIZE_BYTES,
    DEFAULT_SMALL_ALLOCATION_ALIGNMENT_BYTES, DEFAULT_SMALL_SIZE_BYTES, GcOptions, HeapLimits,
    HeapOptions, SharedHeapLimits, SharedHeapOptions, SizeClassTable,
};
use destack_workspace::{HeapOptions as WorkspaceHeapOptions, LocalHeapOptions};

/// Resolved heap construction options for one worker-local heap.
#[derive(Debug, Clone)]
pub struct ResolvedHeapOptions {
    /// The exact retained-byte limits for this heap.
    pub limits: HeapLimits,
    /// The local heap options for this heap.
    pub options: HeapOptions,
}

/// Resolved heap construction options for one runtime-shared heap.
#[derive(Debug, Clone)]
pub struct ResolvedSharedHeapOptions {
    /// The exact retained-byte limits for this heap.
    pub limits: SharedHeapLimits,
    /// The shared heap options for this heap.
    pub options: SharedHeapOptions,
}

/// Resolve runtime heap options into worker-local heap settings.
pub fn resolve_local_heap_options(
    options: &WorkspaceHeapOptions,
) -> RuntimeResult<ResolvedHeapOptions> {
    let heap_options = resolve_local_heap_policy(options, "heap.local")?;
    let limits = HeapLimits {
        max_bytes: options.local.max_bytes,
        retained_bytes: None,
    };

    Ok(ResolvedHeapOptions {
        limits,
        options: heap_options,
    })
}

/// Resolve runtime heap options into runtime-shared heap settings.
pub fn resolve_shared_heap_options(
    options: &WorkspaceHeapOptions,
) -> RuntimeResult<ResolvedSharedHeapOptions> {
    let heap_options = resolve_shared_heap_policy(options, "heap.shared")?;
    let limits = SharedHeapLimits {
        max_bytes: options.shared.max_bytes,
        retained_bytes: None,
    };

    Ok(ResolvedSharedHeapOptions {
        limits,
        options: heap_options,
    })
}

/// Build one local heap policy into exact heap construction options.
fn resolve_local_heap_policy(
    options: &WorkspaceHeapOptions,
    scope: &'static str,
) -> RuntimeResult<HeapOptions> {
    let heap_options = HeapOptions {
        gc: local_gc_options(options),
        size_classes: SizeClassTable::default(),
        heap_young_size_bytes: options.local.young_size_bytes,
        max_heap_young_allocation_size_bytes: DEFAULT_MAX_HEAP_YOUNG_ALLOCATION_SIZE_BYTES,
        heap_small_size_bytes: DEFAULT_SMALL_SIZE_BYTES,
        address_space_size_bytes: DEFAULT_ADDRESS_SPACE_SIZE_BYTES,
        page_size_bytes: DEFAULT_ALLOCATOR_PAGE_SIZE_BYTES,
        allocator_chunk_size_bytes: DEFAULT_ALLOCATOR_CHUNK_SIZE_BYTES,
        small_allocation_alignment_bytes: DEFAULT_SMALL_ALLOCATION_ALIGNMENT_BYTES,
    };

    heap_options.validate_local().map_err(|error| {
        RuntimeError::Configuration {
            scope: scope.into(),
            detail: error.to_string(),
        }
        .boxed()
    })?;

    Ok(heap_options)
}

/// Build one shared heap policy into exact heap construction options.
fn resolve_shared_heap_policy(
    options: &WorkspaceHeapOptions,
    scope: &'static str,
) -> RuntimeResult<SharedHeapOptions> {
    let heap_options = SharedHeapOptions {
        gc: shared_gc_options(options),
        size_classes: SizeClassTable::default(),
        heap_small_size_bytes: DEFAULT_SHARED_SMALL_SIZE_BYTES,
        address_space_size_bytes: DEFAULT_ADDRESS_SPACE_SIZE_BYTES,
        page_size_bytes: DEFAULT_ALLOCATOR_PAGE_SIZE_BYTES,
        allocator_chunk_size_bytes: DEFAULT_ALLOCATOR_CHUNK_SIZE_BYTES,
        small_allocation_alignment_bytes: DEFAULT_SMALL_ALLOCATION_ALIGNMENT_BYTES,
    };

    heap_options.validate().map_err(|error| {
        RuntimeError::Configuration {
            scope: scope.into(),
            detail: error.to_string(),
        }
        .boxed()
    })?;

    Ok(heap_options)
}

/// Resolve local collector policy.
fn local_gc_options(options: &WorkspaceHeapOptions) -> GcOptions {
    GcOptions {
        growth_percent: options.growth_percent,
        trigger_percent: DEFAULT_GC_TRIGGER_PERCENT,
        soft_limit_bytes: options.memory_limit_bytes,
        minimum_heap_bytes: Some(local_min_bytes(&options.local)),
        minimum_work_bytes: DEFAULT_GC_MINIMUM_WORK_BYTES,
    }
}

/// Resolve shared collector policy.
fn shared_gc_options(options: &WorkspaceHeapOptions) -> GcOptions {
    GcOptions {
        growth_percent: options.growth_percent,
        trigger_percent: DEFAULT_GC_TRIGGER_PERCENT,
        soft_limit_bytes: options.memory_limit_bytes,
        minimum_heap_bytes: Some(
            options
                .shared
                .min_bytes
                .unwrap_or(DEFAULT_GC_MINIMUM_HEAP_BYTES),
        ),
        minimum_work_bytes: DEFAULT_GC_MINIMUM_WORK_BYTES,
    }
}

/// Resolve the local heap minimum from nursery width.
fn local_min_bytes(options: &LocalHeapOptions) -> u64 {
    let young_min_size_bytes = 4 * options.young_size_bytes as u64;

    options
        .min_bytes
        .unwrap_or(DEFAULT_GC_MINIMUM_HEAP_BYTES.max(young_min_size_bytes))
}
