use std::sync::Arc;

use crate::host::core::registry::{HostRegistrationGuard, next_host_runtime_id};
use crate::host::core::{HostQueue, HostRuntimeRegistry};
use crate::host::ios::{
    IosHostBindings, IosHostLocationCallbacks, destack_host_ios_location_last_known,
    destack_host_ios_location_services_enabled, destack_host_ios_location_watch_close,
    destack_host_ios_location_watch_open, destack_host_ios_register_bindings,
    unregister_ios_bindings,
};
use crate::host::{
    HOST_STATUS_INVALID_ARGUMENT, HOST_STATUS_NOT_FOUND, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK,
    Platform,
};
use crate::platform::os::{LocationAccuracy, LocationSample, LocationWatchOptions};
use crate::runtime::NativeStringRef;

/// Register one temporary iOS host queue and keep registration state alive.
fn register_ios_runtime() -> (Arc<HostQueue>, HostRegistrationGuard, u64) {
    let runtime_id = next_host_runtime_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration = HostRuntimeRegistry::register_queue(
        Platform::IOS,
        runtime_id,
        Arc::clone(&queue),
        Some(unregister_ios_bindings),
    );
    let runtime_id = registration.host_runtime_id().0;

    (queue, registration, runtime_id)
}

/// Register one runtime-scoped iOS host bindings payload.
fn register_ios_bindings(runtime_id: u64, bindings: IosHostBindings) -> u32 {
    unsafe { destack_host_ios_register_bindings(runtime_id, bindings) }
}

unsafe extern "C" fn test_services_enabled_callback(
    _runtime_id: u64,
    is_enabled: *mut bool,
) -> u32 {
    let is_enabled = unsafe { &mut *is_enabled };
    *is_enabled = true;
    HOST_STATUS_OK
}

unsafe extern "C" fn test_last_known_callback(
    _runtime_id: u64,
    sample: *mut LocationSample,
) -> u32 {
    let sample = unsafe { &mut *sample };
    *sample = test_location_sample();
    HOST_STATUS_OK
}

unsafe extern "C" fn test_watch_open_callback(
    _runtime_id: u64,
    watch_id: NativeStringRef,
    options: LocationWatchOptions,
) -> u32 {
    let watch_id = unsafe { watch_id.as_str() }.unwrap();
    assert_eq!(watch_id, "watch-1");
    assert_eq!(options, test_watch_options());
    HOST_STATUS_OK
}

unsafe extern "C" fn test_watch_close_callback(_runtime_id: u64, watch_id: NativeStringRef) -> u32 {
    let watch_id = unsafe { watch_id.as_str() }.unwrap();
    assert_eq!(watch_id, "watch-1");
    HOST_STATUS_OK
}

#[test]
fn test_location_callbacks_report_missing_runtime_registration() {
    let runtime_id = next_host_runtime_id().0;
    let status =
        unsafe { destack_host_ios_location_services_enabled(runtime_id, std::ptr::null_mut()) };
    assert_eq!(status, HOST_STATUS_INVALID_ARGUMENT);

    let mut enabled = false;
    let status = unsafe { destack_host_ios_location_services_enabled(runtime_id, &mut enabled) };
    assert_eq!(status, HOST_STATUS_NOT_FOUND);
}

#[test]
fn test_location_callbacks_report_unsupported_without_registered_handler() {
    let (_queue, _registration, runtime_id) = register_ios_runtime();
    let status = register_ios_bindings(runtime_id, IosHostBindings::default());
    assert_eq!(status, HOST_STATUS_OK);

    let mut enabled = false;
    let status = unsafe { destack_host_ios_location_services_enabled(runtime_id, &mut enabled) };
    assert_eq!(status, HOST_STATUS_NOT_SUPPORTED);
}

#[test]
fn test_location_callbacks_route_registered_handlers() {
    let (_queue, _registration, runtime_id) = register_ios_runtime();
    let status = register_ios_bindings(
        runtime_id,
        IosHostBindings {
            location: IosHostLocationCallbacks {
                services_enabled: Some(test_services_enabled_callback),
                last_known: Some(test_last_known_callback),
                watch_open: Some(test_watch_open_callback),
                watch_close: Some(test_watch_close_callback),
            },
            ..IosHostBindings::default()
        },
    );
    assert_eq!(status, HOST_STATUS_OK);

    let mut enabled = false;
    let services_status =
        unsafe { destack_host_ios_location_services_enabled(runtime_id, &mut enabled) };
    assert_eq!(services_status, HOST_STATUS_OK);
    assert!(enabled);

    let mut sample = test_location_sample();
    let last_known_status =
        unsafe { destack_host_ios_location_last_known(runtime_id, &mut sample) };
    assert_eq!(last_known_status, HOST_STATUS_OK);
    assert_eq!(sample, test_location_sample());

    let watch_open_status = unsafe {
        destack_host_ios_location_watch_open(
            runtime_id,
            NativeStringRef::from("watch-1"),
            test_watch_options(),
        )
    };
    assert_eq!(watch_open_status, HOST_STATUS_OK);

    let watch_close_status = unsafe {
        destack_host_ios_location_watch_close(runtime_id, NativeStringRef::from("watch-1"))
    };
    assert_eq!(watch_close_status, HOST_STATUS_OK);
}

fn test_watch_options() -> LocationWatchOptions {
    LocationWatchOptions {
        accuracy: LocationAccuracy::Best,
        minimum_interval_ns: 1_000_000_000,
        minimum_distance_meters: 5.0,
        include_heading: true,
    }
}

fn test_location_sample() -> LocationSample {
    LocationSample {
        latitude_degrees: 47.0,
        longitude_degrees: 8.0,
        altitude_meters: 400.0,
        horizontal_accuracy_meters: 5.0,
        vertical_accuracy_meters: 10.0,
        speed_meters_per_second: 0.0,
        heading_degrees: 0.0,
        timestamp_unix_ns: 42,
    }
}
