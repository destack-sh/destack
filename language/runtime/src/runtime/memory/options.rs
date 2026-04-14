use crate::diagnostic::{RuntimeError, RuntimeResult};
use destack_heap::{HeapLayout, HeapLimits, ManagedLimits, RawLimits, SizeClassTable};
use destack_workspace::{HeapOptions, HeapSizeClasses};

/// Resolved heap construction options for one agent heap.
#[derive(Debug, Clone)]
pub struct ResolvedHeapOptions {
    /// The exact retained-byte limits for this heap.
    pub limits: HeapLimits,
    /// The local heap layout configuration for this heap.
    pub layout: HeapLayout,
}

/// Resolve runtime heap options into heap-construction settings.
pub fn resolve_heap_options(
    options: &HeapOptions,
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
        HeapSizeClasses::Explicit(classes) => SizeClassTable::new("explicit", classes.clone())
            .map_err(|error| {
                RuntimeError::Internal {
                    message: format!("invalid heap size-class configuration: {error:?}"),
                }
                .boxed()
            })?,
    };

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
        layout: HeapLayout {
            size_classes,
            managed_reference_bytes,
            managed_young_bytes: options.managed_young_bytes,
            managed_small_bytes: options.managed_small_bytes,
            raw_small_bytes: options.raw_small_bytes,
            page_bytes: options.page_bytes,
        },
    })
}
