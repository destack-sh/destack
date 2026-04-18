use std::sync::Arc;

use super::core::*;
use super::device::camera_descriptor_snapshot;
use super::service::{
    CAMERA_WATCH_QUEUE_CAPACITY, LinuxCameraWatchRecord, LinuxCameraWatchResource,
    LinuxCameraWatchService,
};
use crate::runtime::control::queue::BoundedQueue;

/// Finalizer for one opened Linux camera watch.
struct LinuxCameraWatchFinalizer {
    /// The shared Linux camera topology service.
    service: Arc<LinuxCameraWatchService>,
    /// The registered watch identifier.
    registration_id: u64,
    /// The shared event queue.
    event_queue: Arc<BoundedQueue<LinuxCameraWatchRecord>>,
}

impl ResourceFinalizer for LinuxCameraWatchFinalizer {
    /// Unregister the watch and wake blocked consumers.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        self.service.unregister_watch(self.registration_id);
        self.event_queue.close();
    }
}

/// Resolve one typed Linux camera watch resource from the resource table.
fn camera_watch_resource(
    binding: &BindingCallContext,
    handle: resource::CameraWatchHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<LinuxCameraWatchResource>> {
    camera_watch_payload(binding, handle, operation)
}

/// Open one Linux camera topology watch stream.
pub(crate) unsafe fn destack_device_camera_device_watch_open(
    binding: &BindingCallContext,
    out: *mut resource::CameraWatchHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // capture the initial topology snapshot before registering the watch
    let snapshot = camera_descriptor_snapshot("destack.device.camera.device.watchOpen")?;
    let event_queue = Arc::new(BoundedQueue::new(CAMERA_WATCH_QUEUE_CAPACITY));
    let resource = Arc::new(LinuxCameraWatchResource {
        event_queue: event_queue.clone(),
        event_state: Mutex::new(CameraWatchEventState { next_sequence: 1 }),
    });

    // seed the queue with the current topology snapshot
    for descriptor in snapshot.values() {
        resource
            .event_queue
            .push_drop_oldest(LinuxCameraWatchRecord::Instance(descriptor.clone()));
    }

    // register the opened watch with the shared topology service
    let service = binding
        .worker()
        .platform_state
        .device
        .linux_camera_watch_service("destack.device.camera.device.watchOpen")?;
    let registration_id = service.register_watch(&resource, snapshot)?;

    // store the opened watch resource
    let entry = ResourceEntry::new(ResourceKind::CameraWatch)
        .with_label(CAMERA_WATCH_RESOURCE_LABEL)
        .with_payload(resource)
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            LinuxCameraWatchFinalizer {
                service,
                registration_id,
                event_queue,
            },
        ));
    let handle = binding
        .worker()
        .resources
        .insert(&binding.world(), entry, Some(binding.engine()));

    unsafe {
        out.write(resource::CameraWatchHandle(handle));
    }

    Ok(())
}

/// Close one Linux camera topology watch stream.
pub(crate) unsafe fn destack_device_camera_device_watch_close(
    binding: &BindingCallContext,
    handle: resource::CameraWatchHandle,
) -> RuntimeResult<()> {
    close_camera_watch_resource(binding, handle, "destack.device.camera.device.watchClose")
}

/// Read one blocking Linux camera topology event.
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
        LinuxCameraWatchRecord::Instance(descriptor) => {
            attached_watch_event(binding, &resource.event_state, descriptor)
        }
        LinuxCameraWatchRecord::Detached(descriptor) => {
            detached_watch_event(binding, &resource.event_state, descriptor)
        }
    };
    unsafe {
        out.write(event);
    }

    Ok(())
}

/// Poll one Linux camera topology event without blocking.
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
        LinuxCameraWatchRecord::Instance(descriptor) => {
            attached_watch_event(binding, &resource.event_state, descriptor)
        }
        LinuxCameraWatchRecord::Detached(descriptor) => {
            detached_watch_event(binding, &resource.event_state, descriptor)
        }
    };
    unsafe {
        out.write(event);
    }

    Ok(())
}
