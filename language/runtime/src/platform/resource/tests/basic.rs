use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::{assert_platform_error_code, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::{
    ResourceEntry, ResourceFinalizer, ResourceId, ResourceKind, ResourceOwnership,
};

/// Finalizer that records how many times one resource is finalized.
struct CountingFinalizer {
    /// Shared finalize counter.
    hits: Arc<AtomicUsize>,
}

impl ResourceFinalizer for CountingFinalizer {
    /// Increment the finalize counter for one removed resource.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        self.hits.fetch_add(1, Ordering::SeqCst);
    }
}

#[cfg(any(unix, windows))]
#[test]
fn test_resource_kind_returns_kind_label() {
    with_harness_context(|mut context| {
        // insert one timer resource entry
        let entry = ResourceEntry::new(ResourceKind::Timer);
        let id = context
            .call_context
            .agent()
            .resources
            .insert(entry, Some(context.call_context.engine()));

        // resolve the kind label through the binding
        let value = context.destack_resource_kind(id)?;
        let label = context.resource_kind_label_from_value(value)?;
        assert_eq!(label, "timer");

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_resource_close_removes_entry_and_runs_finalizer() {
    with_harness_context(|mut context| {
        // insert one resource entry with one counting finalizer
        let hits = Arc::new(AtomicUsize::new(0));
        let finalizer = CountingFinalizer { hits: hits.clone() };
        let entry = ResourceEntry::new(ResourceKind::Pipe).with_finalizer(finalizer);
        let id = context
            .call_context
            .agent()
            .resources
            .insert(entry, Some(context.call_context.engine()));

        // close the resource entry through the binding
        context.destack_resource_close(id)?;

        // closing should remove the entry and invoke finalization
        assert!(!context.call_context.agent().resources.contains(id));
        assert_eq!(hits.load(Ordering::SeqCst), 1);

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_resource_remove_removes_entry_and_runs_finalizer() {
    with_harness_context(|mut context| {
        // insert one resource entry with one counting finalizer
        let hits = Arc::new(AtomicUsize::new(0));
        let finalizer = CountingFinalizer { hits: hits.clone() };
        let entry = ResourceEntry::new(ResourceKind::Socket).with_finalizer(finalizer);
        let id = context
            .call_context
            .agent()
            .resources
            .insert(entry, Some(context.call_context.engine()));

        // remove the resource entry through the binding
        context.destack_resource_remove(id)?;

        // removal should delete the entry and invoke finalization
        assert!(!context.call_context.agent().resources.contains(id));
        assert_eq!(hits.load(Ordering::SeqCst), 1);

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_resource_transfer_keeps_existing_entry() {
    with_harness_context(|mut context| {
        // insert one resource entry that can be transferred
        let entry = ResourceEntry::new(ResourceKind::File);
        let id = context
            .call_context
            .agent()
            .resources
            .insert(entry, Some(context.call_context.engine()));

        // transfer ownership for the existing entry
        context.destack_resource_transfer(id, ResourceOwnership::Owned)?;

        // transfer should keep the entry in the table
        assert!(context.call_context.agent().resources.contains(id));

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_resource_close_returns_not_found_for_unknown_id() {
    with_harness_context(|mut context| {
        let result = context.destack_resource_close(ResourceId(0));
        assert_platform_error_code(result, PlatformErrorCode::IoNotFound)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_resource_kind_returns_not_found_for_unknown_id() {
    with_harness_context(|mut context| {
        let result = context.destack_resource_kind(ResourceId(0));
        assert_platform_error_code(result, PlatformErrorCode::IoNotFound)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_resource_remove_returns_not_found_for_unknown_id() {
    with_harness_context(|mut context| {
        let result = context.destack_resource_remove(ResourceId(0));
        assert_platform_error_code(result, PlatformErrorCode::IoNotFound)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_resource_transfer_returns_not_found_for_unknown_id() {
    with_harness_context(|mut context| {
        let result = context.destack_resource_transfer(ResourceId(0), ResourceOwnership::Borrowed);
        assert_platform_error_code(result, PlatformErrorCode::IoNotFound)?;

        Ok(())
    });
}
