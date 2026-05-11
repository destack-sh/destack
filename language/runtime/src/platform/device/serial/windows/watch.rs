use std::sync::Arc;

use parking_lot::Mutex;

use super::core::*;
use super::identity::serial_descriptor_snapshot;
use super::service::{WindowsSerialService, WindowsSerialWatchRecord, WindowsSerialWatchResource};
use super::state::serial_descriptor_from_info;
use crate::diagnostic::RuntimeResult;
use crate::platform::resource::{ResourceEntry, ResourceFinalizer, ResourceKind};
use crate::platform::{core as core_platform, resource};
use crate::runtime::control::queue::BoundedQueue;

use super::super::core::{
    SERIAL_WATCH_RESOURCE_LABEL, SerialWatchEventState, attached_watch_event,
    close_serial_watch_resource, detached_watch_event, overflow_watch_event, serial_watch_payload,
    take_watch_overflow_count,
};

/// Maximum queued serial topology events per opened watch stream.
const SERIAL_WATCH_QUEUE_CAPACITY: usize = 64;

/// Finalizer that unregisters one windows serial watch and closes its queue.
pub(super) struct WindowsSerialWatchFinalizer {
    /// Shared windows serial topology service.
    pub(super) service: Arc<WindowsSerialService>,
    /// Stable watch registration identifier.
    pub(super) registration_id: u64,
    /// Shared watch queue to close during teardown.
    pub(super) event_queue: Arc<BoundedQueue<WindowsSerialWatchRecord>>,
}

impl ResourceFinalizer for WindowsSerialWatchFinalizer {
    /// Unregister the watch and close its queue.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        self.service.unregister_watch(self.registration_id);
        self.event_queue.close();
    }
}

/// Resolve one typed windows serial-watch resource from the resource table.
fn watch_resource(
    binding: &BindingCallContext,
    handle: resource::SerialWatchHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<WindowsSerialWatchResource>> {
    serial_watch_payload(binding, handle, operation)
}

/// Build one binding-visible serial watch event from one queued windows watch record.
fn event_from_record(
    binding: &BindingCallContext,
    resource: &WindowsSerialWatchResource,
    record: WindowsSerialWatchRecord,
) -> SerialWatchEvent {
    match record {
        WindowsSerialWatchRecord::Instance(info) => attached_watch_event(
            binding,
            &resource.event_state,
            serial_descriptor_from_info(binding, &info),
        ),
        WindowsSerialWatchRecord::Detached(info) => detached_watch_event(
            binding,
            &resource.event_state,
            serial_descriptor_from_info(binding, &info),
        ),
    }
}

/// Open one windows serial topology watch stream.
pub(crate) unsafe fn destack_device_serial_watch_open(
    binding: &BindingCallContext,
    out: *mut resource::SerialWatchHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the shared topology service and capture the initial snapshot
    let service = binding
        .worker()
        .platform_state
        .device
        .windows_serial_service("destack.device.serial.watchOpen")?;
    let known_ports = serial_descriptor_snapshot("destack.device.serial.watchOpen")?;
    let event_queue = Arc::new(BoundedQueue::new(SERIAL_WATCH_QUEUE_CAPACITY));
    let resource = Arc::new(WindowsSerialWatchResource {
        event_queue: event_queue.clone(),
        event_state: Mutex::new(SerialWatchEventState {
            next_sequence: 1,
            reported_dropped_count: 0,
        }),
    });

    // seed the queue with the current topology snapshot
    for info in known_ports.values() {
        resource
            .event_queue
            .push_drop_oldest(WindowsSerialWatchRecord::Instance(info.clone()));
    }

    // register the watch before publishing the resource
    let registration_id = service.register_watch(&resource, known_ports)?;
    let entry = ResourceEntry::new(ResourceKind::SerialWatch)
        .with_label(SERIAL_WATCH_RESOURCE_LABEL)
        .with_payload(resource)
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            WindowsSerialWatchFinalizer {
                service,
                registration_id,
                event_queue,
            },
        ));
    let handle = binding
        .worker()
        .resources
        .insert(binding.world(), entry, Some(binding.engine()));

    unsafe {
        out.write(resource::SerialWatchHandle(handle));
    }

    Ok(())
}

/// Close one windows serial topology watch stream.
pub(crate) unsafe fn destack_device_serial_watch_close(
    binding: &BindingCallContext,
    handle: resource::SerialWatchHandle,
) -> RuntimeResult<()> {
    close_serial_watch_resource(binding, handle, "destack.device.serial.watchClose")
}

/// Read one blocking windows serial topology event.
pub(crate) unsafe fn destack_device_serial_watch_read(
    binding: &BindingCallContext,
    out: *mut SerialWatchEvent,
    handle: resource::SerialWatchHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // wait for one queued topology event
    let resource = watch_resource(binding, handle, "destack.device.serial.watchRead")?;
    if let Some(dropped_count) =
        take_watch_overflow_count(&resource.event_queue, &resource.event_state)
    {
        unsafe {
            out.write(overflow_watch_event(
                binding,
                &resource.event_state,
                dropped_count,
            ));
        }

        return Ok(());
    }

    let timeout = Duration::from_nanos(timeoutns);
    let record = resource.event_queue.pop_with_timeout_or_else(timeout, || {
        Err(core_platform::io_would_block(
            "destack.device.serial.watchRead",
            "serial watch event queue is empty",
        ))
    })?;

    unsafe {
        out.write(event_from_record(binding, &resource, record));
    }

    Ok(())
}

/// Poll one windows serial topology event without blocking.
pub(crate) unsafe fn destack_device_serial_watch_try_read(
    binding: &BindingCallContext,
    out: *mut SerialWatchEvent,
    handle: resource::SerialWatchHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // read one queued topology event when available
    let resource = watch_resource(binding, handle, "destack.device.serial.watchTryRead")?;
    if let Some(dropped_count) =
        take_watch_overflow_count(&resource.event_queue, &resource.event_state)
    {
        unsafe {
            out.write(overflow_watch_event(
                binding,
                &resource.event_state,
                dropped_count,
            ));
        }

        return Ok(());
    }

    let record = resource.event_queue.try_pop_or_else(|| {
        Err(core_platform::io_would_block(
            "destack.device.serial.watchTryRead",
            "serial watch event queue is empty",
        ))
    })?;

    unsafe {
        out.write(event_from_record(binding, &resource, record));
    }

    Ok(())
}
