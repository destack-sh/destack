#![allow(unsafe_op_in_unsafe_fn)]

use super::core::*;
use super::metadata::*;
use super::runtime::*;

/// One opened macOS bluetooth adapter-watch resource.
struct MacBluetoothAdapterWatchResource {
    /// The queued adapter events.
    event_queue: Arc<BoundedQueue<BluetoothAdapterEventValue>>,
}

/// Finalizer for one macOS bluetooth adapter watch.
struct MacBluetoothAdapterWatchFinalizer {
    /// The owning central manager.
    _central: BluetoothDispatchBound<CBCentralManager>,
    /// The shared event queue.
    event_queue: Arc<BoundedQueue<BluetoothAdapterEventValue>>,
}

impl ResourceFinalizer for MacBluetoothAdapterWatchFinalizer {
    /// Wake blocked adapter-watch consumers on teardown.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        self.event_queue.close();
    }
}

/// One adapter-watch delegate ivar set.
struct MacBluetoothAdapterWatchDelegateState {
    /// Whether the adapter supports extended advertising.
    extended_advertising: Option<bool>,
    /// The shared adapter event queue.
    event_queue: Arc<BoundedQueue<BluetoothAdapterEventValue>>,
    /// The shared adapter event state.
    event_state: Arc<Mutex<BluetoothEventState>>,
    /// The last emitted adapter descriptor.
    previous_descriptor: Arc<Mutex<Option<BluetoothAdapterDescriptorValue>>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = MacBluetoothAdapterWatchDelegateState]
    #[name = "DestackBluetoothAdapterWatchDelegate"]
    struct MacBluetoothAdapterWatchDelegate;

    unsafe impl NSObjectProtocol for MacBluetoothAdapterWatchDelegate {}

    unsafe impl CBCentralManagerDelegate for MacBluetoothAdapterWatchDelegate {
        #[unsafe(method(centralManagerDidUpdateState:))]
        fn central_manager_did_update_state(&self, central: &CBCentralManager) {
            let state = self.ivars();
            let descriptor = adapter_descriptor_for_state(
                unsafe { central.state() },
                state.extended_advertising,
            );
            let mut previous_descriptor = state.previous_descriptor.lock();

            // first publication
            let event = match previous_descriptor.as_ref() {
                None => Some(adapter_attached_event(
                    &state.event_state,
                    descriptor.clone(),
                )),
                Some(previous) if previous != &descriptor => Some(adapter_changed_event(
                    &state.event_state,
                    descriptor.clone(),
                )),
                Some(_) => None,
            };
            *previous_descriptor = Some(descriptor);

            if let Some(event) = event {
                state.event_queue.push_drop_oldest(event);
            }
        }
    }
);

impl MacBluetoothAdapterWatchDelegate {
    /// Create one adapter-watch delegate.
    fn new(
        extended_advertising: Option<bool>,
        event_queue: Arc<BoundedQueue<BluetoothAdapterEventValue>>,
        event_state: Arc<Mutex<BluetoothEventState>>,
        previous_descriptor: Arc<Mutex<Option<BluetoothAdapterDescriptorValue>>>,
    ) -> Retained<Self> {
        let value = Self::alloc().set_ivars(MacBluetoothAdapterWatchDelegateState {
            extended_advertising,
            event_queue,
            event_state,
            previous_descriptor,
        });

        unsafe { objc2::msg_send![super(value), init] }
    }

    /// Return the delegate as one protocol object.
    fn as_protocol(&self) -> &ProtocolObject<dyn CBCentralManagerDelegate> {
        ProtocolObject::from_ref(self)
    }
}

/// Build one adapter descriptor snapshot for one CoreBluetooth manager state.
fn adapter_descriptor_for_state(
    state: CBManagerState,
    extended_advertising: Option<bool>,
) -> BluetoothAdapterDescriptorValue {
    adapter_descriptor(
        state == CBManagerState::PoweredOn,
        None,
        extended_advertising,
    )
}

/// Resolve one typed macOS bluetooth adapter-watch resource from the resource table.
fn adapter_watch_resource(
    binding: &BindingCallContext,
    handle: resource::BluetoothAdapterWatchHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<MacBluetoothAdapterWatchResource>> {
    bluetooth_payload(
        binding,
        handle.0,
        ResourceKind::BluetoothAdapterWatch,
        operation,
        "adapter watch",
    )
}

/// Open one macOS bluetooth adapter watch stream.
pub(crate) unsafe fn destack_device_bluetooth_adapter_watch_open(
    binding: &BindingCallContext,
    out: *mut resource::BluetoothAdapterWatchHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;
    require_bluetooth_authorization("destack.device.bluetooth.adapterWatchOpen")?;

    // create one central manager and adapter-state delegate
    let event_queue = Arc::new(BoundedQueue::new(16));
    let event_state = Arc::new(Mutex::new(BluetoothEventState { next_sequence: 1 }));
    let previous_descriptor = Arc::new(Mutex::new(None));
    let extended_advertising = Some(unsafe {
        CBCentralManager::supportsFeatures(CBCentralManagerFeature::ExtendedScanAndConnect)
    });
    let delegate = MacBluetoothAdapterWatchDelegate::new(
        extended_advertising,
        event_queue.clone(),
        event_state.clone(),
        previous_descriptor.clone(),
    );
    let central = central_manager_with_delegate(delegate.as_protocol());

    // seed the current adapter state when already known
    let initial_state = central.dispatch(|central| unsafe { central.state() });
    if initial_state != CBManagerState::Unknown {
        let descriptor = adapter_descriptor_for_state(initial_state, extended_advertising);
        *previous_descriptor.lock() = Some(descriptor.clone());
        event_queue.push_drop_oldest(adapter_attached_event(&event_state, descriptor));
    }

    // store the opened adapter watch
    let entry = ResourceEntry::new(ResourceKind::BluetoothAdapterWatch)
        .with_label(BLUETOOTH_ADAPTER_WATCH_RESOURCE_LABEL)
        .with_payload(Arc::new(MacBluetoothAdapterWatchResource {
            event_queue: event_queue.clone(),
        }))
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            MacBluetoothAdapterWatchFinalizer {
                _central: central,
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

/// Close one macOS bluetooth adapter watch stream.
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

/// Read one blocking macOS bluetooth adapter event.
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

/// Poll one macOS bluetooth adapter event without blocking.
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
