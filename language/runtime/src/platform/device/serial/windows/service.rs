use std::collections::{BTreeMap, HashMap};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Weak};

use parking_lot::Mutex;

use super::core::*;
use super::identity::serial_descriptor_snapshot;
use super::io::{
    WindowsSerialOverlappedIo, WindowsSerialWaitSubmission, process_serial_event_mask,
    submit_serial_event_wait,
};
use super::state::{
    WindowsSerialDescriptorInfo, WindowsSerialEventRecord, WindowsSerialPortResource,
    is_disconnected_code, serial_io_error, serial_io_error_with_code,
};
use crate::platform::PlatformError;
use crate::platform::core::BoundedQueue;
use crate::runtime::service::Service;
use crate::runtime::service::windows::WindowsRegisteredWait;
use crate::runtime::{ExecutionMode, ExecutionPolicy};

use super::super::core::SerialWatchEventState;

/// Process-global Windows serial ingress service.
pub(crate) struct WindowsSerialService {
    /// The next registration identifier.
    next_registration_id: AtomicU64,
    /// Live port event runtimes keyed by registration identifier.
    runtimes: Mutex<HashMap<u64, Arc<WindowsSerialEventRuntime>>>,
    /// The next serial watch registration identifier.
    next_watch_registration_id: AtomicU64,
    /// Live serial watch runtimes keyed by registration identifier.
    watch_runtimes: Mutex<HashMap<u64, WindowsSerialWatchRegistration>>,
    /// Shared native serial watch subscription when at least one watch is active.
    watch_notifications: Mutex<Option<WindowsSerialWatchNotifications>>,
}

/// Shared wait state for one registered windows serial port.
struct WindowsSerialEventWaitState {
    /// The active overlapped wait state.
    overlapped_io: WindowsSerialOverlappedIo,
    /// The current event mask target for `WaitCommEvent`.
    event_mask: u32,
    /// The registered shared windows wait when one is active.
    wait: Option<WindowsRegisteredWait>,
}

/// One live Windows serial event runtime.
struct WindowsSerialEventRuntime {
    /// Shared serial resource state.
    resource: Arc<WindowsSerialPortResource>,
    /// Whether shutdown has started for this runtime.
    is_shutdown: AtomicBool,
    /// Mutable wait registration state.
    wait_state: Mutex<WindowsSerialEventWaitState>,
}

/// One queued windows serial topology event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum WindowsSerialWatchRecord {
    /// One attached serial endpoint.
    Instance(WindowsSerialDescriptorInfo),
    /// One detached serial endpoint.
    Detached(WindowsSerialDescriptorInfo),
}

/// One opened windows serial watch resource.
pub(super) struct WindowsSerialWatchResource {
    /// Queued topology events for this watch stream.
    pub(super) event_queue: Arc<BoundedQueue<WindowsSerialWatchRecord>>,
    /// Monotonic event sequencing for this watch stream.
    pub(super) event_state: Mutex<SerialWatchEventState>,
}

/// One registered windows serial topology watch.
struct WindowsSerialWatchRegistration {
    /// Weak handle to the opened watch resource.
    resource: Weak<WindowsSerialWatchResource>,
    /// Last published descriptor snapshot for this watch stream.
    known_ports: BTreeMap<String, WindowsSerialDescriptorInfo>,
}

/// One registered windows serial-notification subscription.
struct WindowsSerialWatchNotifications {
    /// The native configuration-manager notification handle.
    handle: HCMNOTIFICATION,
    /// The retained callback context for the native subscription.
    _context: Arc<WindowsSerialWatchNotificationContext>,
}

impl Drop for WindowsSerialWatchNotifications {
    /// Unregister the native notification subscription.
    fn drop(&mut self) {
        let status = unsafe { CM_Unregister_Notification(self.handle) };
        if status != CR_SUCCESS {
            tracing::warn!("windows serial notification teardown failed: {status}");
        }
    }
}

/// One callback context for native windows serial notifications.
struct WindowsSerialWatchNotificationContext {
    /// Weak handle back to the shared serial service.
    service: Weak<WindowsSerialService>,
}

impl WindowsSerialService {
    /// Create one empty Windows serial ingress service.
    fn new() -> RuntimeResult<Self> {
        Ok(Self {
            next_registration_id: AtomicU64::new(1),
            runtimes: Mutex::new(HashMap::new()),
            next_watch_registration_id: AtomicU64::new(1),
            watch_runtimes: Mutex::new(HashMap::new()),
            watch_notifications: Mutex::new(None),
        })
    }

    /// Start one ingress runtime for one opened serial resource.
    pub(super) fn start_event_runtime(
        &self,
        resource: Arc<WindowsSerialPortResource>,
        _port_name: &str,
    ) -> RuntimeResult<u64> {
        // configure the comm event mask before registering wait callbacks
        let set_mask_status = unsafe { SetCommMask(resource.handle, SERIAL_COMM_EVENT_MASK) };
        if set_mask_status == 0 {
            return Err(serial_io_error(
                "destack.device.serial.open",
                "SetCommMask",
                "failed to configure serial event mask",
            ));
        }

        // build and arm the shared event runtime
        let runtime = Arc::new(WindowsSerialEventRuntime::new(resource)?);
        runtime.arm_wait()?;

        let registration_id = self.next_registration_id.fetch_add(1, Ordering::AcqRel);
        self.runtimes.lock().insert(registration_id, runtime);

        Ok(registration_id)
    }

    /// Stop one ingress runtime when the serial resource drops.
    pub(super) fn stop_event_runtime(&self, registration_id: u64) {
        let Some(runtime) = self.runtimes.lock().remove(&registration_id) else {
            return;
        };

        runtime.shutdown();
    }

    /// Register one opened serial watch stream.
    pub(super) fn register_watch(
        self: &Arc<Self>,
        resource: &Arc<WindowsSerialWatchResource>,
        known_ports: BTreeMap<String, WindowsSerialDescriptorInfo>,
    ) -> RuntimeResult<u64> {
        let registration_id = self
            .next_watch_registration_id
            .fetch_add(1, Ordering::AcqRel);

        // retain the watch before arming the shared topology worker
        {
            let mut watch_runtimes = self.watch_runtimes.lock();
            watch_runtimes.insert(
                registration_id,
                WindowsSerialWatchRegistration {
                    resource: Arc::downgrade(resource),
                    known_ports,
                },
            );
        }

        // ensure one shared native watch subscription is active
        if let Err(error) = self.ensure_watch_notifications() {
            self.watch_runtimes.lock().remove(&registration_id);
            return Err(error);
        }

        Ok(registration_id)
    }

    /// Unregister one opened serial watch stream.
    pub(super) fn unregister_watch(&self, registration_id: u64) {
        let should_stop = {
            let mut watch_runtimes = self.watch_runtimes.lock();
            watch_runtimes.remove(&registration_id);
            watch_runtimes.is_empty()
        };

        // retire the shared native watch subscription when no watches remain
        if should_stop {
            self.watch_notifications.lock().take();
        }
    }

    /// Ensure one shared native serial watch subscription is active.
    fn ensure_watch_notifications(self: &Arc<Self>) -> RuntimeResult<()> {
        let mut watch_notifications = self.watch_notifications.lock();
        if watch_notifications.is_some() {
            return Ok(());
        }

        // register one native configuration-manager notification
        let context = Arc::new(WindowsSerialWatchNotificationContext {
            service: Arc::downgrade(self),
        });
        let filter = CM_NOTIFY_FILTER {
            cbSize: std::mem::size_of::<CM_NOTIFY_FILTER>() as u32,
            Flags: 0,
            FilterType: CM_NOTIFY_FILTER_TYPE_DEVICEINTERFACE,
            Reserved: 0,
            u: CM_NOTIFY_FILTER_0 {
                DeviceInterface: CM_NOTIFY_FILTER_0_2 {
                    ClassGuid: GUID_DEVINTERFACE_COMPORT,
                },
            },
        };
        let mut handle = 0isize;
        let status = unsafe {
            CM_Register_Notification(
                &filter,
                Arc::as_ptr(&context).cast(),
                Some(serial_watch_notification_callback),
                &mut handle,
            )
        };
        if status != CR_SUCCESS {
            return Err(serial_io_error_with_code(
                "destack.device.serial.watchOpen",
                "CM_Register_Notification",
                status,
                "failed to subscribe to serial device-interface notifications",
            ));
        }

        *watch_notifications = Some(WindowsSerialWatchNotifications {
            handle,
            _context: context,
        });

        Ok(())
    }

    /// Refresh all registered watch streams from the current topology snapshot.
    fn refresh_watches(&self) {
        let snapshot = match serial_descriptor_snapshot("destack.device.serial.watchRead") {
            Ok(snapshot) => snapshot,
            Err(error) => {
                tracing::warn!("windows serial topology refresh failed: {error}");
                return;
            }
        };

        let should_stop = {
            let mut watch_runtimes = self.watch_runtimes.lock();

            // publish one delta pass to each live watch and discard stale registrations
            watch_runtimes.retain(|_, registration| {
                let Some(resource) = registration.resource.upgrade() else {
                    return false;
                };

                publish_watch_delta(&resource, &mut registration.known_ports, &snapshot);
                true
            });

            watch_runtimes.is_empty()
        };

        // retire the native subscription once the last watch disappears
        if should_stop {
            self.watch_notifications.lock().take();
        }
    }
}

impl Service for WindowsSerialService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Inline);
}

/// Publish one snapshot delta into one watch queue.
fn publish_watch_delta(
    resource: &WindowsSerialWatchResource,
    known_ports: &mut BTreeMap<String, WindowsSerialDescriptorInfo>,
    snapshot: &BTreeMap<String, WindowsSerialDescriptorInfo>,
) {
    // queue detach or replace transitions from the previous snapshot
    for (port_id, previous) in &*known_ports {
        match snapshot.get(port_id) {
            Some(current) if current == previous => {}
            Some(_) | None => {
                resource
                    .event_queue
                    .push_drop_oldest(WindowsSerialWatchRecord::Detached(previous.clone()));
            }
        }
    }

    // queue attach or replace transitions from the current snapshot
    for (port_id, current) in snapshot {
        match known_ports.get(port_id) {
            Some(previous) if previous == current => {}
            Some(_) | None => {
                resource
                    .event_queue
                    .push_drop_oldest(WindowsSerialWatchRecord::Instance(current.clone()));
            }
        }
    }

    *known_ports = snapshot.clone();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build one deterministic windows serial descriptor fixture.
    fn descriptor(id: &str, name: &str) -> WindowsSerialDescriptorInfo {
        WindowsSerialDescriptorInfo {
            id: id.to_string(),
            transport: crate::platform::device::SerialPortTransport::Native,
            name: name.to_string(),
            manufacturer: None,
            product: None,
            serial_number: None,
            path_units: name.encode_utf16().collect(),
            usb_vendor_id: None,
            usb_product_id: None,
            bluetooth_service_class_id: None,
        }
    }

    /// Publish attached and detached records when the topology snapshot changes.
    #[test]
    fn test_publish_watch_delta_emits_attached_and_detached_records() {
        let resource = WindowsSerialWatchResource {
            event_queue: Arc::new(BoundedQueue::new(8)),
            event_state: Mutex::new(SerialWatchEventState {
                next_sequence: 1,
                reported_dropped_count: 0,
            }),
        };
        let previous = descriptor("COM3", "COM3");
        let current = descriptor("COM4", "COM4");
        let mut known_ports = BTreeMap::from([(previous.id.clone(), previous.clone())]);
        let snapshot = BTreeMap::from([(current.id.clone(), current.clone())]);

        // publish one topology transition
        publish_watch_delta(&resource, &mut known_ports, &snapshot);

        let first = resource.event_queue.try_pop().unwrap();
        let second = resource.event_queue.try_pop().unwrap();

        assert_eq!(first, WindowsSerialWatchRecord::Detached(previous));
        assert_eq!(second, WindowsSerialWatchRecord::Instance(current.clone()));
        assert_eq!(known_ports.get(&current.id), Some(&current));
    }

    /// Keep the watch queue quiet when the topology snapshot is unchanged.
    #[test]
    fn test_publish_watch_delta_skips_unchanged_snapshots() {
        let current = descriptor("COM3", "COM3");
        let resource = WindowsSerialWatchResource {
            event_queue: Arc::new(BoundedQueue::new(8)),
            event_state: Mutex::new(SerialWatchEventState {
                next_sequence: 1,
                reported_dropped_count: 0,
            }),
        };
        let mut known_ports = BTreeMap::from([(current.id.clone(), current.clone())]);
        let snapshot = BTreeMap::from([(current.id.clone(), current.clone())]);

        // publish one identical snapshot
        publish_watch_delta(&resource, &mut known_ports, &snapshot);

        assert!(resource.event_queue.try_pop().is_none());
        assert_eq!(known_ports.get(&current.id), Some(&current));
    }
}

impl WindowsSerialEventRuntime {
    /// Build one live event runtime for one serial resource.
    fn new(resource: Arc<WindowsSerialPortResource>) -> RuntimeResult<Self> {
        let overlapped_io =
            WindowsSerialOverlappedIo::new("destack.device.serial.open", "CreateEventW")?;

        Ok(Self {
            resource,
            is_shutdown: AtomicBool::new(false),
            wait_state: Mutex::new(WindowsSerialEventWaitState {
                overlapped_io,
                event_mask: 0,
                wait: None,
            }),
        })
    }

    /// Queue one disconnect event for this runtime.
    fn queue_disconnected(&self) {
        self.resource
            .event_queue
            .push_drop_oldest(WindowsSerialEventRecord::Disconnected);
    }

    /// Queue one generic backend error event for this runtime.
    fn queue_unknown_error(&self, backend_code: Option<u32>) {
        self.resource
            .event_queue
            .push_drop_oldest(WindowsSerialEventRecord::Error {
                kind: SerialErrorKind::Unknown,
                backend_code: backend_code.map(|code| code as i32),
                backend_detail: None,
            });
    }

    /// Register one threadpool wait callback for the current overlapped event.
    fn register_wait_callback(self: &Arc<Self>) -> RuntimeResult<()> {
        let runtime_pointer = Arc::as_ptr(self).cast_mut().cast::<c_void>();

        // register one one-shot wait against the shared windows wait ingress
        let registered_wait = {
            let wait_state = self.wait_state.lock();

            WindowsRegisteredWait::register(
                "destack.device.serial.open",
                wait_state.overlapped_io.event_handle(),
                Some(serial_event_wait_callback),
                runtime_pointer,
            )?
        };

        // publish the wait handle unless shutdown won the race
        let mut wait_state = self.wait_state.lock();
        if self.is_shutdown.load(Ordering::Acquire) {
            let mut registered_wait = registered_wait;
            let _ = registered_wait.unregister("destack.device.serial.close");

            return Ok(());
        }

        wait_state.wait = Some(registered_wait);

        Ok(())
    }

    /// Submit or immediately process the next serial comm-event wait.
    fn arm_wait(self: &Arc<Self>) -> RuntimeResult<()> {
        loop {
            // stop rearming after shutdown
            if self.is_shutdown.load(Ordering::Acquire) {
                return Ok(());
            }

            // submit the next overlapped wait
            let submission = {
                let mut wait_state = self.wait_state.lock();
                let WindowsSerialEventWaitState {
                    overlapped_io,
                    event_mask,
                    ..
                } = &mut *wait_state;
                *event_mask = 0;

                submit_serial_event_wait(&self.resource, event_mask, overlapped_io)
            };

            // react to the submission result
            match submission {
                Ok(WindowsSerialWaitSubmission::Pending) => {
                    self.register_wait_callback()?;

                    return Ok(());
                }
                Ok(WindowsSerialWaitSubmission::Ready) => {
                    let event_mask = self.wait_state.lock().event_mask;
                    if !process_serial_event_mask(&self.resource, event_mask) {
                        return Ok(());
                    }
                }
                Err(code) => {
                    if is_disconnected_code(code) {
                        self.queue_disconnected();
                    } else {
                        self.queue_unknown_error(Some(code));
                    }

                    return Ok(());
                }
            }
        }
    }

    /// Process one completed threadpool wait callback.
    fn process_wait_callback(self: &Arc<Self>) {
        // ignore late callbacks after shutdown
        if self.is_shutdown.load(Ordering::Acquire) {
            return;
        }

        // retire the one-shot wait registration before observing completion
        let event_mask = {
            let mut wait_state = self.wait_state.lock();
            wait_state.wait = None;

            let mut transferred = 0u32;
            let status = unsafe {
                GetOverlappedResult(
                    self.resource.handle,
                    &wait_state.overlapped_io.overlapped,
                    &mut transferred,
                    0,
                )
            };
            if status == 0 {
                let code = core_platform::last_error_code() as u32;

                if code == ERROR_OPERATION_ABORTED && self.is_shutdown.load(Ordering::Acquire) {
                    return;
                }

                if is_disconnected_code(code) {
                    self.queue_disconnected();
                } else {
                    self.queue_unknown_error(Some(code));
                }

                return;
            }

            wait_state.event_mask
        };

        // process the completed event mask and rearm the next wait
        if !process_serial_event_mask(&self.resource, event_mask) {
            return;
        }

        if let Err(error) = self.arm_wait() {
            tracing::warn!("windows serial wait rearm failed: {error}");
            self.queue_unknown_error(None);
        }
    }

    /// Shut down this runtime and retire its pending wait state.
    fn shutdown(&self) {
        self.is_shutdown.store(true, Ordering::Release);

        // detach the registered wait handle before waiting on callbacks
        let wait_handle = {
            let mut wait_state = self.wait_state.lock();
            wait_state.wait.take()
        };
        if let Some(mut wait_handle) = wait_handle
            && let Err(error) = wait_handle.unregister("destack.device.serial.close")
        {
            tracing::warn!("windows serial wait unregistration failed: {error}");
        }

        // cancel any in-flight overlapped wait on the serial handle
        let wait_state = self.wait_state.lock();
        let cancel_status =
            unsafe { CancelIoEx(self.resource.handle, &wait_state.overlapped_io.overlapped) };
        if cancel_status == 0 {
            let code = core_platform::last_error_code() as u32;
            if code != ERROR_NOT_FOUND && code != ERROR_OPERATION_ABORTED {
                tracing::warn!("windows serial wait cancellation failed with code {code}");
            }
        }
    }
}

/// Dispatch one completed windows serial wait callback.
unsafe extern "system" fn serial_event_wait_callback(context: *mut c_void, _timed_out: u8) {
    if context.is_null() {
        return;
    }

    let runtime_pointer = context.cast::<WindowsSerialEventRuntime>();

    // retain one temporary strong reference while the callback runs
    unsafe {
        Arc::increment_strong_count(runtime_pointer);
    }
    let runtime = unsafe { Arc::from_raw(runtime_pointer) };

    runtime.process_wait_callback();
}

/// Dispatch one native Windows serial topology notification.
unsafe extern "system" fn serial_watch_notification_callback(
    _notification: HCMNOTIFICATION,
    context: *const c_void,
    action: CM_NOTIFY_ACTION,
    _event_data: *const CM_NOTIFY_EVENT_DATA,
    _event_data_size: u32,
) -> u32 {
    if context.is_null() {
        return CR_SUCCESS;
    }

    // ignore notification kinds outside serial interface topology changes
    if action != CM_NOTIFY_ACTION_DEVICEINTERFACEARRIVAL
        && action != CM_NOTIFY_ACTION_DEVICEINTERFACEREMOVAL
    {
        return CR_SUCCESS;
    }

    let context = unsafe { &*context.cast::<WindowsSerialWatchNotificationContext>() };

    // refresh every live watch from the latest SetupAPI snapshot
    if let Some(service) = context.service.upgrade() {
        service.refresh_watches();
    }

    CR_SUCCESS
}

/// Resolve the shared Windows serial ingress service.
pub(crate) fn windows_serial_service(
    operation: &'static str,
) -> RuntimeResult<Arc<WindowsSerialService>> {
    WindowsSerialService::global(WindowsSerialService::new).map_err(|error| {
        let platform_code = error.platform_error().map(|error| error.code);

        RuntimeError::from(PlatformError::io_with(
            platform_code,
            None,
            None,
            Some(operation.to_string()),
            Some(String::from("windowsSerialService")),
            format!("windows serial service unavailable: {error}"),
        ))
        .boxed()
    })
}
