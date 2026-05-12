use std::collections::{BTreeMap, HashMap};
use std::ffi::{CString, c_char, c_void};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Weak};

use core_foundation_sys::runloop::{
    CFRunLoopAddSource, CFRunLoopGetCurrent, CFRunLoopRef, CFRunLoopRun, CFRunLoopSourceRef,
    CFRunLoopStop, kCFRunLoopDefaultMode,
};
use parking_lot::Mutex;

use super::identity::serial_descriptor_snapshot;
use super::state::UnixSerialDescriptorInfo;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::core::BoundedQueue;
use crate::runtime::service::Service;
use crate::runtime::{ExecutionMode, ExecutionPolicy, WorkerLoop};

use super::super::core::SerialWatchEventState;

/// Successful IOKit result code.
const APPLE_KERN_SUCCESS: i32 = 0;

/// Null master-port value used by current IOKit matching entry points.
const APPLE_IOKIT_MASTER_PORT_DEFAULT: u32 = 0;

/// First-match notification selector.
const APPLE_IO_FIRST_MATCH_NOTIFICATION: &[u8] = b"IOServiceFirstMatch\0";

/// Termination notification selector.
const APPLE_IO_TERMINATED_NOTIFICATION: &[u8] = b"IOServiceTerminate\0";

/// One IOKit object handle.
type AppleIoObject = u32;

/// One retained IOKit notification port.
type AppleIoNotificationPortRef = *mut c_void;

/// Maximum queued serial topology events per opened watch stream.
pub(super) const SERIAL_WATCH_QUEUE_CAPACITY: usize = 64;

/// One queued unix serial topology event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum UnixSerialWatchRecord {
    /// One attached serial endpoint.
    Instance(UnixSerialDescriptorInfo),
    /// One detached serial endpoint.
    Detached(UnixSerialDescriptorInfo),
}

/// One opened unix serial watch resource.
pub(super) struct UnixSerialWatchResource {
    /// Queued topology events for this watch stream.
    pub(super) event_queue: Arc<BoundedQueue<UnixSerialWatchRecord>>,
    /// Monotonic event sequencing for this watch stream.
    pub(super) event_state: Mutex<SerialWatchEventState>,
}

/// One registered unix serial topology watch.
struct UnixSerialWatchRegistration {
    /// Weak handle to the opened watch resource.
    resource: Weak<UnixSerialWatchResource>,
    /// Last published descriptor snapshot for this watch stream.
    known_ports: BTreeMap<String, UnixSerialDescriptorInfo>,
}

/// One process-global unix serial topology service.
pub(crate) struct UnixSerialWatchService {
    /// The next stable watch registration identifier.
    next_registration_id: AtomicU64,
    /// Registered watch streams keyed by registration identifier.
    registrations: Mutex<HashMap<u64, UnixSerialWatchRegistration>>,
    /// Shared native watcher when at least one watch is active.
    watch_runtime: Mutex<Option<UnixSerialWatchRuntime>>,
}

/// One callback context for native IOKit notifications.
struct AppleSerialWatchNotificationContext {
    /// Weak handle back to the shared serial service.
    service: Weak<UnixSerialWatchService>,
}

/// One live macOS serial topology watcher.
struct UnixSerialWatchRuntime {
    /// Shared ingress loop runtime.
    _loop: WorkerLoop,
}

/// One shared run loop handle for watcher teardown.
struct AppleRunLoopHandle {
    /// The active run loop when the watcher thread is ready.
    run_loop: Mutex<Option<usize>>,
}

impl AppleRunLoopHandle {
    /// Build one empty run loop handle.
    fn new() -> Self {
        Self {
            run_loop: Mutex::new(None),
        }
    }

    /// Publish one watcher run loop.
    fn set(&self, run_loop: CFRunLoopRef) {
        *self.run_loop.lock() = Some(run_loop as usize);
    }

    /// Stop the active watcher run loop when present.
    fn stop(&self) {
        let run_loop = *self.run_loop.lock();

        if let Some(run_loop) = run_loop {
            unsafe {
                CFRunLoopStop(run_loop as CFRunLoopRef);
            }
        }
    }
}

impl UnixSerialWatchService {
    /// Build one empty unix serial topology service.
    fn new() -> RuntimeResult<Self> {
        Ok(Self {
            next_registration_id: AtomicU64::new(1),
            registrations: Mutex::new(HashMap::new()),
            watch_runtime: Mutex::new(None),
        })
    }

    /// Register one opened serial watch stream.
    pub(super) fn register_watch(
        self: &Arc<Self>,
        resource: &Arc<UnixSerialWatchResource>,
        known_ports: BTreeMap<String, UnixSerialDescriptorInfo>,
    ) -> RuntimeResult<u64> {
        let registration_id = self.next_registration_id.fetch_add(1, Ordering::AcqRel);

        // retain the watch before arming the shared worker
        {
            let mut registrations = self.registrations.lock();
            registrations.insert(
                registration_id,
                UnixSerialWatchRegistration {
                    resource: Arc::downgrade(resource),
                    known_ports,
                },
            );
        }

        // ensure one shared topology watcher is active
        if let Err(error) = self.ensure_watch_runtime() {
            self.registrations.lock().remove(&registration_id);
            return Err(error);
        }

        Ok(registration_id)
    }

    /// Unregister one opened serial watch stream.
    pub(super) fn unregister_watch(&self, registration_id: u64) {
        let should_stop = {
            let mut registrations = self.registrations.lock();
            registrations.remove(&registration_id);
            registrations.is_empty()
        };

        // retire the shared watcher when no watches remain
        if should_stop {
            self.watch_runtime.lock().take();
        }
    }

    /// Ensure one shared native topology watcher is active.
    fn ensure_watch_runtime(self: &Arc<Self>) -> RuntimeResult<()> {
        let mut watch_runtime = self.watch_runtime.lock();
        if watch_runtime.is_some() {
            return Ok(());
        }

        let context = Arc::new(AppleSerialWatchNotificationContext {
            service: Arc::downgrade(self),
        });
        let run_loop = Arc::new(AppleRunLoopHandle::new());
        // run the IOKit notification loop on one ingress thread
        let loop_runtime = WorkerLoop::open(
            "destack-serial-iokit-watch",
            "platform.service.spawn",
            Self::POLICY,
            move || {
                let notification_port =
                    unsafe { arm_serial_watch_notifications(&context, &run_loop) }?;
                let notification_port = notification_port as usize;
                let shutdown_run_loop = Arc::clone(&run_loop);

                let shutdown = Box::new(move || {
                    shutdown_run_loop.stop();
                });
                let run = Box::new(move || {
                    let _context = context;
                    let notification_port = notification_port as AppleIoNotificationPortRef;

                    unsafe {
                        CFRunLoopRun();
                        IONotificationPortDestroy(notification_port);
                    }

                    Ok(())
                });

                Ok((shutdown, run))
            },
        )?;

        *watch_runtime = Some(UnixSerialWatchRuntime {
            _loop: loop_runtime,
        });

        Ok(())
    }

    /// Refresh all registered watch streams from the current topology snapshot.
    fn refresh_watches(&self) {
        let snapshot = match serial_descriptor_snapshot("destack.device.serial.watchRead") {
            Ok(snapshot) => snapshot,
            Err(error) => {
                tracing::warn!("macos serial topology refresh failed: {error}");
                return;
            }
        };

        let should_stop = {
            let mut registrations = self.registrations.lock();

            // publish one delta pass to each live watch and discard stale registrations
            registrations.retain(|_, registration| {
                let Some(resource) = registration.resource.upgrade() else {
                    return false;
                };

                publish_watch_delta(&resource, &mut registration.known_ports, &snapshot);
                true
            });

            registrations.is_empty()
        };

        // retire the watcher once the last watch disappears
        if should_stop {
            self.watch_runtime.lock().take();
        }
    }
}

impl Service for UnixSerialWatchService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Loop);
}

/// Arm native IOKit serial notifications on the current thread run loop.
unsafe fn arm_serial_watch_notifications(
    context: &Arc<AppleSerialWatchNotificationContext>,
    run_loop_handle: &Arc<AppleRunLoopHandle>,
) -> RuntimeResult<AppleIoNotificationPortRef> {
    let notification_port = unsafe { IONotificationPortCreate(APPLE_IOKIT_MASTER_PORT_DEFAULT) };
    if notification_port.is_null() {
        return Err(macos_watch_error(
            "IONotificationPortCreate",
            "failed to create macos serial notification port",
        ));
    }

    // attach the notification source to the current thread run loop
    let run_loop_source = unsafe { IONotificationPortGetRunLoopSource(notification_port) };
    if run_loop_source.is_null() {
        unsafe {
            IONotificationPortDestroy(notification_port);
        }

        return Err(macos_watch_error(
            "IONotificationPortGetRunLoopSource",
            "failed to resolve macos serial notification run loop source",
        ));
    }

    let run_loop = unsafe { CFRunLoopGetCurrent() };
    if run_loop.is_null() {
        unsafe {
            IONotificationPortDestroy(notification_port);
        }

        return Err(macos_watch_error(
            "CFRunLoopGetCurrent",
            "failed to resolve macos serial watcher run loop",
        ));
    }

    unsafe {
        CFRunLoopAddSource(run_loop, run_loop_source, kCFRunLoopDefaultMode);
    }

    run_loop_handle.set(run_loop);

    // register one first-match and one terminate notification
    let first_match = unsafe {
        register_serial_notification(
            notification_port,
            APPLE_IO_FIRST_MATCH_NOTIFICATION.as_ptr().cast(),
            context,
        )
    };
    let terminated = unsafe {
        register_serial_notification(
            notification_port,
            APPLE_IO_TERMINATED_NOTIFICATION.as_ptr().cast(),
            context,
        )
    };

    match (first_match, terminated) {
        (Ok(_), Ok(_)) => Ok(notification_port),
        (Err(error), _) | (_, Err(error)) => {
            unsafe {
                IONotificationPortDestroy(notification_port);
            }

            Err(error)
        }
    }
}

/// Register one native serial notification and arm its iterator.
unsafe fn register_serial_notification(
    notification_port: AppleIoNotificationPortRef,
    notification_type: *const c_char,
    context: &Arc<AppleSerialWatchNotificationContext>,
) -> RuntimeResult<AppleIoObject> {
    let class_name = CString::new("IOSerialBSDClient").map_err(|error| {
        RuntimeError::from(PlatformError::io(format!(
            "failed to encode macos serial service class name: {error}",
        )))
        .boxed()
    })?;
    let matching = unsafe { IOServiceMatching(class_name.as_ptr()) };
    if matching.is_null() {
        return Err(macos_watch_error(
            "IOServiceMatching",
            "failed to create macos serial matching dictionary",
        ));
    }

    let mut iterator = 0;
    let status = unsafe {
        IOServiceAddMatchingNotification(
            notification_port,
            notification_type,
            matching,
            serial_watch_notification_callback,
            Arc::as_ptr(context).cast_mut().cast(),
            &mut iterator,
        )
    };
    if status != APPLE_KERN_SUCCESS {
        return Err(macos_watch_error(
            "IOServiceAddMatchingNotification",
            "failed to subscribe to macos serial topology notifications",
        ));
    }

    unsafe {
        drain_serial_iterator(iterator, context);
    }

    Ok(iterator)
}

/// Drain one notification iterator and refresh the shared snapshot.
unsafe extern "C" fn serial_watch_notification_callback(
    refcon: *mut c_void,
    iterator: AppleIoObject,
) {
    let context = unsafe { (refcon as *const AppleSerialWatchNotificationContext).as_ref() };

    if let Some(context) = context {
        unsafe {
            drain_serial_iterator(iterator, context);
        }
    }
}

/// Drain one iterator to rearm the notification and publish one refresh.
unsafe fn drain_serial_iterator(
    iterator: AppleIoObject,
    context: &AppleSerialWatchNotificationContext,
) {
    loop {
        let object = unsafe { IOIteratorNext(iterator) };
        if object == 0 {
            break;
        }

        unsafe {
            IOObjectRelease(object);
        }
    }

    if let Some(service) = context.service.upgrade() {
        service.refresh_watches();
    }
}

/// Build one macOS serial watch error.
fn macos_watch_error(call: &'static str, message: &str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        None,
        None,
        None,
        Some(String::from("destack.device.serial.watchOpen")),
        Some(call.to_string()),
        message.to_string(),
    ))
    .boxed()
}

/// Publish one snapshot delta into one watch queue.
fn publish_watch_delta(
    resource: &UnixSerialWatchResource,
    known_ports: &mut BTreeMap<String, UnixSerialDescriptorInfo>,
    snapshot: &BTreeMap<String, UnixSerialDescriptorInfo>,
) {
    // queue detach or replace transitions from the previous snapshot
    for (port_id, previous) in &*known_ports {
        match snapshot.get(port_id) {
            Some(current) if current == previous => {}
            Some(_) | None => {
                resource
                    .event_queue
                    .push_drop_oldest(UnixSerialWatchRecord::Detached(previous.clone()));
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
                    .push_drop_oldest(UnixSerialWatchRecord::Instance(current.clone()));
            }
        }
    }

    *known_ports = snapshot.clone();
}

/// Resolve the shared unix serial topology service.
pub(crate) fn unix_serial_watch_service(
    operation: &'static str,
) -> RuntimeResult<Arc<UnixSerialWatchService>> {
    UnixSerialWatchService::global(UnixSerialWatchService::new).map_err(|error| {
        let platform_code = error.platform_error().map(|error| error.code);

        RuntimeError::from(PlatformError::io_with(
            platform_code,
            None,
            None,
            Some(operation.to_string()),
            Some(String::from("unixSerialWatchService")),
            format!("unix serial watch service unavailable: {error}"),
        ))
        .boxed()
    })
}

// link apple serial notification entry points
#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    /// Create one matching dictionary for one IOKit service class.
    fn IOServiceMatching(name: *const c_char) -> *mut c_void;
    /// Create one retained notification port.
    fn IONotificationPortCreate(master_port: u32) -> AppleIoNotificationPortRef;
    /// Destroy one retained notification port.
    fn IONotificationPortDestroy(notification_port: AppleIoNotificationPortRef);
    /// Resolve one run loop source for the retained notification port.
    fn IONotificationPortGetRunLoopSource(
        notification_port: AppleIoNotificationPortRef,
    ) -> CFRunLoopSourceRef;
    /// Register one matching notification against the retained notification port.
    fn IOServiceAddMatchingNotification(
        notification_port: AppleIoNotificationPortRef,
        notification_type: *const c_char,
        matching: *mut c_void,
        callback: unsafe extern "C" fn(*mut c_void, AppleIoObject),
        refcon: *mut c_void,
        iterator: *mut AppleIoObject,
    ) -> i32;
    /// Advance one iterator and return the next object.
    fn IOIteratorNext(iterator: AppleIoObject) -> AppleIoObject;
    /// Release one IOKit object.
    fn IOObjectRelease(object: AppleIoObject) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build one deterministic unix serial descriptor fixture.
    fn descriptor(id: &str, name: &str) -> UnixSerialDescriptorInfo {
        UnixSerialDescriptorInfo {
            id: id.to_string(),
            transport: crate::platform::device::SerialPortTransport::Native,
            name: name.to_string(),
            manufacturer: None,
            product: None,
            serial_number: None,
            path_bytes: name.as_bytes().to_vec(),
            usb_vendor_id: None,
            usb_product_id: None,
            bluetooth_service_class_id: None,
        }
    }

    /// Publish attached and detached records when the topology snapshot changes.
    #[test]
    fn test_publish_watch_delta_emits_attached_and_detached_records() {
        let resource = UnixSerialWatchResource {
            event_queue: Arc::new(BoundedQueue::new(8)),
            event_state: Mutex::new(SerialWatchEventState {
                next_sequence: 1,
                reported_dropped_count: 0,
            }),
        };
        let previous = descriptor("serial-a", "/dev/cu.usbserial-a");
        let current = descriptor("serial-b", "/dev/cu.usbserial-b");
        let mut known_ports = BTreeMap::from([(previous.id.clone(), previous.clone())]);
        let snapshot = BTreeMap::from([(current.id.clone(), current.clone())]);

        // publish one topology transition
        publish_watch_delta(&resource, &mut known_ports, &snapshot);

        let first = resource.event_queue.try_pop().unwrap();
        let second = resource.event_queue.try_pop().unwrap();

        assert_eq!(first, UnixSerialWatchRecord::Detached(previous));
        assert_eq!(second, UnixSerialWatchRecord::Instance(current.clone()));
        assert_eq!(known_ports.get(&current.id), Some(&current));
    }

    /// Keep the watch queue quiet when the topology snapshot is unchanged.
    #[test]
    fn test_publish_watch_delta_skips_unchanged_snapshots() {
        let current = descriptor("serial-a", "/dev/cu.usbserial-a");
        let resource = UnixSerialWatchResource {
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
