use crate::diagnostic::{RuntimeError, RuntimeResult};
use destack_heap::{
    HeapLimits, HeapOptions, ManagedLimits, RawLimits, SharedRawLimits, SizeClassTable,
};
use destack_workspace::{HeapOptions as WorkspaceHeapOptions, HeapSizeClasses};

/// Resolved heap construction options for one worker heap.
#[derive(Debug, Clone)]
pub struct ResolvedHeapOptions {
    /// The exact retained-byte limits for this heap.
    pub limits: HeapLimits,
    /// The local heap options for this heap.
    pub options: HeapOptions,
}

/// Resolve runtime heap options into heap-construction settings.
pub fn resolve_heap_options(
    options: &WorkspaceHeapOptions,
    managed_reference_bytes: u8,
) -> RuntimeResult<ResolvedHeapOptions> {
    let size_classes = match &options.size_classes {
        HeapSizeClasses::Default => SizeClassTable::default(),
        HeapSizeClasses::Named(name) => {
            if name == "default" {
                SizeClassTable::default()
            } else {
                return Err(RuntimeError::Internal {
                    message: format!("unknown heap size-class preset: {name}"),
                }
                .boxed());
            }
        }
        HeapSizeClasses::Explicit(classes) => {
            SizeClassTable::new(classes.clone()).map_err(|error| {
                RuntimeError::ConfigurationInvalid {
                    scope: "heap.sizeClasses".into(),
                    detail: format!("{error:?}"),
                }
                .boxed()
            })?
        }
    };

    let heap_options = HeapOptions {
        size_classes,
        managed_reference_bytes,
        managed_young_bytes: options.managed_young_bytes,
        max_managed_young_allocation_bytes: options.max_managed_young_allocation_bytes,
        managed_small_bytes: options.managed_small_bytes,
        raw_small_bytes: options.raw_small_bytes,
        page_bytes: options.page_bytes,
        arena_segment_bytes: options.arena_segment_bytes,
        card_bytes: options.card_bytes,
        small_allocation_alignment_bytes: options.small_allocation_alignment_bytes,
        table_chunk_len: options.table_chunk_len,
    };

    heap_options.validate().map_err(|error| {
        RuntimeError::ConfigurationInvalid {
            scope: "heap".into(),
            detail: error.to_string(),
        }
        .boxed()
    })?;

    Ok(ResolvedHeapOptions {
        limits: HeapLimits {
            max_bytes: options.max_bytes,
            managed: ManagedLimits {
                max_bytes: options.max_managed_bytes,
            },
            raw: RawLimits {
                max_bytes: options.max_raw_bytes,
            },
        },
        options: heap_options,
    })
}

/// Resolve the shared raw-space limits for one runtime heap configuration.
pub fn resolve_shared_raw_limits(options: &WorkspaceHeapOptions) -> SharedRawLimits {
    SharedRawLimits {
        max_bytes: options.max_shared_bytes,
    }
}
