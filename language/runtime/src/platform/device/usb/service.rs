use super::core::*;
use super::descriptor::*;
use super::ffi::{LibusbApi, ffi, libusb_library_candidates, load_libraryusb_api};
use super::transfer::usb_hotplug_callback;
use crate::runtime::service::Service;
use crate::runtime::{ExecutionMode, ExecutionPolicy, WorkerLoop};

/// One polling interval used for fallback hotplug snapshot diffs.
///
/// libusb hotplug callbacks are not guaranteed everywhere, so the background service falls back
/// to one bounded snapshot poll when callback registration is unavailable.
const USB_HOTPLUG_POLL_INTERVAL: Duration = Duration::from_millis(250);

/// One libusb event-pump timeout.
///
/// the service waits in bounded chunks so shutdown and transfer cancellation stay responsive
const USB_EVENT_PUMP_TIMEOUT: Duration = Duration::from_millis(50);

/// One queued usb watch event kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UsbWatchEventKind {
    /// Device attached.
    Instance,
    /// Device detached.
    Detached,
}

/// One queued hotplug event record.
#[derive(Debug, Clone)]
pub(crate) struct UsbHotplugEventRecord {
    /// Event timestamp in nanoseconds.
    pub(crate) timestamp_ns: u64,
    /// Event kind.
    pub(crate) kind: UsbWatchEventKind,
    /// Device descriptor snapshot.
    pub(crate) descriptor: UsbDeviceDescriptorValue,
}

/// One opened usb watch resource.
pub(crate) struct UsbWatchResource {
    /// Shared event queue.
    pub(crate) queue: BoundedQueue<UsbHotplugEventRecord>,
    /// Mutable watch state.
    pub(crate) state: Mutex<UsbWatchState>,
}

/// One owned libusb device finalizer.
pub(crate) struct UsbDeviceFinalizer {
    /// Shared usb service state.
    pub(crate) service: Arc<UsbServiceState>,
    /// Opened libusb device handle.
    pub(crate) handle: *mut ffi::LibusbDeviceHandle,
    /// Referenced libusb device pointer.
    pub(crate) device: *mut ffi::LibusbDevice,
}

// libusb finalization is serialized by the resource table, and the raw handles remain valid
// until this finalizer runs
unsafe impl Send for UsbDeviceFinalizer {}

// libusb finalization is serialized by the resource table, and the raw handles remain valid
// until this finalizer runs
unsafe impl Sync for UsbDeviceFinalizer {}

impl ResourceFinalizer for UsbDeviceFinalizer {
    /// Close the opened device handle and release the retained device reference.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        unsafe {
            (self.service.api.libusb_close)(self.handle);
            (self.service.api.libusb_unref_device)(self.device);
        }
    }
}

/// One owned usb watch finalizer.
#[cfg(not(target_os = "android"))]
pub(crate) struct UsbWatchFinalizer {
    /// Shared usb service state.
    pub(crate) service: Arc<UsbServiceState>,
    /// Shared event queue.
    pub(crate) queue: Arc<UsbWatchResource>,
}

#[cfg(not(target_os = "android"))]
impl ResourceFinalizer for UsbWatchFinalizer {
    /// Close the queue when the watch resource drops.
    fn finalize(self: Box<Self>, resource_id: resource::ResourceId) {
        self.service.watch_registry.lock().unregister(resource_id);
        self.queue.queue.close();
        signal_usb_service_runtime(&self.service);
    }
}

/// One opened usb device resource.
pub(crate) struct UsbDeviceResource {
    /// Shared usb service state.
    pub(crate) service: Arc<UsbServiceState>,
    /// Opened libusb device handle.
    pub(crate) handle: *mut ffi::LibusbDeviceHandle,
    /// Referenced libusb device pointer.
    pub(crate) device: *mut ffi::LibusbDevice,
    /// Stable descriptor snapshot.
    pub(crate) descriptor: UsbDeviceDescriptorValue,
    /// In-flight endpoint transfers that can be cancelled from other threads.
    pub(crate) active_transfers: Mutex<BTreeMap<u8, Vec<NonNull<ffi::LibusbTransfer>>>>,
    /// Task serialization lock.
    pub(crate) operation_lock: Mutex<()>,
}

// libusb allows cross-thread device access, and this resource serializes synchronous operations
// through operation_lock
unsafe impl Send for UsbDeviceResource {}

// libusb allows cross-thread device access, and this resource serializes synchronous operations
// through operation_lock
unsafe impl Sync for UsbDeviceResource {}

/// One loaded process-global usb service.
pub(crate) struct UsbService {
    /// Loaded dynamic library lifetime owner.
    _library: DynamicLibrary,
    /// Shared service state used by resources and the pump thread.
    pub(super) state: Arc<UsbServiceState>,
    /// Lazily initialized runtime machinery for watches and transfer pumping.
    runtime: Mutex<UsbServiceRuntime>,
}

/// One owned libusb hotplug callback state pointer.
struct UsbHotplugCallbackState {
    /// Raw callback user-data allocation.
    pointer: *mut Arc<UsbServiceState>,
}

// the callback user-data allocation is created once, only read by libusb callbacks, and freed
// after callback deregistration during service drop
unsafe impl Send for UsbHotplugCallbackState {}

// the callback user-data allocation is created once, only read by libusb callbacks, and freed
// after callback deregistration during service drop
unsafe impl Sync for UsbHotplugCallbackState {}

/// Lazily initialized usb service runtime.
struct UsbServiceRuntime {
    /// Registered libusb hotplug callback handle when available.
    hotplug_callback_handle: Option<ffi::LibusbHotplugCallbackHandle>,
    /// Registered libusb hotplug callback user-data allocation.
    hotplug_callback_state: Option<UsbHotplugCallbackState>,
    /// Background ingress loop runtime.
    ingress_loop: Option<WorkerLoop>,
}

/// One process-global usb service state.
pub(crate) struct UsbServiceState {
    /// Loaded libusb function table.
    pub(super) api: LibusbApi,
    /// Initialized libusb context pointer.
    pub(super) context: NonNull<ffi::LibusbContext>,
    /// Registered weak usb watch subscribers.
    pub(super) watch_registry:
        Mutex<ProcessSubscriberRegistry<resource::ResourceId, UsbWatchResource>>,
    /// Previous hotplug polling snapshot.
    pub(super) watch_snapshot: Mutex<BTreeMap<String, UsbDeviceDescriptorValue>>,
    /// Number of in-flight asynchronous endpoint transfers.
    pub(super) active_transfer_count: AtomicUsize,
    /// Whether libusb hotplug callbacks are active for this service.
    pub(super) uses_hotplug_callbacks: AtomicBool,
    /// One-shot shutdown flag for the event-pump thread.
    pub(super) is_shutdown: AtomicBool,
    /// Wait state for the demand-driven service pump.
    pub(super) runtime_wait: Mutex<()>,
    /// Wake signal for the demand-driven service pump.
    pub(super) runtime_wake: Condvar,
}

// libusb contexts are process-global and explicitly designed for concurrent use with
// libusb_handle_events* running on a background thread
unsafe impl Send for UsbServiceState {}

// libusb contexts are process-global and explicitly designed for concurrent use with
// libusb_handle_events* running on a background thread
unsafe impl Sync for UsbServiceState {}

/// One submitted isochronous transfer waiter.
#[derive(Debug)]
pub(crate) struct UsbTransferWaiter {
    /// Completion state.
    state: Mutex<UsbTransferWaiterState>,
    /// Completion wake.
    wake: Condvar,
}

/// Mutable transfer wait state.
#[derive(Debug, Default)]
struct UsbTransferWaiterState {
    /// Whether the callback has fired.
    is_complete: bool,
}

/// One asynchronous endpoint transfer shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UsbAsyncTransferKind {
    /// Bulk endpoint transfer.
    Bulk,
    /// Interrupt endpoint transfer.
    Interrupt,
}

impl UsbTransferWaiter {
    /// Build one empty transfer waiter.
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(UsbTransferWaiterState::default()),
            wake: Condvar::new(),
        })
    }

    /// Mark the transfer complete and wake the waiter.
    pub(crate) fn complete(&self) {
        let mut state = self.state.lock();
        state.is_complete = true;
        self.wake.notify_all();
    }

    /// Wait until the transfer completes.
    pub(crate) fn wait(&self) {
        let mut state = self.state.lock();
        while !state.is_complete {
            self.wake.wait(&mut state);
        }
    }
}

/// Register one in-flight transfer on one endpoint.
pub(crate) fn register_active_transfer(
    resource: &UsbDeviceResource,
    endpoint_address: u8,
    transfer: NonNull<ffi::LibusbTransfer>,
) {
    let mut active_transfers = resource.active_transfers.lock();
    active_transfers
        .entry(endpoint_address)
        .or_default()
        .push(transfer);

    resource
        .service
        .active_transfer_count
        .fetch_add(1, Ordering::SeqCst);
    signal_usb_service_runtime(&resource.service);
}

/// Remove one completed transfer from the endpoint registry.
pub(crate) fn unregister_active_transfer(
    resource: &UsbDeviceResource,
    endpoint_address: u8,
    transfer: *mut ffi::LibusbTransfer,
) {
    let mut active_transfers = resource.active_transfers.lock();
    let Some(transfers) = active_transfers.get_mut(&endpoint_address) else {
        return;
    };

    transfers.retain(|registered| registered.as_ptr() != transfer);
    if transfers.is_empty() {
        active_transfers.remove(&endpoint_address);
    }

    resource
        .service
        .active_transfer_count
        .fetch_sub(1, Ordering::SeqCst);
    signal_usb_service_runtime(&resource.service);
}

/// Cancel all in-flight transfers on one endpoint.
pub(crate) fn cancel_active_endpoint_transfers(
    resource: &UsbDeviceResource,
    endpoint_address: u8,
    operation: &'static str,
) -> RuntimeResult<()> {
    let active_transfers = resource.active_transfers.lock();
    let Some(transfers) = active_transfers.get(&endpoint_address) else {
        return Ok(());
    };

    for transfer in transfers {
        let status = unsafe { (resource.service.api.libusb_cancel_transfer)(transfer.as_ptr()) };
        if status == ffi::LIBUSB_SUCCESS || status == ffi::LIBUSB_ERROR_NOT_FOUND {
            continue;
        }

        return Err(libusb_error(operation, "libusb_cancel_transfer", status));
    }

    Ok(())
}

/// Cancel all in-flight endpoint transfers on one device.
pub(crate) fn cancel_all_active_transfers(
    resource: &UsbDeviceResource,
    operation: &'static str,
) -> RuntimeResult<()> {
    let active_transfers = resource.active_transfers.lock();

    for transfers in active_transfers.values() {
        for transfer in transfers {
            let status =
                unsafe { (resource.service.api.libusb_cancel_transfer)(transfer.as_ptr()) };
            if status == ffi::LIBUSB_SUCCESS || status == ffi::LIBUSB_ERROR_NOT_FOUND {
                continue;
            }

            return Err(libusb_error(operation, "libusb_cancel_transfer", status));
        }
    }

    Ok(())
}

impl UsbService {
    /// Create one process-global usb service.
    fn new() -> RuntimeResult<Self> {
        // load one usable libusb implementation
        let (library, api) =
            load_library_with_api(libusb_library_candidates(), load_libraryusb_api).map_err(
                |error| core_platform::not_supported(format!("destack.device.usb: {error}")),
            )?;

        // initialize the shared libusb context
        let mut context = ptr::null_mut();
        let status = unsafe { (api.libusb_init)(&mut context) };
        if status != ffi::LIBUSB_SUCCESS {
            return Err(libusb_error("destack.device.usb", "libusb_init", status));
        }

        let context = NonNull::new(context).ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_data(
                "destack.device.usb: libusb_init returned one null context",
            ))
            .boxed()
        })?;

        // seed one deterministic initial snapshot for hotplug diffing
        let state = Arc::new(UsbServiceState {
            api,
            context,
            watch_registry: Mutex::new(ProcessSubscriberRegistry::default()),
            watch_snapshot: Mutex::new(BTreeMap::new()),
            active_transfer_count: AtomicUsize::new(0),
            uses_hotplug_callbacks: AtomicBool::new(false),
            is_shutdown: AtomicBool::new(false),
            runtime_wait: Mutex::new(()),
            runtime_wake: Condvar::new(),
        });
        #[cfg(target_os = "android")]
        let initial_snapshot = BTreeMap::new();
        #[cfg(not(target_os = "android"))]
        let initial_snapshot = enumerate_usb_device_map(&state, "destack.device.usb.list")?;
        *state.watch_snapshot.lock() = initial_snapshot;

        Ok(Self {
            _library: library,
            state,
            runtime: Mutex::new(UsbServiceRuntime {
                hotplug_callback_handle: None,
                hotplug_callback_state: None,
                ingress_loop: None,
            }),
        })
    }
}

impl Service for UsbService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Loop);
}

impl Drop for UsbService {
    /// Stop the background pump and tear down libusb.
    fn drop(&mut self) {
        self.state.is_shutdown.store(true, Ordering::SeqCst);
        signal_usb_service_runtime(&self.state);

        // extract the lazy runtime so shutdown does not hold the mutex while joining
        let (ingress_loop, hotplug_callback_handle, hotplug_callback_state) = {
            let mut runtime = self.runtime.lock();

            (
                runtime.ingress_loop.take(),
                runtime.hotplug_callback_handle.take(),
                runtime.hotplug_callback_state.take(),
            )
        };

        drop(ingress_loop);

        if let Some(callback_handle) = hotplug_callback_handle {
            unsafe {
                (self.state.api.libusb_hotplug_deregister_callback)(
                    self.state.context.as_ptr(),
                    callback_handle,
                );
            }
        }

        if let Some(callback_state) = hotplug_callback_state {
            unsafe {
                drop(Box::from_raw(callback_state.pointer));
            }
        }

        unsafe {
            (self.state.api.libusb_exit)(self.state.context.as_ptr());
        }
    }
}

/// Resolve the shared USB service or return a not-supported error.
pub(crate) fn usb_service(operation: &'static str) -> RuntimeResult<Arc<UsbService>> {
    UsbService::global(UsbService::new).map_err(|error| {
        let platform_code = error.platform_error().map(|error| error.code);

        RuntimeError::from(PlatformError::io_with(
            platform_code,
            None,
            None,
            Some(operation.to_string()),
            Some(String::from("usbService")),
            format!("usb service unavailable: {error}"),
        ))
        .boxed()
    })
}

/// Start the usb service runtime when one operation needs callbacks or transfer pumping.
pub(crate) fn ensure_usb_service_runtime(service: &Arc<UsbService>) -> RuntimeResult<()> {
    let mut runtime = service.runtime.lock();

    // reuse the initialized runtime for later opens and watches
    if runtime.ingress_loop.is_some() {
        return Ok(());
    }

    // register hotplug callbacks before the event loop starts
    let (hotplug_callback_handle, hotplug_callback_state) =
        try_register_usb_hotplug_callback(&service.state);
    if hotplug_callback_handle.is_some() {
        service
            .state
            .uses_hotplug_callbacks
            .store(true, Ordering::SeqCst);
    }

    // start the shared libusb event loop for transfers and hotplug callbacks
    let thread_state = service.state.clone();
    let ingress_loop = match WorkerLoop::open(
        "destack-usb",
        "platform.service.spawn",
        UsbService::POLICY,
        move || {
            let shutdown_state = thread_state.clone();
            let run = Box::new(move || {
                run_usb_service_loop(thread_state);
                Ok(())
            });
            let shutdown = Box::new(move || {
                shutdown_state.is_shutdown.store(true, Ordering::SeqCst);
                signal_usb_service_runtime(&shutdown_state);
            });

            Ok((shutdown, run))
        },
    ) {
        Ok(loop_runtime) => loop_runtime,
        Err(error) => {
            service
                .state
                .uses_hotplug_callbacks
                .store(false, Ordering::SeqCst);

            if let Some(callback_handle) = hotplug_callback_handle {
                unsafe {
                    (service.state.api.libusb_hotplug_deregister_callback)(
                        service.state.context.as_ptr(),
                        callback_handle,
                    );
                }
            }

            if let Some(callback_state) = hotplug_callback_state {
                unsafe {
                    drop(Box::from_raw(callback_state.pointer));
                }
            }

            return Err(RuntimeError::from(PlatformError::io_with(
                Some(PlatformErrorCode::Io),
                None,
                None,
                Some(String::from("destack.device.usb")),
                Some(String::from("UsbService::ingress_loop")),
                format!("failed to spawn usb service thread: {error}"),
            ))
            .boxed());
        }
    };

    runtime.hotplug_callback_handle = hotplug_callback_handle;
    runtime.hotplug_callback_state = hotplug_callback_state;
    runtime.ingress_loop = Some(ingress_loop);

    Ok(())
}

/// Register one native hotplug callback when libusb supports it.
fn try_register_usb_hotplug_callback(
    service: &Arc<UsbServiceState>,
) -> (
    Option<ffi::LibusbHotplugCallbackHandle>,
    Option<UsbHotplugCallbackState>,
) {
    let is_supported =
        unsafe { (service.api.libusb_has_capability)(ffi::LIBUSB_CAP_HAS_HOTPLUG) != 0 };
    if !is_supported {
        return (None, None);
    }

    let callback_state = Box::into_raw(Box::new(service.clone()));
    let mut callback_handle = 0;
    let status = unsafe {
        (service.api.libusb_hotplug_register_callback)(
            service.context.as_ptr(),
            ffi::LIBUSB_HOTPLUG_EVENT_DEVICE_ARRIVED | ffi::LIBUSB_HOTPLUG_EVENT_DEVICE_LEFT,
            ffi::LIBUSB_HOTPLUG_NO_FLAGS,
            ffi::LIBUSB_HOTPLUG_MATCH_ANY,
            ffi::LIBUSB_HOTPLUG_MATCH_ANY,
            ffi::LIBUSB_HOTPLUG_MATCH_ANY,
            Some(usb_hotplug_callback),
            callback_state.cast::<c_void>(),
            &mut callback_handle,
        )
    };
    if status != ffi::LIBUSB_SUCCESS {
        unsafe {
            drop(Box::from_raw(callback_state));
        }

        tracing::warn!(
            "usb hotplug callback registration failed, falling back to polling: {}",
            libusb_error_name_for_api(&service.api, status)
        );
        return (None, None);
    }

    (
        Some(callback_handle),
        Some(UsbHotplugCallbackState {
            pointer: callback_state,
        }),
    )
}

/// Run the background libusb event and hotplug pump.
fn run_usb_service_loop(service: Arc<UsbServiceState>) {
    let mut last_watch_poll = Instant::now();

    // pump libusb async work only while runtime demand exists
    while !service.is_shutdown.load(Ordering::SeqCst) {
        if !usb_service_has_runtime_demand(&service) {
            wait_for_usb_service_runtime_demand(&service);
            last_watch_poll = Instant::now();
            continue;
        }

        pump_usb_events(&service);

        if !service.uses_hotplug_callbacks.load(Ordering::SeqCst)
            && last_watch_poll.elapsed() >= USB_HOTPLUG_POLL_INTERVAL
        {
            if let Err(error) = poll_usb_hotplug_watchers(&service) {
                tracing::warn!("usb hotplug poll failed: {error}");
            }

            last_watch_poll = Instant::now();
        }
    }
}

/// Return whether the usb runtime currently has one live watch or transfer to service.
fn usb_service_has_runtime_demand(service: &UsbServiceState) -> bool {
    if service.active_transfer_count.load(Ordering::SeqCst) > 0 {
        return true;
    }

    !service.watch_registry.lock().is_empty()
}

/// Wake the usb runtime after one demand transition.
pub(crate) fn signal_usb_service_runtime(service: &UsbServiceState) {
    let _wait = service.runtime_wait.lock();
    service.runtime_wake.notify_all();
}

/// Wait until the usb runtime has one live watch or transfer to service.
fn wait_for_usb_service_runtime_demand(service: &UsbServiceState) {
    let mut wait = service.runtime_wait.lock();

    while !service.is_shutdown.load(Ordering::SeqCst) && !usb_service_has_runtime_demand(service) {
        service.runtime_wake.wait(&mut wait);
    }
}

/// Pump pending libusb async events once.
fn pump_usb_events(service: &UsbServiceState) {
    let timeout = ffi::LibusbTimeval {
        tv_sec: 0,
        tv_usec: i64::try_from(USB_EVENT_PUMP_TIMEOUT.as_micros()).unwrap_or(50_000),
    };
    let mut completed = 0;

    let status = unsafe {
        (service.api.libusb_handle_events_timeout_completed)(
            service.context.as_ptr(),
            &timeout,
            &mut completed,
        )
    };

    // libusb pump errors should be visible during service operation
    if status != ffi::LIBUSB_SUCCESS {
        tracing::warn!(
            "usb event pump failed: {}",
            libusb_error_name_for_api(&service.api, status)
        );
    }
}

/// Poll one hotplug snapshot and queue attach or detach events.
fn poll_usb_hotplug_watchers(service: &UsbServiceState) -> RuntimeResult<()> {
    // skip enumeration work when nobody is watching
    if service.watch_registry.lock().is_empty() {
        return Ok(());
    }

    let next_snapshot = enumerate_usb_device_map(service, "destack.device.usb.watchOpen")?;
    let mut previous_snapshot = service.watch_snapshot.lock();

    // enqueue attach events for newly visible devices
    for (id, descriptor) in &next_snapshot {
        if previous_snapshot.contains_key(id) {
            continue;
        }

        queue_usb_hotplug_event(
            service,
            UsbHotplugEventRecord {
                timestamp_ns: core_platform::monotonic_now_ns(),
                kind: UsbWatchEventKind::Instance,
                descriptor: descriptor.clone(),
            },
        );
    }

    // enqueue detach events for disappeared devices
    for (id, descriptor) in previous_snapshot.iter() {
        if next_snapshot.contains_key(id) {
            continue;
        }

        queue_usb_hotplug_event(
            service,
            UsbHotplugEventRecord {
                timestamp_ns: core_platform::monotonic_now_ns(),
                kind: UsbWatchEventKind::Detached,
                descriptor: descriptor.clone(),
            },
        );
    }

    *previous_snapshot = next_snapshot;

    Ok(())
}

/// Broadcast one hotplug record to all live watch streams.
pub(crate) fn queue_usb_hotplug_event(service: &UsbServiceState, event: UsbHotplugEventRecord) {
    let subscribers = service.watch_registry.lock().snapshot();
    for subscriber in subscribers {
        subscriber.queue.push_drop_oldest(event.clone());
    }
}

/// Resolve one usb device handle into one opened resource.
pub(crate) fn usb_device_resource(
    binding: &BindingCallContext,
    handle: resource::UsbDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<UsbDeviceResource>> {
    let resource = binding.worker().resources.with_entry(handle.0, |entry| {
        if entry.kind != ResourceKind::UsbDevice {
            return None;
        }

        entry.payload_cloned::<Arc<UsbDeviceResource>>()
    });

    resource
        .flatten()
        .ok_or_else(|| invalid_usb_handle(operation, "unknown usb device handle"))
}

/// Resolve one usb watch handle into one opened resource.
#[cfg(not(target_os = "android"))]
pub(crate) fn usb_watch_resource(
    binding: &BindingCallContext,
    handle: resource::UsbWatchHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<UsbWatchResource>> {
    let resource = binding.worker().resources.with_entry(handle.0, |entry| {
        if entry.kind != ResourceKind::UsbWatch {
            return None;
        }

        entry.payload_cloned::<Arc<UsbWatchResource>>()
    });

    resource
        .flatten()
        .ok_or_else(|| invalid_usb_handle(operation, "unknown usb watch handle"))
}

/// Build one invalid usb handle error.
pub(crate) fn invalid_usb_handle(
    operation: &'static str,
    message: &'static str,
) -> Box<RuntimeError> {
    core_platform::io_operation_error(
        operation,
        Some(PlatformErrorCode::InvalidArgumentValue),
        message,
    )
}

/// Build one libusb-mapped runtime error.
pub(crate) fn libusb_error(
    operation: &'static str,
    action: &'static str,
    code: c_int,
) -> Box<RuntimeError> {
    let platform_code = match code {
        ffi::LIBUSB_ERROR_INVALID_PARAM => Some(PlatformErrorCode::InvalidArgumentValue),
        ffi::LIBUSB_ERROR_ACCESS => Some(PlatformErrorCode::IoPermissionDenied),
        ffi::LIBUSB_ERROR_NO_DEVICE | ffi::LIBUSB_ERROR_NOT_FOUND => {
            Some(PlatformErrorCode::IoNotFound)
        }
        ffi::LIBUSB_ERROR_BUSY => Some(PlatformErrorCode::IoBusy),
        ffi::LIBUSB_ERROR_TIMEOUT => Some(PlatformErrorCode::IoWouldBlock),
        ffi::LIBUSB_ERROR_OVERFLOW => Some(PlatformErrorCode::IoInvalidData),
        ffi::LIBUSB_ERROR_INTERRUPTED => Some(PlatformErrorCode::IoInterrupted),
        ffi::LIBUSB_ERROR_NOT_SUPPORTED => Some(PlatformErrorCode::NotSupported),
        ffi::LIBUSB_ERROR_NO_MEM => Some(PlatformErrorCode::Io),
        _ => Some(PlatformErrorCode::Io),
    };

    RuntimeError::from(PlatformError::io_with(
        platform_code,
        None,
        Some(code),
        Some(operation.to_string()),
        Some(action.to_string()),
        format!("{action} failed: {}", libusb_error_name(code)),
    ))
    .boxed()
}

/// Return one textual libusb error name when available.
pub(crate) fn libusb_error_name(code: c_int) -> String {
    let Some(service) = UsbService::active() else {
        return fallback_libusb_error_name(code);
    };

    libusb_error_name_for_api(&service.state.api, code)
}

/// Return one textual libusb error name from one already-loaded api table.
pub(crate) fn libusb_error_name_for_api(api: &LibusbApi, code: c_int) -> String {
    let pointer = unsafe { (api.libusb_error_name)(code) };
    if pointer.is_null() {
        return fallback_libusb_error_name(code);
    }

    unsafe { CStr::from_ptr(pointer) }
        .to_string_lossy()
        .into_owned()
}

/// Return one fallback textual libusb error name without loading libusb.
fn fallback_libusb_error_name(code: c_int) -> String {
    let name = match code {
        ffi::LIBUSB_SUCCESS => "LIBUSB_SUCCESS",
        ffi::LIBUSB_ERROR_IO => "LIBUSB_ERROR_IO",
        ffi::LIBUSB_ERROR_INVALID_PARAM => "LIBUSB_ERROR_INVALID_PARAM",
        ffi::LIBUSB_ERROR_ACCESS => "LIBUSB_ERROR_ACCESS",
        ffi::LIBUSB_ERROR_NO_DEVICE => "LIBUSB_ERROR_NO_DEVICE",
        ffi::LIBUSB_ERROR_NOT_FOUND => "LIBUSB_ERROR_NOT_FOUND",
        ffi::LIBUSB_ERROR_BUSY => "LIBUSB_ERROR_BUSY",
        ffi::LIBUSB_ERROR_TIMEOUT => "LIBUSB_ERROR_TIMEOUT",
        ffi::LIBUSB_ERROR_OVERFLOW => "LIBUSB_ERROR_OVERFLOW",
        ffi::LIBUSB_ERROR_PIPE => "LIBUSB_ERROR_PIPE",
        ffi::LIBUSB_ERROR_INTERRUPTED => "LIBUSB_ERROR_INTERRUPTED",
        ffi::LIBUSB_ERROR_NO_MEM => "LIBUSB_ERROR_NO_MEM",
        ffi::LIBUSB_ERROR_NOT_SUPPORTED => "LIBUSB_ERROR_NOT_SUPPORTED",
        ffi::LIBUSB_ERROR_OTHER => "LIBUSB_ERROR_OTHER",
        _ => return format!("libusb error {code}"),
    };

    format!("{name} ({code})")
}

/// Return one transfer status from one synchronous libusb result code.
pub(crate) fn transfer_status_from_libusb_result(code: c_int) -> Option<UsbTransferStatus> {
    match code {
        ffi::LIBUSB_SUCCESS => Some(UsbTransferStatus::Ok),
        ffi::LIBUSB_ERROR_PIPE => Some(UsbTransferStatus::Stall),
        ffi::LIBUSB_ERROR_OVERFLOW => Some(UsbTransferStatus::Babble),
        _ => None,
    }
}
