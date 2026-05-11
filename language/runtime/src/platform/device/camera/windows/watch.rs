use std::collections::BTreeMap;
use std::sync::Arc;

use parking_lot::Mutex;
use windows::Devices::Enumeration::{
    DeviceClass, DeviceInformation, DeviceInformationUpdate, DeviceWatcher,
};
use windows::Foundation::TypedEventHandler;
use windows::core::IInspectable;

use super::core::*;
use super::metadata::{camera_descriptor_snapshot, windows_camera_error};

/// One camera-watch event queue capacity.
const CAMERA_WATCH_QUEUE_CAPACITY: usize = 64;

/// One queued Windows camera topology event.
#[derive(Debug, Clone)]
enum WindowsCameraWatchRecord {
    /// One attached camera endpoint.
    Instance(CameraDeviceDescriptorValue),
    /// One detached camera endpoint.
    Detached(CameraDeviceDescriptorValue),
}

/// One opened Windows camera watch resource.
struct WindowsCameraWatchResource {
    /// The queued camera events.
    event_queue: Arc<BoundedQueue<WindowsCameraWatchRecord>>,
    /// The shared event sequencing state.
    event_state: Mutex<CameraWatchEventState>,
}

/// Finalizer for one Windows camera watch.
struct WindowsCameraWatchFinalizer {
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
    /// The shared camera event queue.
    event_queue: Arc<BoundedQueue<WindowsCameraWatchRecord>>,
}

impl ResourceFinalizer for WindowsCameraWatchFinalizer {
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

/// Resolve one typed Windows camera watch resource from the resource table.
fn camera_watch_resource(
    binding: &BindingCallContext,
    handle: resource::CameraWatchHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<WindowsCameraWatchResource>> {
    camera_watch_payload(binding, handle, operation)
}

/// Diff the current camera snapshot and queue topology events.
fn refresh_camera_events(
    event_queue: &Arc<BoundedQueue<WindowsCameraWatchRecord>>,
    known_devices: &Arc<Mutex<BTreeMap<String, CameraDeviceDescriptorValue>>>,
    operation: &'static str,
) {
    let Ok(current_snapshot) = camera_descriptor_snapshot(operation) else {
        return;
    };
    let mut known_devices = known_devices.lock();

    publish_watch_delta(event_queue, &mut known_devices, &current_snapshot);
}

/// Publish one snapshot delta into one Windows watch queue.
fn publish_watch_delta(
    event_queue: &Arc<BoundedQueue<WindowsCameraWatchRecord>>,
    known_devices: &mut BTreeMap<String, CameraDeviceDescriptorValue>,
    snapshot: &BTreeMap<String, CameraDeviceDescriptorValue>,
) {
    // queue detach or replace transitions from the previous snapshot
    for (device_id, previous) in &*known_devices {
        match snapshot.get(device_id) {
            Some(current) if current == previous => {}
            Some(_) | None => {
                event_queue.push_drop_oldest(WindowsCameraWatchRecord::Detached(previous.clone()));
            }
        }
    }

    // queue attach or replace transitions from the current snapshot
    for (device_id, current) in snapshot {
        match known_devices.get(device_id) {
            Some(previous) if previous == current => {}
            Some(_) | None => {
                event_queue.push_drop_oldest(WindowsCameraWatchRecord::Instance(current.clone()));
            }
        }
    }

    *known_devices = snapshot.clone();
}

/// Build one watcher callback that refreshes the camera snapshot.
fn watcher_refresh_handler<T: windows::core::RuntimeType + 'static>(
    event_queue: Arc<BoundedQueue<WindowsCameraWatchRecord>>,
    known_devices: Arc<Mutex<BTreeMap<String, CameraDeviceDescriptorValue>>>,
    operation: &'static str,
) -> TypedEventHandler<DeviceWatcher, T> {
    TypedEventHandler::new(move |_watcher, _args| {
        refresh_camera_events(&event_queue, &known_devices, operation);
        Ok(())
    })
}

/// Open one Windows camera topology watch stream.
pub(crate) unsafe fn destack_device_camera_device_watch_open(
    binding: &BindingCallContext,
    out: *mut resource::CameraWatchHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // load the initial topology snapshot before subscribing
    let known_devices = Arc::new(Mutex::new(camera_descriptor_snapshot(
        "destack.device.camera.device.watchOpen",
    )?));
    let event_queue = Arc::new(BoundedQueue::new(CAMERA_WATCH_QUEUE_CAPACITY));
    let resource = Arc::new(WindowsCameraWatchResource {
        event_queue: event_queue.clone(),
        event_state: Mutex::new(CameraWatchEventState { next_sequence: 1 }),
    });

    // seed the queue with the current topology snapshot
    for descriptor in known_devices.lock().values() {
        event_queue.push_drop_oldest(WindowsCameraWatchRecord::Instance(descriptor.clone()));
    }

    // create one watcher for the video capture device class
    let watcher = DeviceInformation::CreateWatcherDeviceClass(DeviceClass::VideoCapture).map_err(
        |error| {
            windows_camera_error(
                "destack.device.camera.device.watchOpen",
                "DeviceInformation::CreateWatcherDeviceClass",
                &error,
            )
        },
    )?;
    let added_token = watcher
        .Added(&watcher_refresh_handler::<DeviceInformation>(
            event_queue.clone(),
            known_devices.clone(),
            "destack.device.camera.device.watchOpen",
        ))
        .map_err(|error| {
            windows_camera_error(
                "destack.device.camera.device.watchOpen",
                "DeviceWatcher::Added",
                &error,
            )
        })?;
    let updated_token = watcher
        .Updated(&watcher_refresh_handler::<DeviceInformationUpdate>(
            event_queue.clone(),
            known_devices.clone(),
            "destack.device.camera.device.watchOpen",
        ))
        .map_err(|error| {
            windows_camera_error(
                "destack.device.camera.device.watchOpen",
                "DeviceWatcher::Updated",
                &error,
            )
        })?;
    let removed_token = watcher
        .Removed(&watcher_refresh_handler::<DeviceInformationUpdate>(
            event_queue.clone(),
            known_devices.clone(),
            "destack.device.camera.device.watchOpen",
        ))
        .map_err(|error| {
            windows_camera_error(
                "destack.device.camera.device.watchOpen",
                "DeviceWatcher::Removed",
                &error,
            )
        })?;
    let enumeration_completed_token = watcher
        .EnumerationCompleted(&watcher_refresh_handler::<IInspectable>(
            event_queue.clone(),
            known_devices.clone(),
            "destack.device.camera.device.watchOpen",
        ))
        .map_err(|error| {
            windows_camera_error(
                "destack.device.camera.device.watchOpen",
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
            windows_camera_error(
                "destack.device.camera.device.watchOpen",
                "DeviceWatcher::Stopped",
                &error,
            )
        })?;
    watcher.Start().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.device.watchOpen",
            "DeviceWatcher::Start",
            &error,
        )
    })?;

    // store the opened camera watch
    let entry = ResourceEntry::new(ResourceKind::CameraWatch)
        .with_label(CAMERA_WATCH_RESOURCE_LABEL)
        .with_payload(resource)
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            WindowsCameraWatchFinalizer {
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
        out.write(resource::CameraWatchHandle(handle));
    }

    Ok(())
}

/// Close one Windows camera topology watch stream.
pub(crate) unsafe fn destack_device_camera_device_watch_close(
    binding: &BindingCallContext,
    handle: resource::CameraWatchHandle,
) -> RuntimeResult<()> {
    close_camera_watch_resource(binding, handle, "destack.device.camera.device.watchClose")
}

/// Read one blocking Windows camera topology event.
pub(crate) unsafe fn destack_device_camera_device_watch_read(
    binding: &BindingCallContext,
    out: *mut CameraWatchEvent,
    handle: resource::CameraWatchHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // read the next queued topology event
    let resource =
        camera_watch_resource(binding, handle, "destack.device.camera.device.watchRead")?;
    let timeout = std::time::Duration::from_nanos(timeoutns.max(1));
    let event = resource.event_queue.pop_with_timeout_or_else(timeout, || {
        Err(core_platform::io_would_block(
            "destack.device.camera.device.watchRead",
            "camera watch event queue is empty",
        ))
    })?;

    // encode the event on the binding heap
    let event = match event {
        WindowsCameraWatchRecord::Instance(descriptor) => {
            attached_watch_event(binding, &resource.event_state, descriptor)
        }
        WindowsCameraWatchRecord::Detached(descriptor) => {
            detached_watch_event(binding, &resource.event_state, descriptor)
        }
    };
    unsafe {
        out.write(event);
    }

    Ok(())
}

/// Poll one Windows camera topology event without blocking.
pub(crate) unsafe fn destack_device_camera_device_watch_try_read(
    binding: &BindingCallContext,
    out: *mut CameraWatchEvent,
    handle: resource::CameraWatchHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // read the next queued topology event
    let resource =
        camera_watch_resource(binding, handle, "destack.device.camera.device.watchTryRead")?;
    let event = resource.event_queue.try_pop_or_else(|| {
        Err(core_platform::io_would_block(
            "destack.device.camera.device.watchTryRead",
            "camera watch event queue is empty",
        ))
    })?;

    // encode the event on the binding heap
    let event = match event {
        WindowsCameraWatchRecord::Instance(descriptor) => {
            attached_watch_event(binding, &resource.event_state, descriptor)
        }
        WindowsCameraWatchRecord::Detached(descriptor) => {
            detached_watch_event(binding, &resource.event_state, descriptor)
        }
    };
    unsafe {
        out.write(event);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use super::{CAMERA_WATCH_QUEUE_CAPACITY, WindowsCameraWatchRecord, publish_watch_delta};
    use crate::platform::device::{CameraDeviceDescriptorValue, CameraFacingMode};
    use crate::runtime::control::queue::BoundedQueue;

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

    /// Ignore unchanged camera snapshots when publishing Windows watch deltas.
    #[test]
    fn test_publish_watch_delta_ignores_unchanged_snapshot() {
        let event_queue = Arc::new(BoundedQueue::new(CAMERA_WATCH_QUEUE_CAPACITY));
        let descriptor = test_camera_descriptor("camera.test", "Camera");
        let mut known_devices = BTreeMap::from([(descriptor.id.clone(), descriptor.clone())]);
        let snapshot = BTreeMap::from([(descriptor.id.clone(), descriptor)]);

        publish_watch_delta(&event_queue, &mut known_devices, &snapshot);

        let event = event_queue.try_pop_or_else(|| Err(()));
        assert!(event.is_err());
    }

    /// Emit one detach and attach pair when one Windows camera descriptor changes.
    #[test]
    fn test_publish_watch_delta_emits_replace_transition() {
        let event_queue = Arc::new(BoundedQueue::new(CAMERA_WATCH_QUEUE_CAPACITY));
        let previous = test_camera_descriptor("camera.test", "Camera");
        let current = test_camera_descriptor("camera.test", "Camera Updated");
        let mut known_devices = BTreeMap::from([(previous.id.clone(), previous.clone())]);
        let current_snapshot = BTreeMap::from([(current.id.clone(), current.clone())]);

        publish_watch_delta(&event_queue, &mut known_devices, &current_snapshot);

        let detached = event_queue.try_pop_or_else(|| Err(())).unwrap();
        let attached = event_queue.try_pop_or_else(|| Err(())).unwrap();

        assert!(matches!(detached, WindowsCameraWatchRecord::Detached(value) if value == previous));
        assert!(matches!(attached, WindowsCameraWatchRecord::Instance(value) if value == current));
    }
}
