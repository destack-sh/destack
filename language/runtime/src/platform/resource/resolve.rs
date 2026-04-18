use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::{BindingCallContext, binding_affinity_name};

use super::{ResourceAffinity, ResourceId, ResourceKind};

/// Return one affinity-violation error for one resource operation.
fn resource_affinity_violation(
    operation: &'static str,
    affinity: ResourceAffinity,
) -> Box<RuntimeError> {
    RuntimeError::AffinityViolation {
        name: operation.to_string(),
        affinity: binding_affinity_name(affinity.binding_affinity()).to_string(),
    }
    .boxed()
}

/// Ensure the current execution context satisfies one resource entry affinity.
pub(crate) fn ensure_resource_affinity(
    binding: &BindingCallContext,
    handle: ResourceId,
    operation: &'static str,
) -> RuntimeResult<()> {
    let affinity = binding
        .worker()
        .resources
        .with_entry(handle, |entry| entry.affinity)
        .flatten();

    // allow resources without explicit affinity requirements
    let Some(affinity) = affinity else {
        return Ok(());
    };

    // reject execution-context mismatches before exposing resource payloads
    if !affinity.satisfies(
        binding.execution_context(),
        binding.event_loop().execution_context_id(),
    ) {
        return Err(resource_affinity_violation(operation, affinity));
    }

    Ok(())
}

/// Resolve one typed payload by resource kind and optional label.
#[cfg_attr(not(any(target_os = "macos", windows)), allow(dead_code))]
pub(crate) fn resolve_payload<T: Clone + Send + Sync + 'static>(
    binding: &BindingCallContext,
    handle: ResourceId,
    kind: ResourceKind,
    label: Option<&str>,
    operation: &'static str,
) -> RuntimeResult<Option<T>> {
    // reject execution-context mismatches before exposing the entry payload
    ensure_resource_affinity(binding, handle, operation)?;

    // resolve the typed payload after validating kind and label
    Ok(binding
        .worker()
        .resources
        .with_entry(handle, |entry| {
            // reject mismatched resource kinds
            if entry.kind != kind {
                return None;
            }

            // reject mismatched resource labels
            if let Some(expected_label) = label
                && entry.label.as_deref() != Some(expected_label)
            {
                return None;
            }

            entry.payload_cloned::<T>()
        })
        .flatten())
}
