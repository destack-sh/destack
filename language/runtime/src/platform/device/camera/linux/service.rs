use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Weak};
use std::time::Duration;

use parking_lot::Mutex;

use super::device::camera_descriptor_snapshot;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::core::BoundedQueue;
use crate::platform::device::CameraDeviceDescriptorValue;
use crate::runtime::service::Service;
use crate::runtime::service::executor::periodic::{PeriodicTaskHandle, open_periodic_task};
use crate::runtime::{ExecutionMode, ExecutionPolicy};

use super::super::core::CameraWatchEventState;

/// Maximum queued camera topology events per opened watch stream.
pub(super) const CAMERA_WATCH_QUEUE_CAPACITY: usize = 64;

/// Poll interval for one shared Linux camera topology worker.
const CAMERA_WATCH_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// One queued Linux camera topology event.
#[derive(Debug, Clone)]
pub(super) enum LinuxCameraWatchRecord {
    /// One attached camera endpoint.
    Instance(CameraDeviceDescriptorValue),
    /// One detached camera endpoint.
    Detached(CameraDeviceDescriptorValue),
}

/// One opened Linux camera watch resource.
pub(super) struct LinuxCameraWatchResource {
    /// Queued topology events for this watch stream.
    pub(super) event_queue: Arc<BoundedQueue<LinuxCameraWatchRecord>>,
    /// Monotonic event sequencing for this watch stream.
    pub(super) event_state: Mutex<CameraWatchEventState>,
}

/// One registered Linux camera topology watch.
struct LinuxCameraWatchRegistration {
    /// Weak handle to the opened watch resource.
    resource: Weak<LinuxCameraWatchResource>,
    /// Last published descriptor snapshot for this watch stream.
    known_devices: BTreeMap<String, CameraDeviceDescriptorValue>,
}

/// One process-global Linux camera topology service.
pub(crate) struct LinuxCameraWatchService {
    /// The next stable watch registration identifier.
    next_registration_id: AtomicU64,
    /// Registered watch streams keyed by registration identifier.
    registrations: Mutex<HashMap<u64, LinuxCameraWatchRegistration>>,
    /// Shared periodic worker when at least one watch is active.
    task: Mutex<Option<PeriodicTaskHandle>>,
}

impl LinuxCameraWatchService {
    /// Build one empty Linux camera topology service.
    fn new() -> RuntimeResult<Self> {
        Ok(Self {
            next_registration_id: AtomicU64::new(1),
            registrations: Mutex::new(HashMap::new()),
            task: Mutex::new(None),
        })
    }

    /// Register one opened camera watch stream.
    pub(super) fn register_watch(
        self: &Arc<Self>,
        resource: &Arc<LinuxCameraWatchResource>,
        known_devices: BTreeMap<String, CameraDeviceDescriptorValue>,
    ) -> RuntimeResult<u64> {
        let registration_id = self.next_registration_id.fetch_add(1, Ordering::AcqRel);

        // retain the watch before arming the shared worker
        {
            let mut registrations = self.registrations.lock();
            registrations.insert(
                registration_id,
                LinuxCameraWatchRegistration {
                    resource: Arc::downgrade(resource),
                    known_devices,
                },
            );
        }

        // ensure one shared topology worker is running
        if let Err(error) = self.ensure_worker() {
            self.registrations.lock().remove(&registration_id);
            return Err(error);
        }

        Ok(registration_id)
    }

    /// Unregister one opened camera watch stream.
    pub(super) fn unregister_watch(&self, registration_id: u64) {
        let should_stop = {
            let mut registrations = self.registrations.lock();
            registrations.remove(&registration_id);
            registrations.is_empty()
        };

        // retire the shared worker when no watches remain
        if should_stop {
            self.task.lock().take();
        }
    }

    /// Ensure one shared periodic topology worker is running.
    fn ensure_worker(self: &Arc<Self>) -> RuntimeResult<()> {
        let mut task = self.task.lock();
        if task.is_some() {
            return Ok(());
        }

        // spawn one shared periodic snapshot worker
        let service = Arc::clone(self);
        let next_task = open_periodic_task(
            "destack-camera-linux-watch",
            Self::POLICY,
            CAMERA_WATCH_POLL_INTERVAL,
            move || {
                service.refresh_watches();

                Ok(())
            },
        )?;
        *task = Some(next_task);

        Ok(())
    }

    /// Refresh all registered watch streams from the current topology snapshot.
    fn refresh_watches(&self) {
        let snapshot = match camera_descriptor_snapshot("destack.device.camera.device.watchRead") {
            Ok(snapshot) => snapshot,
            Err(error) => {
                tracing::warn!("linux camera topology refresh failed: {error}");
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

                publish_watch_delta(&resource, &mut registration.known_devices, &snapshot);
                true
            });

            registrations.is_empty()
        };

        // retire the worker once the last watch disappears
        if should_stop {
            self.task.lock().take();
        }
    }
}

impl Service for LinuxCameraWatchService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Polling);
}

/// Publish one snapshot delta into one watch queue.
fn publish_watch_delta(
    resource: &LinuxCameraWatchResource,
    known_devices: &mut BTreeMap<String, CameraDeviceDescriptorValue>,
    snapshot: &BTreeMap<String, CameraDeviceDescriptorValue>,
) {
    // queue detach or replace transitions from the previous snapshot
    for (device_id, previous) in &*known_devices {
        match snapshot.get(device_id) {
            Some(current) if current == previous => {}
            Some(_) | None => {
                resource
                    .event_queue
                    .push_drop_oldest(LinuxCameraWatchRecord::Detached(previous.clone()));
            }
        }
    }

    // queue attach or replace transitions from the current snapshot
    for (device_id, current) in snapshot {
        match known_devices.get(device_id) {
            Some(previous) if previous == current => {}
            Some(_) | None => {
                resource
                    .event_queue
                    .push_drop_oldest(LinuxCameraWatchRecord::Instance(current.clone()));
            }
        }
    }

    *known_devices = snapshot.clone();
}

/// Resolve the shared Linux camera topology service.
pub(crate) fn linux_camera_watch_service(
    operation: &'static str,
) -> RuntimeResult<Arc<LinuxCameraWatchService>> {
    LinuxCameraWatchService::global(LinuxCameraWatchService::new).map_err(|error| {
        let platform_code = error.platform_error().map(|error| error.code);

        RuntimeError::from(PlatformError::io_with(
            platform_code,
            None,
            None,
            Some(operation.to_string()),
            Some(String::from("linuxCameraWatchService")),
            format!("linux camera watch service unavailable: {error}"),
        ))
        .boxed()
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use parking_lot::Mutex;

    use super::{
        CAMERA_WATCH_QUEUE_CAPACITY, LinuxCameraWatchRecord, LinuxCameraWatchResource,
        publish_watch_delta,
    };
    use crate::platform::core::BoundedQueue;
    use crate::platform::device::{CameraDeviceDescriptorValue, CameraFacingMode};

    /// Return one deterministic camera descriptor fixture.
    fn test_camera_descriptor(id: &str, name: &str) -> CameraDeviceDescriptorValue {
        CameraDeviceDescriptorValue {
            id: id.to_string(),
            group_id: None,
            name: name.to_string(),
            manufacturer: None,
            facing_mode: CameraFacingMode::Unknown,
            depth_capable: false,
        }
    }

    /// Ignore unchanged camera snapshots when publishing Linux watch deltas.
    #[test]
    fn test_publish_watch_delta_ignores_unchanged_snapshot() {
        let event_queue = Arc::new(BoundedQueue::new(CAMERA_WATCH_QUEUE_CAPACITY));
        let resource = LinuxCameraWatchResource {
            event_queue: event_queue.clone(),
            event_state: Mutex::new(super::CameraWatchEventState { next_sequence: 1 }),
        };
        let descriptor = test_camera_descriptor("camera.test", "Camera");
        let mut known_devices = BTreeMap::from([(descriptor.id.clone(), descriptor.clone())]);
        let snapshot = BTreeMap::from([(descriptor.id.clone(), descriptor)]);

        publish_watch_delta(&resource, &mut known_devices, &snapshot);

        let event = event_queue.try_pop_or_else(|| Err(()));
        assert!(event.is_err());
    }

    /// Emit one detach and attach pair when one Linux camera descriptor changes.
    #[test]
    fn test_publish_watch_delta_emits_replace_transition() {
        let event_queue = Arc::new(BoundedQueue::new(CAMERA_WATCH_QUEUE_CAPACITY));
        let resource = LinuxCameraWatchResource {
            event_queue: event_queue.clone(),
            event_state: Mutex::new(super::CameraWatchEventState { next_sequence: 1 }),
        };
        let previous = test_camera_descriptor("camera.test", "Camera");
        let current = test_camera_descriptor("camera.test", "Camera Updated");
        let mut known_devices = BTreeMap::from([(previous.id.clone(), previous.clone())]);
        let snapshot = BTreeMap::from([(current.id.clone(), current.clone())]);

        publish_watch_delta(&resource, &mut known_devices, &snapshot);

        let detached = event_queue.try_pop_or_else(|| Err(())).unwrap();
        let attached = event_queue.try_pop_or_else(|| Err(())).unwrap();

        assert!(matches!(detached, LinuxCameraWatchRecord::Detached(value) if value == previous));
        assert!(matches!(attached, LinuxCameraWatchRecord::Instance(value) if value == current));
    }
}
