mod core;
mod device;
mod stream;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub(crate) use device::publish_device_snapshot_native_if_service_live;
pub(crate) use device::{
    publish_device_events_from_snapshot, refresh_device_events_for_stream,
    refresh_device_subscriptions_for_rescan,
};
pub(crate) use stream::{publish_stream_event_native, refresh_stream_events_for_stream};
