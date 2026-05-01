use crate::diagnostic::{RuntimeError, RuntimeResult};
use destack_heap::{
    DEFAULT_GC_MINIMUM_WORK_BYTES, GcOptions, HeapLimits, HeapOptions, HeapSpaceLimits, RawLimits,
    SharedHeapLimits, SharedHeapSpaceLimits, SharedRawLimits, SizeClassTable,
};
use destack_workspace::{
    HeapLayoutOptions, HeapOptions as WorkspaceHeapOptions, HeapSizeClasses, LocalGcOptions,
    LocalHeapLimitOptions, SharedGcOptions, SharedHeapLimitOptions,
};

/// Resolved heap construction options for one worker-local heap.
#[derive(Debug, Clone)]
pub struct ResolvedHeapOptions {
    /// The exact retained-byte limits for this heap.
    pub limits: HeapLimits,
    /// The local heap options for this heap.
    pub options: HeapOptions,
}

/// Resolved heap construction options for one world-shared heap.
#[derive(Debug, Clone)]
pub struct ResolvedSharedHeapOptions {
    /// The exact retained-byte limits for this heap.
    pub limits: SharedHeapLimits,
    /// The shared heap options for this heap.
    pub options: HeapOptions,
}

/// Resolve runtime heap options into worker-local heap settings.
pub fn resolve_local_heap_options(
    options: &WorkspaceHeapOptions,
) -> RuntimeResult<ResolvedHeapOptions> {
    let heap_options = resolve_local_heap_policy(&options.gc.local, &options.layout, "heap.local")?;
    let limits = resolve_local_heap_limits(&options.limit.local);

    Ok(ResolvedHeapOptions {
        limits,
        options: heap_options,
    })
}

/// Resolve runtime heap options into world-shared heap settings.
pub fn resolve_shared_heap_options(
    options: &WorkspaceHeapOptions,
) -> RuntimeResult<ResolvedSharedHeapOptions> {
    check_shared_heap_limits(&options.limit.shared)?;

    let heap_options =
        resolve_shared_heap_policy(&options.gc.shared, &options.layout, "heap.shared")?;
    let limits = resolve_shared_heap_limits(&options.limit.shared);

    Ok(ResolvedSharedHeapOptions {
        limits,
        options: heap_options,
    })
}

/// Resolve one shared-memory size-class table.
fn resolve_size_classes(size_classes: &HeapSizeClasses) -> RuntimeResult<SizeClassTable> {
    match size_classes {
        HeapSizeClasses::Default => Ok(SizeClassTable::default()),
        HeapSizeClasses::Named(name) => {
            if name == "default" {
                Ok(SizeClassTable::default())
            } else {
                Err(RuntimeError::Internal {
                    message: format!("unknown heap size-class preset: {name}"),
                }
                .boxed())
            }
        }
        HeapSizeClasses::Explicit(classes) => {
            SizeClassTable::new(classes.clone()).map_err(|error| {
                RuntimeError::ConfigurationInvalid {
                    scope: "heap.layout.sizeClasses".into(),
                    detail: format!("{error:?}"),
                }
                .boxed()
            })
        }
    }
}

/// Build one local heap policy plus layout geometry into heap options.
fn resolve_local_heap_policy(
    gc: &impl HeapGcConfig,
    layout: &HeapLayoutOptions,
    scope: &'static str,
) -> RuntimeResult<HeapOptions> {
    let size_classes = resolve_size_classes(&layout.size_classes)?;
    let heap_options = HeapOptions {
        gc: resolved_gc_options(gc),
        size_classes,
        heap_young_bytes: layout.heap_young_bytes,
        max_heap_young_allocation_bytes: layout.max_heap_young_allocation_bytes,
        heap_small_bytes: layout.heap_span_bytes,
        raw_small_bytes: layout.raw_span_bytes,
        heap_space_bytes: layout.heap_space_bytes,
        raw_space_bytes: layout.raw_space_bytes,
        page_bytes: layout.page_bytes,
        allocator_chunk_bytes: layout.chunk_bytes,
        small_allocation_alignment_bytes: layout.small_alignment_bytes,
    };

    heap_options.validate_local().map_err(|error| {
        RuntimeError::ConfigurationInvalid {
            scope: scope.into(),
            detail: error.to_string(),
        }
        .boxed()
    })?;

    Ok(heap_options)
}

/// Build one shared heap policy plus layout geometry into heap options.
fn resolve_shared_heap_policy(
    gc: &impl HeapGcConfig,
    layout: &HeapLayoutOptions,
    scope: &'static str,
) -> RuntimeResult<HeapOptions> {
    let size_classes = resolve_size_classes(&layout.size_classes)?;
    let heap_options = HeapOptions {
        gc: resolved_gc_options(gc),
        size_classes,
        heap_young_bytes: 0,
        max_heap_young_allocation_bytes: 0,
        heap_small_bytes: layout.shared_heap_span_bytes,
        raw_small_bytes: layout.raw_span_bytes,
        heap_space_bytes: layout.heap_space_bytes,
        raw_space_bytes: layout.raw_space_bytes,
        page_bytes: layout.page_bytes,
        allocator_chunk_bytes: layout.chunk_bytes,
        small_allocation_alignment_bytes: layout.small_alignment_bytes,
    };

    heap_options.validate_shared().map_err(|error| {
        RuntimeError::ConfigurationInvalid {
            scope: scope.into(),
            detail: error.to_string(),
        }
        .boxed()
    })?;

    Ok(heap_options)
}

/// Build one resolved heap collector config.
fn resolved_gc_options(gc: &impl HeapGcConfig) -> GcOptions {
    GcOptions {
        growth_percent: gc.growth_percent(),
        trigger_percent: gc.trigger_percent(),
        soft_limit_bytes: gc.memory_limit_bytes(),
        minimum_heap_bytes: gc.minimum_heap_bytes(),
        minimum_work_bytes: DEFAULT_GC_MINIMUM_WORK_BYTES,
    }
}

/// Resolve one worker-local heap limit profile.
fn resolve_local_heap_limits(limits: &LocalHeapLimitOptions) -> HeapLimits {
    HeapLimits {
        max_bytes: limits.max_bytes,
        heap: HeapSpaceLimits {
            max_bytes: limits.heap_max_bytes,
        },
        raw: RawLimits {
            max_bytes: limits.raw_max_bytes,
        },
    }
}

/// Resolve one world-shared heap limit profile.
fn resolve_shared_heap_limits(limits: &SharedHeapLimitOptions) -> SharedHeapLimits {
    SharedHeapLimits {
        max_bytes: limits.max_bytes,
        heap: SharedHeapSpaceLimits {
            max_bytes: limits.heap_max_bytes,
        },
        raw: SharedRawLimits {
            max_bytes: limits.raw_max_bytes,
        },
    }
}

/// Check the semantic shape of one shared-heap limit profile.
fn check_shared_heap_limits(limits: &SharedHeapLimitOptions) -> RuntimeResult<()> {
    let has_total_limit = limits.max_bytes.is_some();
    let has_partial_space_limits = limits.heap_max_bytes.is_some() ^ limits.raw_max_bytes.is_some();

    // mixed total plus one-sided caps is ambiguous
    if has_total_limit && has_partial_space_limits {
        return Err(RuntimeError::ConfigurationInvalid {
            scope: "heap.limit.shared".into(),
            detail:
                "shared total heap limits cannot be combined with only one explicit shared-space cap"
                    .into(),
        }
        .boxed());
    }

    Ok(())
}

/// One GC config surface that can feed heap pacing.
trait HeapGcConfig {
    /// Return the configured growth target percentage.
    fn growth_percent(&self) -> u32;

    /// Return the configured trigger percentage.
    fn trigger_percent(&self) -> u32;

    /// Return the configured soft memory limit.
    fn memory_limit_bytes(&self) -> Option<u64>;

    /// Return the configured minimum live heap floor.
    fn minimum_heap_bytes(&self) -> Option<u64>;
}

impl HeapGcConfig for LocalGcOptions {
    fn growth_percent(&self) -> u32 {
        self.growth_percent
    }

    fn trigger_percent(&self) -> u32 {
        self.trigger_percent
    }

    fn memory_limit_bytes(&self) -> Option<u64> {
        self.memory_limit_bytes
    }

    fn minimum_heap_bytes(&self) -> Option<u64> {
        self.minimum_heap_bytes
    }
}

impl HeapGcConfig for SharedGcOptions {
    fn growth_percent(&self) -> u32 {
        self.growth_percent
    }

    fn trigger_percent(&self) -> u32 {
        self.trigger_percent
    }

    fn memory_limit_bytes(&self) -> Option<u64> {
        self.memory_limit_bytes
    }

    fn minimum_heap_bytes(&self) -> Option<u64> {
        self.minimum_heap_bytes
    }
}
