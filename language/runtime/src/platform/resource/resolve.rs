use crate::diagnostic::RuntimeResult;
use crate::platform::{ResourceId, core as core_platform};
use crate::runtime::BindingCallContext;

use super::{ResourceEntry, ResourceHandle, ResourceKind};

/// Resolve one resource entry without kind or label filtering.
pub(crate) fn with_any_entry<R>(
    context: &BindingCallContext,
    handle: ResourceId,
    read: impl FnOnce(&ResourceEntry) -> R,
) -> Option<R> {
    context.runtime().resources.with_entry(handle, read)
}

/// Return whether one resource entry matches one kind and optional label.
fn entry_matches(entry: &ResourceEntry, kind: ResourceKind, label: Option<&str>) -> bool {
    if entry.kind != kind {
        return false;
    }

    if let Some(expected_label) = label
        && entry.label.as_deref() != Some(expected_label)
    {
        return false;
    }

    true
}

/// Resolve one resource entry by kind and optional label.
pub(crate) fn with_entry<R>(
    context: &BindingCallContext,
    handle: ResourceId,
    kind: ResourceKind,
    label: Option<&str>,
    read: impl FnOnce(&ResourceEntry) -> R,
) -> Option<R> {
    context
        .runtime()
        .resources
        .with_entry(handle, |entry| {
            if !entry_matches(entry, kind, label) {
                return None;
            }

            Some(read(entry))
        })
        .flatten()
}

/// Resolve one mutable resource entry by kind and optional label.
pub(crate) fn with_entry_mut<R>(
    context: &BindingCallContext,
    handle: ResourceId,
    kind: ResourceKind,
    label: Option<&str>,
    write: impl FnOnce(&mut ResourceEntry) -> R,
) -> Option<R> {
    context
        .runtime()
        .resources
        .with_entry_mut(handle, |entry| {
            if !entry_matches(entry, kind, label) {
                return None;
            }

            Some(write(entry))
        })
        .flatten()
}

/// Resolve one typed payload by resource kind and optional label.
pub(crate) fn resolve_payload<T: Clone + Send + Sync + 'static>(
    context: &BindingCallContext,
    handle: ResourceId,
    kind: ResourceKind,
    label: Option<&str>,
) -> Option<T> {
    with_entry(context, handle, kind, label, |entry| {
        entry.payload_cloned::<T>()
    })
    .flatten()
}

/// Resolve one typed payload reference by resource kind and optional label.
pub(crate) fn with_payload<T: Send + Sync + 'static, R>(
    context: &BindingCallContext,
    handle: ResourceId,
    kind: ResourceKind,
    label: Option<&str>,
    read: impl FnOnce(&T, &ResourceEntry) -> R,
) -> Option<R> {
    with_entry(context, handle, kind, label, |entry| {
        let payload = entry.payload_ref::<T>()?;
        Some(read(payload, entry))
    })
    .flatten()
}

/// Resolve one typed payload and read one derived value, or return one unknown-handle error.
#[cfg(windows)]
pub(crate) fn require_payload_with<T: Send + Sync + 'static, R>(
    context: &BindingCallContext,
    handle: ResourceId,
    kind: ResourceKind,
    label: Option<&str>,
    field: &'static str,
    handle_kind: &'static str,
    read: impl FnOnce(&T, &ResourceEntry) -> R,
) -> RuntimeResult<R> {
    with_payload(context, handle, kind, label, read)
        .ok_or_else(|| core_platform::unknown_handle_with_id(field, handle_kind, handle.0))
}

/// Resolve one typed payload by kind and label, or return one unknown-handle error.
pub(crate) fn require_payload<T: Clone + Send + Sync + 'static>(
    context: &BindingCallContext,
    handle: ResourceId,
    kind: ResourceKind,
    label: Option<&str>,
    field: &'static str,
    handle_kind: &'static str,
) -> RuntimeResult<T> {
    resolve_payload::<T>(context, handle, kind, label)
        .ok_or_else(|| core_platform::unknown_handle_with_id(field, handle_kind, handle.0))
}

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
        .agent()
        .resources
        .with_entry(handle, |entry| {
            if entry.kind != kind {
                return None;
            }
            extract(entry)
        })
        .flatten()
        .ok_or_else(|| core_platform::unknown_handle("handle", label))?;

    // return the extracted payload
    Ok(resource)
}

/// Resolve a resource entry for one typed resource handle.
#[allow(dead_code)]
pub(crate) fn require_handle<H: ResourceHandle, T>(
    context: &BindingCallContext,
    handle: H,
    label: &str,
    extract: impl FnOnce(&ResourceEntry) -> Option<T>,
) -> RuntimeResult<T> {
    require_resource(context, handle.resource_id(), H::KIND, label, extract)
}
