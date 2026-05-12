use std::collections::BTreeMap;
use std::sync::{Arc, Weak};

use parking_lot::Mutex;

use super::codec::read_camera_device_descriptors;
use super::core::*;
use crate::platform::core::BoundedQueue;
use crate::runtime::{WorkerCallbackControl, WorkerCallbackHandle};

/// One camera-watch event queue capacity.
const CAMERA_WATCH_QUEUE_CAPACITY: usize = 64;

/// One synthetic Android camera-watch poll interval.
const CAMERA_WATCH_POLL_INTERVAL_NS: u64 = 500_000_000;

/// One queued Android camera topology event.
#[derive(Debug, Clone)]
enum AndroidCameraWatchRecord {
    /// One attached camera endpoint.
    Instance(CameraDeviceDescriptorValue),
    /// One detached camera endpoint.
    Detached(CameraDeviceDescriptorValue),
}

/// One opened Android camera-watch resource.
struct AndroidCameraWatchResource {
    /// The shared camera-watch state.
    state: Arc<Mutex<AndroidCameraWatchState>>,
}

/// Mutable Android camera-watch state.
struct AndroidCameraWatchState {
    /// The queued camera events.
    event_queue: Arc<BoundedQueue<AndroidCameraWatchRecord>>,
    /// The per-stream sequence state.
    event_state: Mutex<CameraWatchEventState>,
    /// The last known camera snapshot.
    known_devices: BTreeMap<String, CameraDeviceDescriptorValue>,
    /// The active worker callback when registered.
    callback_handle: Option<WorkerCallbackHandle>,
    /// Whether the watch has begun teardown.
    is_closed: bool,
}

/// Finalizer for one Android camera watch.
struct AndroidCameraWatchFinalizer {
    /// The shared camera-watch state.
    state: Arc<Mutex<AndroidCameraWatchState>>,
}

impl ResourceFinalizer for AndroidCameraWatchFinalizer {
    /// Mark the watch closed and wake blocked consumers.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        let mut state = self.state.lock();
        state.is_closed = true;
        state.event_queue.close();
    }
}

impl AndroidCameraWatchState {
    /// Queue topology events for the current camera snapshot.
    fn refresh(&mut self, devices: Vec<CameraDeviceDescriptorValue>) {
        let mut current_snapshot = BTreeMap::new();

        for descriptor in devices {
            current_snapshot.insert(descriptor.id.clone(), descriptor);
        }

        // attached or replaced
        for (device_id, descriptor) in &current_snapshot {
            match self.known_devices.get(device_id) {
                Some(previous) if previous == descriptor => {}
                Some(previous) => {
                    self.event_queue
                        .push_drop_oldest(AndroidCameraWatchRecord::Detached(previous.clone()));
                    self.event_queue
                        .push_drop_oldest(AndroidCameraWatchRecord::Instance(descriptor.clone()));
                }
                None => {
                    self.event_queue
                        .push_drop_oldest(AndroidCameraWatchRecord::Instance(descriptor.clone()));
                }
            }
        }

        // detached
        for (device_id, previous) in &self.known_devices {
            if current_snapshot.contains_key(device_id) {
                continue;
            }

            self.event_queue
                .push_drop_oldest(AndroidCameraWatchRecord::Detached(previous.clone()));
        }

        self.known_devices = current_snapshot;
    }
}

/// Resolve one typed Android camera-watch resource from the resource table.
fn camera_watch_resource(
    binding: &BindingCallContext,
    handle: resource::CameraWatchHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<AndroidCameraWatchResource>> {
    camera_watch_payload(binding, handle, operation)
}

/// Register one synthetic poll callback for the Android camera watch.
fn register_camera_watch_callback(
    binding: &BindingCallContext,
    state: &Arc<Mutex<AndroidCameraWatchState>>,
) -> RuntimeResult<WorkerCallbackHandle> {
    let state = Arc::downgrade(state);

    binding.schedule_worker_callback(
        CAMERA_WATCH_POLL_INTERVAL_NS,
        Some(CAMERA_WATCH_POLL_INTERVAL_NS),
        move |binding| poll_camera_watch(binding, &state),
    )
}

/// Refresh one Android camera watch from the current host snapshot.
fn poll_camera_watch(
    binding: &BindingCallContext,
    state: &Weak<Mutex<AndroidCameraWatchState>>,
) -> RuntimeResult<WorkerCallbackControl> {
    let Some(state) = state.upgrade() else {
        return Ok(WorkerCallbackControl::Cancel);
    };
    let mut state = state.lock();
    if state.is_closed {
        return Ok(WorkerCallbackControl::Cancel);
    }

    // synthetic topology refresh
    let devices = match read_camera_device_descriptors(
        binding,
        "destack.device.camera.device.watch.syntheticPoll",
    ) {
        Ok(devices) => devices,
        Err(_) => {
            state.is_closed = true;
            state.event_queue.close();
            return Ok(WorkerCallbackControl::Cancel);
        }
    };
    state.refresh(devices);

    Ok(WorkerCallbackControl::Keep)
}

/// Open one Android camera topology watch stream.
pub(crate) unsafe fn destack_device_camera_device_watch_open(
    binding: &BindingCallContext,
    out: *mut resource::CameraWatchHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // capture the initial topology snapshot before starting the synthetic watch
    let devices =
        read_camera_device_descriptors(binding, "destack.device.camera.device.watchOpen")?;
    let event_queue = Arc::new(BoundedQueue::new(CAMERA_WATCH_QUEUE_CAPACITY));
    let state = Arc::new(Mutex::new(AndroidCameraWatchState {
        event_queue: event_queue.clone(),
        event_state: Mutex::new(CameraWatchEventState { next_sequence: 1 }),
        known_devices: BTreeMap::new(),
        callback_handle: None,
        is_closed: false,
    }));

    // seed the queue with the current topology snapshot
    state.lock().refresh(devices);

    // register the synthetic poll callback on the owning runtime thread
    let callback_handle = register_camera_watch_callback(binding, &state)?;
    state.lock().callback_handle = Some(callback_handle);

    // store the opened camera watch
    let entry = ResourceEntry::new(ResourceKind::CameraWatch)
        .with_label(CAMERA_WATCH_RESOURCE_LABEL)
        .with_payload(Arc::new(AndroidCameraWatchResource {
            state: state.clone(),
        }))
        .with_finalizer(
            binding
                .worker()
                .platform_state
                .device
                .wrap_finalizer(AndroidCameraWatchFinalizer { state }),
        );
    let handle = binding
        .worker()
        .resources
        .insert(binding.world(), entry, Some(binding.engine()));

    unsafe {
        out.write(resource::CameraWatchHandle(handle));
    }

    Ok(())
}

/// Close one Android camera topology watch stream.
pub(crate) unsafe fn destack_device_camera_device_watch_close(
    binding: &BindingCallContext,
    handle: resource::CameraWatchHandle,
) -> RuntimeResult<()> {
    if let Ok(resource) =
        camera_watch_resource(binding, handle, "destack.device.camera.device.watchClose")
    {
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

    close_camera_watch_resource(binding, handle, "destack.device.camera.device.watchClose")
}

/// Read one blocking Android camera topology event.
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
    let event_queue = resource.state.lock().event_queue.clone();
    let timeout = std::time::Duration::from_nanos(timeoutns.max(1));
    let event = event_queue.pop_with_timeout_or_else(timeout, || {
        Err(core_platform::io_would_block(
            "destack.device.camera.device.watchRead",
            "camera watch event queue is empty",
        ))
    })?;

    // encode the event on the binding heap
    let resource_state = resource.state.lock();
    let event = match event {
        AndroidCameraWatchRecord::Instance(descriptor) => {
            attached_watch_event(binding, &resource_state.event_state, descriptor)
        }
        AndroidCameraWatchRecord::Detached(descriptor) => {
            detached_watch_event(binding, &resource_state.event_state, descriptor)
        }
    };
    unsafe {
        out.write(event);
    }

    Ok(())
}

/// Poll one Android camera topology event without blocking.
pub(crate) unsafe fn destack_device_camera_device_watch_try_read(
    binding: &BindingCallContext,
    out: *mut CameraWatchEvent,
    handle: resource::CameraWatchHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // read the next queued topology event
    let resource =
        camera_watch_resource(binding, handle, "destack.device.camera.device.watchTryRead")?;
    let event_queue = resource.state.lock().event_queue.clone();
    let event = event_queue.try_pop_or_else(|| {
        Err(core_platform::io_would_block(
            "destack.device.camera.device.watchTryRead",
            "camera watch event queue is empty",
        ))
    })?;

    // encode the event on the binding heap
    let resource_state = resource.state.lock();
    let event = match event {
        AndroidCameraWatchRecord::Instance(descriptor) => {
            attached_watch_event(binding, &resource_state.event_state, descriptor)
        }
        AndroidCameraWatchRecord::Detached(descriptor) => {
            detached_watch_event(binding, &resource_state.event_state, descriptor)
        }
    };
    unsafe {
        out.write(event);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use parking_lot::Mutex;

    use super::{AndroidCameraWatchState, CAMERA_WATCH_QUEUE_CAPACITY};
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

    /// Ignore unchanged Android camera snapshots when refreshing one watch.
    #[test]
    fn test_refresh_ignores_unchanged_camera_snapshots() {
        let event_queue = Arc::new(BoundedQueue::new(CAMERA_WATCH_QUEUE_CAPACITY));
        let descriptor = test_camera_descriptor("camera.test", "Camera");
        let mut state = AndroidCameraWatchState {
            event_queue: event_queue.clone(),
            event_state: Mutex::new(super::CameraWatchEventState { next_sequence: 1 }),
            known_devices: std::collections::BTreeMap::from([(
                descriptor.id.clone(),
                descriptor.clone(),
            )]),
            callback_handle: None,
            is_closed: false,
        };

        state.refresh(vec![descriptor]);

        let event = event_queue.try_pop_or_else(|| Err(()));
        assert!(event.is_err());
    }
}
