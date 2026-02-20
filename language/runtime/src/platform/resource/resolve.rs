use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{PlatformError, ResourceId};
use crate::runtime::BindingCallContext;

use super::{ResourceEntry, ResourceKind};

/// Resolve a resource entry for the requested handle and kind.
#[allow(dead_code)]
pub(crate) fn require_resource<T>(
    context: &BindingCallContext,
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
