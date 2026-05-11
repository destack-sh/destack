use std::collections::BTreeMap;
use std::sync::Arc;

use parking_lot::Mutex;
use windows::Devices::Enumeration::{DeviceInformation, DeviceInformationUpdate, DeviceWatcher};
use windows::Foundation::TypedEventHandler;
use windows::core::IInspectable;

use super::core::*;
use super::metadata::{enumerate_adapters, windows_bluetooth_error};

/// One adapter-watch event queue capacity.
const BLUETOOTH_ADAPTER_EVENT_QUEUE_CAPACITY: usize = 64;

/// One opened Windows bluetooth adapter-watch resource.
struct WindowsBluetoothAdapterWatchResource {
    /// The queued adapter events.
    event_queue: Arc<BoundedQueue<BluetoothAdapterEventValue>>,
}

/// Finalizer for one Windows bluetooth adapter watch.
struct WindowsBluetoothAdapterWatchFinalizer {
    /// The active device watcher.
    watcher: DeviceWatcher,
    /// The added callback token.
    added_token: i64,
    /// The updated callback token.
    updated_token: i64,
    /// The removed callback token.
    removed_token: i64,
    /// The enumeration-completed callback token.
    enumeration_completed_token: i64,
    /// The stopped callback token.
    stopped_token: i64,
    /// The shared adapter event queue.
    event_queue: Arc<BoundedQueue<BluetoothAdapterEventValue>>,
}

impl ResourceFinalizer for WindowsBluetoothAdapterWatchFinalizer {
    /// Stop the watcher and wake blocked consumers.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        let _ = self.watcher.RemoveAdded(self.added_token);
        let _ = self.watcher.RemoveUpdated(self.updated_token);
        let _ = self.watcher.RemoveRemoved(self.removed_token);
        let _ = self
            .watcher
            .RemoveEnumerationCompleted(self.enumeration_completed_token);
        let _ = self.watcher.RemoveStopped(self.stopped_token);
        let _ = self.watcher.Stop();
        self.event_queue.close();
    }
}

/// Resolve one typed Windows bluetooth adapter-watch resource from the resource table.
fn adapter_watch_resource(
    binding: &BindingCallContext,
    handle: resource::BluetoothAdapterWatchHandle,
    operation: &'static str,
) -> DiagnosticResult<Arc<WindowsBluetoothAdapterWatchResource>> {
    bluetooth_payload(
        binding,
        handle.0,
        ResourceKind::BluetoothAdapterWatch,
        operation,
        "adapter watch",
    )
}

/// Load the current Windows adapter snapshot keyed by stable identifier.
fn adapter_snapshot(
    operation: &'static str,
) -> DiagnosticResult<BTreeMap<String, BluetoothAdapterDescriptorValue>> {
    let adapters = enumerate_adapters(operation)?;
    let mut snapshot = BTreeMap::new();

    for (descriptor, _adapter) in adapters {
        snapshot.insert(descriptor.id.clone(), descriptor);
    }

    Ok(snapshot)
}

/// Diff the current adapter snapshot and queue topology events.
fn refresh_adapter_events(
    event_queue: &Arc<BoundedQueue<BluetoothAdapterEventValue>>,
    event_state: &Arc<Mutex<BluetoothEventState>>,
    known_adapters: &Arc<Mutex<BTreeMap<String, BluetoothAdapterDescriptorValue>>>,
    operation: &'static str,
) {
    let Ok(current_snapshot) = adapter_snapshot(operation) else {
        return;
    };
    let mut known_adapters = known_adapters.lock();

    // attached and changed
    for (adapter_id, descriptor) in &current_snapshot {
        match known_adapters.get(adapter_id) {
            None => {
                event_queue
                    .push_drop_oldest(adapter_attached_event(event_state, descriptor.clone()));
            }
            Some(previous) if previous != descriptor => {
                event_queue
                    .push_drop_oldest(adapter_changed_event(event_state, descriptor.clone()));
            }
            Some(_) => {}
        }
    }

    // detached
    for (adapter_id, previous) in known_adapters.iter() {
        if current_snapshot.contains_key(adapter_id) {
            continue;
        }

        event_queue.push_drop_oldest(adapter_detached_event(event_state, previous.clone()));
    }

    *known_adapters = current_snapshot;
}

/// Build one watcher callback that refreshes the adapter snapshot.
fn watcher_refresh_handler<T: windows::core::RuntimeType + 'static>(
    event_queue: Arc<BoundedQueue<BluetoothAdapterEventValue>>,
    event_state: Arc<Mutex<BluetoothEventState>>,
    known_adapters: Arc<Mutex<BTreeMap<String, BluetoothAdapterDescriptorValue>>>,
    operation: &'static str,
) -> TypedEventHandler<DeviceWatcher, T> {
    TypedEventHandler::new(move |_watcher, _args| {
        refresh_adapter_events(&event_queue, &event_state, &known_adapters, operation);
        Ok(())
    })
}

/// Open one Windows bluetooth adapter watch stream.
pub(crate) unsafe fn destack_device_bluetooth_adapter_watch_open(
    binding: &BindingCallContext,
    out: *mut resource::BluetoothAdapterWatchHandle,
) -> DiagnosticResult<()> {
    core_platform::ensure_out(out, "out")?;

    // load the initial topology snapshot before subscribing
    let selector = BluetoothAdapter::GetDeviceSelector().map_err(|error| {
        windows_bluetooth_error(
            "destack.device.bluetooth.adapterWatchOpen",
            "BluetoothAdapter::GetDeviceSelector",
            &error,
        )
    })?;
    let known_adapters = Arc::new(Mutex::new(adapter_snapshot(
        "destack.device.bluetooth.adapterWatchOpen",
    )?));
    let event_queue = Arc::new(BoundedQueue::new(BLUETOOTH_ADAPTER_EVENT_QUEUE_CAPACITY));
    let event_state = Arc::new(Mutex::new(BluetoothEventState { next_sequence: 1 }));

    // seed the queue with the current topology snapshot
    for descriptor in known_adapters.lock().values() {
        event_queue.push_drop_oldest(adapter_attached_event(&event_state, descriptor.clone()));
    }

    // create one WinRT watcher for the adapter device class
    let watcher = DeviceInformation::CreateWatcherAqsFilter(&selector).map_err(|error| {
        windows_bluetooth_error(
            "destack.device.bluetooth.adapterWatchOpen",
            "DeviceInformation::CreateWatcherAqsFilter",
            &error,
        )
    })?;
    let added_token = watcher
        .Added(&watcher_refresh_handler::<DeviceInformation>(
            event_queue.clone(),
            event_state.clone(),
            known_adapters.clone(),
            "destack.device.bluetooth.adapterWatchOpen",
        ))
        .map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.adapterWatchOpen",
                "DeviceWatcher::Added",
                &error,
            )
        })?;
    let updated_token = watcher
        .Updated(&watcher_refresh_handler::<DeviceInformationUpdate>(
            event_queue.clone(),
            event_state.clone(),
            known_adapters.clone(),
            "destack.device.bluetooth.adapterWatchOpen",
        ))
        .map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.adapterWatchOpen",
                "DeviceWatcher::Updated",
                &error,
            )
        })?;
    let removed_token = watcher
        .Removed(&watcher_refresh_handler::<DeviceInformationUpdate>(
            event_queue.clone(),
            event_state.clone(),
            known_adapters.clone(),
            "destack.device.bluetooth.adapterWatchOpen",
        ))
        .map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.adapterWatchOpen",
                "DeviceWatcher::Removed",
                &error,
            )
        })?;
    let enumeration_completed_token = watcher
        .EnumerationCompleted(&watcher_refresh_handler::<IInspectable>(
            event_queue.clone(),
            event_state.clone(),
            known_adapters.clone(),
            "destack.device.bluetooth.adapterWatchOpen",
        ))
        .map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.adapterWatchOpen",
                "DeviceWatcher::EnumerationCompleted",
                &error,
            )
        })?;
    let stopped_event_queue = event_queue.clone();
    let stopped_token = watcher
        .Stopped(&TypedEventHandler::new(move |_watcher, _args| {
            stopped_event_queue.close();
            Ok(())
        }))
        .map_err(|error| {
            windows_bluetooth_error(
                "destack.device.bluetooth.adapterWatchOpen",
                "DeviceWatcher::Stopped",
                &error,
            )
        })?;
    watcher.Start().map_err(|error| {
        windows_bluetooth_error(
            "destack.device.bluetooth.adapterWatchOpen",
            "DeviceWatcher::Start",
            &error,
        )
    })?;

    // store the opened adapter watch
    let entry = ResourceEntry::new(ResourceKind::BluetoothAdapterWatch)
        .with_label(BLUETOOTH_ADAPTER_WATCH_RESOURCE_LABEL)
        .with_payload(Arc::new(WindowsBluetoothAdapterWatchResource {
            event_queue: event_queue.clone(),
        }))
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            WindowsBluetoothAdapterWatchFinalizer {
                watcher,
                added_token,
                updated_token,
                removed_token,
                enumeration_completed_token,
                stopped_token,
                event_queue,
            },
        ));
    let handle = binding
        .worker()
        .resources
        .insert(binding.world(), entry, Some(binding.engine()));

    unsafe {
        out.write(resource::BluetoothAdapterWatchHandle(handle));
    }

    Ok(())
}

/// Close one Windows bluetooth adapter watch stream.
pub(crate) unsafe fn destack_device_bluetooth_adapter_watch_close(
    binding: &BindingCallContext,
    handle: resource::BluetoothAdapterWatchHandle,
) -> DiagnosticResult<()> {
    close_bluetooth_resource(
        binding,
        handle.0,
        ResourceKind::BluetoothAdapterWatch,
        "destack.device.bluetooth.adapterWatchClose",
        "adapter watch",
    )
}

/// Read one blocking Windows bluetooth adapter event.
pub(crate) unsafe fn destack_device_bluetooth_adapter_watch_read(
    binding: &BindingCallContext,
    out: *mut BluetoothAdapterEvent,
    handle: resource::BluetoothAdapterWatchHandle,
    timeoutns: u64,
) -> DiagnosticResult<()> {
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

    unsafe {
        out.write(stored_adapter_event(binding, value));
    }

    Ok(())
}

/// Poll one Windows bluetooth adapter event without blocking.
pub(crate) unsafe fn destack_device_bluetooth_adapter_watch_try_read(
    binding: &BindingCallContext,
    out: *mut BluetoothAdapterEvent,
    handle: resource::BluetoothAdapterWatchHandle,
) -> DiagnosticResult<()> {
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

    unsafe {
        out.write(stored_adapter_event(binding, value));
    }

    Ok(())
}
