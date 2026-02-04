use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{PlatformError, ResourceId};
use crate::runtime::RuntimeCallContext;

/// Resolve a resource entry for a filesystem handle.
pub(crate) fn require_resource<T>(
    context: &RuntimeCallContext,
    handle: ResourceId,
    kind: ResourceKind,
    label: &str,
    extract: impl FnOnce(&ResourceEntry) -> Option<T>,
) -> RuntimeResult<T> {
    // resolve the resource entry
    let resource = context
        .runtime()
        .resources
        .with_entry(handle, |entry| {
            if entry.kind != kind {
                return None;
            }
            extract(entry)
        })
        .flatten()
        .ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "handle",
                format!("unknown {label} handle"),
            ))
            .boxed()
        })?;

    // return the extracted payload
    Ok(resource)
}

/// Core filesystem interface exposed to bindings.
/// This provides a stable entrypoint that selects the active OS backend.
pub(crate) use super::os::*;
