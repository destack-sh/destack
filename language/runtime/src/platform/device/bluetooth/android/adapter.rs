use std::collections::BTreeMap;
use std::sync::{Arc, Weak};

use parking_lot::Mutex;

use super::codec::read_adapter_descriptors;
use super::core::*;
use crate::runtime::{WorkerCallbackControl, WorkerCallbackHandle};

/// One adapter-watch event queue capacity.
const BLUETOOTH_ADAPTER_EVENT_QUEUE_CAPACITY: usize = 64;

/// One synthetic Android adapter-watch poll interval.
const BLUETOOTH_ADAPTER_POLL_INTERVAL_NS: u64 = 500_000_000;

/// One opened Android bluetooth adapter-watch resource.
struct AndroidBluetoothAdapterWatchResource {
    /// The shared adapter-watch state.
    state: Arc<Mutex<AndroidBluetoothAdapterWatchState>>,
}

/// Mutable Android bluetooth adapter-watch state.
struct AndroidBluetoothAdapterWatchState {
    /// The queued adapter events.
    event_queue: Arc<BoundedQueue<BluetoothAdapterEventValue>>,
    /// The per-stream sequence state.
    event_state: Mutex<BluetoothEventState>,
    /// The last known adapter snapshot.
    known_adapters: BTreeMap<String, BluetoothAdapterDescriptorValue>,
    /// The active worker callback when registered.
    callback_handle: Option<WorkerCallbackHandle>,
    /// Whether the watch has begun teardown.
    is_closed: bool,
}

/// Finalizer for one Android bluetooth adapter watch.
struct AndroidBluetoothAdapterWatchFinalizer {
    /// The shared adapter-watch state.
    state: Arc<Mutex<AndroidBluetoothAdapterWatchState>>,
}

impl ResourceFinalizer for AndroidBluetoothAdapterWatchFinalizer {
    /// Mark the watch closed and wake blocked consumers.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        let mut state = self.state.lock();
        state.is_closed = true;
        state.event_queue.close();
    }
}

impl AndroidBluetoothAdapterWatchState {
    /// Queue topology events for the current adapter snapshot.
    fn refresh(&mut self, adapters: Vec<BluetoothAdapterDescriptorValue>) {
        let mut current_snapshot = BTreeMap::new();

        for descriptor in adapters {
            current_snapshot.insert(descriptor.id.clone(), descriptor);
        }

        // attached and changed
        for (adapter_id, descriptor) in &current_snapshot {
            match self.known_adapters.get(adapter_id) {
                None => {
                    self.event_queue.push_drop_oldest(adapter_attached_event(
                        &self.event_state,
                        descriptor.clone(),
                    ));
                }
                Some(previous) if previous != descriptor => {
                    self.event_queue.push_drop_oldest(adapter_changed_event(
                        &self.event_state,
                        descriptor.clone(),
                    ));
                }
                Some(_) => {}
            }
        }

        // detached
        for (adapter_id, previous) in &self.known_adapters {
            if current_snapshot.contains_key(adapter_id) {
                continue;
            }

            self.event_queue
                .push_drop_oldest(adapter_detached_event(&self.event_state, previous.clone()));
        }

        self.known_adapters = current_snapshot;
    }
}

/// Resolve one typed Android bluetooth adapter-watch resource from the resource table.
fn adapter_watch_resource(
    binding: &BindingCallContext,
    handle: resource::BluetoothAdapterWatchHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<AndroidBluetoothAdapterWatchResource>> {
    bluetooth_payload(
        binding,
        handle.0,
        ResourceKind::BluetoothAdapterWatch,
        operation,
        "adapter watch",
    )
}

/// Register one synthetic poll callback for the Android adapter watch.
fn register_adapter_watch_callback(
    binding: &BindingCallContext,
    state: &Arc<Mutex<AndroidBluetoothAdapterWatchState>>,
) -> RuntimeResult<WorkerCallbackHandle> {
    let state = Arc::downgrade(state);

    binding.schedule_worker_callback(
        BLUETOOTH_ADAPTER_POLL_INTERVAL_NS,
        Some(BLUETOOTH_ADAPTER_POLL_INTERVAL_NS),
        move |binding| poll_adapter_watch(binding, &state),
    )
}

/// Refresh one Android adapter watch from the current host snapshot.
fn poll_adapter_watch(
    binding: &BindingCallContext,
    state: &Weak<Mutex<AndroidBluetoothAdapterWatchState>>,
) -> RuntimeResult<WorkerCallbackControl> {
    let Some(state) = state.upgrade() else {
        return Ok(WorkerCallbackControl::Cancel);
    };
    let mut state = state.lock();
    if state.is_closed {
        return Ok(WorkerCallbackControl::Cancel);
    }

    // synthetic topology refresh
    let adapters = match read_adapter_descriptors(
        binding,
        "destack.device.bluetooth.adapterWatch.syntheticPoll",
    ) {
        Ok(adapters) => adapters,
        Err(_) => {
            state.is_closed = true;
            state.event_queue.close();
            return Ok(WorkerCallbackControl::Cancel);
        }
    };
    state.refresh(adapters);

    Ok(WorkerCallbackControl::Keep)
}

/// Open one Android bluetooth adapter watch stream.
pub(crate) unsafe fn destack_device_bluetooth_adapter_watch_open(
    binding: &BindingCallContext,
    out: *mut resource::BluetoothAdapterWatchHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // capture the initial topology snapshot before starting the synthetic watch
    let adapters = read_adapter_descriptors(binding, "destack.device.bluetooth.adapterWatchOpen")?;
    let event_queue = Arc::new(BoundedQueue::new(BLUETOOTH_ADAPTER_EVENT_QUEUE_CAPACITY));
    let state = Arc::new(Mutex::new(AndroidBluetoothAdapterWatchState {
        event_queue: event_queue.clone(),
        event_state: Mutex::new(BluetoothEventState { next_sequence: 1 }),
        known_adapters: BTreeMap::new(),
        callback_handle: None,
        is_closed: false,
    }));

    // seed the queue with the current topology snapshot
    state.lock().refresh(adapters);

    // register the synthetic poll callback on the owning runtime thread
    let callback_handle = register_adapter_watch_callback(binding, &state)?;
    state.lock().callback_handle = Some(callback_handle);

    // store the opened adapter watch
    let entry = ResourceEntry::new(ResourceKind::BluetoothAdapterWatch)
        .with_label(BLUETOOTH_ADAPTER_WATCH_RESOURCE_LABEL)
        .with_payload(Arc::new(AndroidBluetoothAdapterWatchResource {
            state: state.clone(),
        }))
        .with_finalizer(
            binding
                .worker()
                .platform_state
                .device
                .wrap_finalizer(AndroidBluetoothAdapterWatchFinalizer { state }),
        );
    let handle = binding
        .worker()
        .resources
        .insert(binding.world(), entry, Some(binding.engine()));

    unsafe {
        out.write(resource::BluetoothAdapterWatchHandle(handle));
    }

    Ok(())
}

/// Close one Android bluetooth adapter watch stream.
pub(crate) unsafe fn destack_device_bluetooth_adapter_watch_close(
    binding: &BindingCallContext,
    handle: resource::BluetoothAdapterWatchHandle,
) -> RuntimeResult<()> {
    if let Ok(resource) = adapter_watch_resource(
        binding,
        handle,
        "destack.device.bluetooth.adapterWatchClose",
    ) {
        let callback_handle = {
            let mut state = resource.state.lock();
            state.is_closed = true;
            state.event_queue.close();
            state.callback_handle.take()
        };

        if let Some(callback_handle) = callback_handle {
            binding.cancel_worker_callback(callback_handle)?;
        }
    }

    close_bluetooth_resource(
        binding,
        handle.0,
        ResourceKind::BluetoothAdapterWatch,
        "destack.device.bluetooth.adapterWatchClose",
        "adapter watch",
    )
}

/// Read one blocking Android bluetooth adapter event.
pub(crate) unsafe fn destack_device_bluetooth_adapter_watch_read(
    binding: &BindingCallContext,
    out: *mut BluetoothAdapterEvent,
    handle: resource::BluetoothAdapterWatchHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource =
        adapter_watch_resource(binding, handle, "destack.device.bluetooth.adapterWatchRead")?;
    let event_queue = resource.state.lock().event_queue.clone();
    let timeout = std::time::Duration::from_nanos(timeoutns.max(1));
    let value = event_queue.pop_with_timeout_or_else(timeout, || {
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

/// Poll one Android bluetooth adapter event without blocking.
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
    let event_queue = resource.state.lock().event_queue.clone();
    let value = event_queue.try_pop_or_else(|| {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Build one deterministic adapter descriptor fixture.
    fn test_adapter_descriptor(
        id: &str,
        name: &str,
        powered: bool,
    ) -> BluetoothAdapterDescriptorValue {
        BluetoothAdapterDescriptorValue {
            id: id.to_string(),
            name: name.to_string(),
            powered,
            discovering: Some(false),
            extended_advertising: Some(true),
        }
    }

    /// Refresh synthetic Android adapter watches without emitting unchanged snapshots.
    #[test]
    fn test_refresh_ignores_unchanged_adapter_snapshots() {
        let event_queue = Arc::new(BoundedQueue::new(BLUETOOTH_ADAPTER_EVENT_QUEUE_CAPACITY));
        let mut state = AndroidBluetoothAdapterWatchState {
            event_queue: event_queue.clone(),
            event_state: Mutex::new(BluetoothEventState { next_sequence: 1 }),
            known_adapters: BTreeMap::new(),
            callback_handle: None,
            is_closed: false,
        };
        let adapter = test_adapter_descriptor("adapter.1", "Adapter", true);

        // seed one initial adapter snapshot
        state.refresh(vec![adapter.clone()]);
        let first = event_queue
            .try_pop_or_else(|| Err(core_platform::io_would_block("test", "queue empty")))
            .expect("first adapter seed event should be queued");
        let BluetoothAdapterEventValue::BluetoothAdapterInstanceEvent(first) = first else {
            panic!("expected attached adapter event");
        };
        assert_eq!(first.metadata.adapter, adapter);

        // unchanged snapshots should not emit new events
        state.refresh(vec![adapter.clone()]);
        let empty = event_queue
            .try_pop_or_else(|| Err(core_platform::io_would_block("test", "queue empty")));
        assert!(empty.is_err());

        // changed snapshots should emit one changed event
        let changed = test_adapter_descriptor("adapter.1", "Adapter", false);
        state.refresh(vec![changed.clone()]);
        let changed_event = event_queue
            .try_pop_or_else(|| Err(core_platform::io_would_block("test", "queue empty")))
            .expect("changed adapter event should be queued");
        let BluetoothAdapterEventValue::BluetoothAdapterChangedEvent(changed_event) = changed_event
        else {
            panic!("expected changed adapter event");
        };
        assert_eq!(changed_event.metadata.adapter, changed);

        // removed snapshots should emit one detached event
        state.refresh(Vec::new());
        let detached = event_queue
            .try_pop_or_else(|| Err(core_platform::io_would_block("test", "queue empty")))
            .expect("detached adapter event should be queued");
        let BluetoothAdapterEventValue::BluetoothAdapterDetachedEvent(detached) = detached else {
            panic!("expected detached adapter event");
        };
        assert_eq!(detached.metadata.adapter.id, "adapter.1");
    }
}
