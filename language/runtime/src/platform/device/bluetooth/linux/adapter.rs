use super::core::*;
use super::metadata::adapter_snapshot;

/// Resolve one typed Linux bluetooth adapter-watch resource from the resource table.
fn adapter_watch_resource(
    binding: &BindingCallContext,
    handle: resource::BluetoothAdapterWatchHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<LinuxBluetoothAdapterWatchResource>> {
    bluetooth_payload(
        binding,
        handle.0,
        ResourceKind::BluetoothAdapterWatch,
        operation,
        "adapter watch",
    )
}

/// Open one Linux bluetooth adapter watch stream.
pub(crate) unsafe fn destack_device_bluetooth_adapter_watch_open(
    binding: &BindingCallContext,
    out: *mut resource::BluetoothAdapterWatchHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // capture the initial adapter snapshot before registering the watcher
    let service = binding
        .worker()
        .platform_state
        .device
        .linux_bluetooth_service("destack.device.bluetooth.adapterWatchOpen")?;
    let connection = service.connection();
    let known_adapters =
        adapter_snapshot(&connection, "destack.device.bluetooth.adapterWatchOpen")?;
    let event_queue = Arc::new(BoundedQueue::new(BLUETOOTH_ADAPTER_EVENT_QUEUE_CAPACITY));
    let resource = Arc::new(LinuxBluetoothAdapterWatchResource {
        event_queue: event_queue.clone(),
        event_state: Mutex::new(BluetoothEventState { next_sequence: 1 }),
    });

    // seed the queue with the current topology snapshot
    for descriptor in known_adapters.values() {
        resource
            .event_queue
            .push_drop_oldest(adapter_attached_event(
                &resource.event_state,
                descriptor.clone(),
            ));
    }

    // register the watch before publishing the resource
    let registration_id = service.register_adapter_watch(&resource, known_adapters);
    let entry = ResourceEntry::new(ResourceKind::BluetoothAdapterWatch)
        .with_label(BLUETOOTH_ADAPTER_WATCH_RESOURCE_LABEL)
        .with_payload(resource)
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            LinuxBluetoothAdapterWatchFinalizer {
                service,
                registration_id,
                event_queue,
            },
        ));
    let handle = binding
        .worker()
        .resources
        .insert(binding.world(), entry, Some(binding.engine()));

    out.write(resource::BluetoothAdapterWatchHandle(handle));

    Ok(())
}

/// Close one Linux bluetooth adapter watch stream.
pub(crate) unsafe fn destack_device_bluetooth_adapter_watch_close(
    binding: &BindingCallContext,
    handle: resource::BluetoothAdapterWatchHandle,
) -> RuntimeResult<()> {
    close_bluetooth_resource(
        binding,
        handle.0,
        ResourceKind::BluetoothAdapterWatch,
        "destack.device.bluetooth.adapterWatchClose",
        "adapter watch",
    )
}

/// Read one blocking Linux bluetooth adapter event.
pub(crate) unsafe fn destack_device_bluetooth_adapter_watch_read(
    binding: &BindingCallContext,
    out: *mut BluetoothAdapterEvent,
    handle: resource::BluetoothAdapterWatchHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource =
        adapter_watch_resource(binding, handle, "destack.device.bluetooth.adapterWatchRead")?;
    let timeout = std::time::Duration::from_nanos(timeoutns.max(1));
    let value = resource.event_queue.pop_with_timeout_or_else(timeout, || {
        Err(core_platform::io_would_block(
            "destack.device.bluetooth.adapterWatchRead",
            "bluetooth adapter watch event queue is empty",
        ))
    })?;

    out.write(stored_adapter_event(binding, value));

    Ok(())
}

/// Poll one Linux bluetooth adapter event without blocking.
pub(crate) unsafe fn destack_device_bluetooth_adapter_watch_try_read(
    binding: &BindingCallContext,
    out: *mut BluetoothAdapterEvent,
    handle: resource::BluetoothAdapterWatchHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = adapter_watch_resource(
        binding,
        handle,
        "destack.device.bluetooth.adapterWatchTryRead",
    )?;
    let value = resource.event_queue.try_pop_or_else(|| {
        Err(core_platform::io_would_block(
            "destack.device.bluetooth.adapterWatchTryRead",
            "bluetooth adapter watch event queue is empty",
        ))
    })?;

    out.write(stored_adapter_event(binding, value));

    Ok(())
}
