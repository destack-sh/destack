use crate::diagnostic::{RuntimeError, RuntimeResult};
use destack_heap::{HeapError, HeapLimits, HeapOptions, SharedHeapLimits, SharedHeapOptions};
use destack_repository as repository;

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
    options: &repository::HeapOptions,
) -> RuntimeResult<ResolvedHeapOptions> {
    let heap_options = options
        .local_heap_options()
        .map_err(|error| heap_configuration_error("heap.local", error))?;
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
    options: &repository::HeapOptions,
) -> RuntimeResult<ResolvedSharedHeapOptions> {
    let heap_options = options
        .shared_heap_options()
        .map_err(|error| heap_configuration_error("heap.shared", error))?;
    let limits = SharedHeapLimits {
        max_bytes: options.shared.max_bytes,
        retained_bytes: None,
    };

    Ok(ResolvedSharedHeapOptions {
        limits,
        options: heap_options,
    })
}

/// Map heap configuration failures into runtime diagnostics.
fn heap_configuration_error(scope: &'static str, error: HeapError) -> Box<RuntimeError> {
    RuntimeError::Configuration {
        scope: scope.into(),
        detail: error.to_string(),
    }
    .boxed()
}
