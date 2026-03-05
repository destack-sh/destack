use crate::runtime::BindingCallContext;

use super::{ResourceId, ResourceKind};

/// Resolve one typed payload by resource kind and optional label.
pub(crate) fn resolve_payload<T: Clone + Send + Sync + 'static>(
    binding: &BindingCallContext,
    handle: ResourceId,
    kind: ResourceKind,
    label: Option<&str>,
) -> Option<T> {
    binding
        .agent()
        .resources
        .with_entry(handle, |entry| {
            if entry.kind != kind {
                return None;
            }

            if let Some(expected_label) = label
                && entry.label.as_deref() != Some(expected_label)
            {
                return None;
            }

            entry.payload_cloned::<T>()
        })
        .flatten()
}
