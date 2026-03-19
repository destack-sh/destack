use std::cell::RefCell;
use std::collections::HashMap;

use objc2::DefinedClass;
use objc2::rc::Retained;
use objc2_core_location::{
    CLLocationManager, kCLDistanceFilterNone, kCLLocationAccuracyBest,
    kCLLocationAccuracyHundredMeters, kCLLocationAccuracyKilometer,
    kCLLocationAccuracyNearestTenMeters,
};

use crate::diagnostic::RuntimeResult;
use crate::host::apple::execution::with_process_main_context_marker_if_needed;
use crate::host::core::HostRuntimeId;
use crate::host::macos::macos_notify_location_sample;
use crate::platform::core::{io_not_found, io_operation_error};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::LocationAccuracy;
use crate::platform::os::abi_generated::{LocationSampleValue, LocationWatchOptionsValue};

use super::core::{
    LOCATION_LAST_KNOWN_OPERATION, LOCATION_WATCH_CLOSE_OPERATION, LOCATION_WATCH_OPEN_OPERATION,
};
use super::delegate::MacosLocationManagerDelegate;
use super::permission::{LocationPermissionRequest, ensure_location_authorized};
use super::sample::location_sample_value_from_native;

/// Runtime-owned macOS location watches for one process main thread.
#[derive(Default)]
struct MacosLocationService {
    /// Active location runtimes keyed by host runtime id.
    runtimes: HashMap<HostRuntimeId, MacosLocationRuntime>,
}

/// Runtime-owned macOS location watches for one host runtime.
#[derive(Default)]
struct MacosLocationRuntime {
    /// Active location watches keyed by runtime-scoped watch id.
    watches: HashMap<String, ActiveLocationWatch>,
}

/// Active Core Location watch registration.
struct ActiveLocationWatch {
    /// Native manager that owns the active host subscription.
    manager: Retained<CLLocationManager>,
    /// Retained delegate for native callback delivery.
    delegate: Retained<MacosLocationManagerDelegate>,
}

thread_local! {
    /// Main-thread-local Core Location watch registry.
    static MACOS_LOCATION_SERVICE: RefCell<MacosLocationService> =
        RefCell::new(MacosLocationService::default());
}

/// Remove one macOS runtime from the active location backend.
pub(super) fn unregister_location_runtime_on_main(host_runtime_id: HostRuntimeId) {
    if let Err(error) = with_process_main_context_marker_if_needed(move |_| {
        with_location_service(|service| {
            let Some(runtime) = service.runtimes.remove(&host_runtime_id) else {
                return Ok(());
            };

            // stop every live watch before dropping the runtime slot
            for (_, watch) in runtime.watches {
                stop_active_watch(watch);
            }

            Ok(())
        })
    }) {
        tracing::warn!(
            target: "destack.runtime.host.macos.location",
            ?error,
            "failed to unregister macOS location runtime",
        );
    }
}

/// Read whether Core Location services are globally enabled.
pub(super) fn location_services_enabled() -> RuntimeResult<bool> {
    with_process_main_context_marker_if_needed(|_| {
        let is_enabled = unsafe { CLLocationManager::locationServicesEnabled_class() };

        Ok(is_enabled)
    })
}

/// Read the most recent Core Location sample.
pub(super) fn read_last_known_location(
    host_runtime_id: HostRuntimeId,
) -> RuntimeResult<LocationSampleValue> {
    with_process_main_context_marker_if_needed(move |mtm| {
        let manager = unsafe { CLLocationManager::new() };
        ensure_location_authorized(
            &manager,
            host_runtime_id,
            LocationPermissionRequest::WhenInUse,
            mtm,
            LOCATION_LAST_KNOWN_OPERATION,
        )?;

        // read the most recent sample directly from Core Location
        let Some(location) = (unsafe { manager.location() }) else {
            return Err(io_not_found(
                LOCATION_LAST_KNOWN_OPERATION,
                "CoreLocation has no location sample for this runtime",
            ));
        };

        Ok(location_sample_value_from_native(location.as_ref(), None))
    })
}

/// Open one live Core Location watch for one runtime.
pub(super) fn open_location_watch(
    host_runtime_id: HostRuntimeId,
    watch_id: &str,
    options: &LocationWatchOptionsValue,
) -> RuntimeResult<()> {
    let watch_id = watch_id.to_string();
    let options = *options;

    // reject duplicate watch identifiers before starting native updates
    with_location_service(|service| {
        let Some(runtime) = service.runtimes.get(&host_runtime_id) else {
            return Ok(());
        };

        if runtime.watches.contains_key(&watch_id) {
            return Err(io_operation_error(
                LOCATION_WATCH_OPEN_OPERATION,
                Some(PlatformErrorCode::IoAlreadyExists),
                format!("location watch `{watch_id}` is already open"),
            ));
        }

        Ok(())
    })?;

    with_process_main_context_marker_if_needed(move |mtm| {
        let manager = unsafe { CLLocationManager::new() };
        let authorization_status = ensure_location_authorized(
            &manager,
            host_runtime_id,
            LocationPermissionRequest::WhenInUse,
            mtm,
            LOCATION_WATCH_OPEN_OPERATION,
        )?;
        let delegate = MacosLocationManagerDelegate::new(
            mtm,
            host_runtime_id,
            Some(watch_id.clone()),
            options.include_heading,
            authorization_status,
        );

        // configure one concrete native manager before starting updates
        configure_location_manager(&manager, &options);

        unsafe {
            manager.setDelegate(Some(delegate.as_protocol()));
            manager.startUpdatingLocation();

            if options.include_heading && CLLocationManager::headingAvailable_class() {
                manager.startUpdatingHeading();
            }
        }

        // publish the current sample immediately so the watch starts hot when possible
        if let Some(location) = unsafe { manager.location() } {
            let sample = location_sample_value_from_native(location.as_ref(), None);
            macos_notify_location_sample(host_runtime_id.0, &watch_id, sample)?;
        }

        with_location_service(|service| {
            let runtime = service.runtimes.entry(host_runtime_id).or_default();
            runtime
                .watches
                .insert(watch_id, ActiveLocationWatch { manager, delegate });

            Ok(())
        })
    })
}

/// Close one live Core Location watch for one runtime.
pub(super) fn close_location_watch(
    host_runtime_id: HostRuntimeId,
    watch_id: &str,
) -> RuntimeResult<()> {
    let watch_id = watch_id.to_string();

    with_process_main_context_marker_if_needed(move |_| {
        let removed_watch = with_location_service(|service| {
            let Some(runtime) = service.runtimes.get_mut(&host_runtime_id) else {
                return Ok(None);
            };
            let removed_watch = runtime.watches.remove(watch_id.as_str());

            if runtime.watches.is_empty() {
                service.runtimes.remove(&host_runtime_id);
            }

            Ok(removed_watch)
        })?;

        let Some(removed_watch) = removed_watch else {
            return Err(io_not_found(
                LOCATION_WATCH_CLOSE_OPERATION,
                format!("unknown macOS location watch id: {watch_id}"),
            ));
        };

        stop_active_watch(removed_watch);

        Ok(())
    })
}

/// Configure one native Core Location manager from runtime watch options.
fn configure_location_manager(manager: &CLLocationManager, options: &LocationWatchOptionsValue) {
    let desired_accuracy = desired_accuracy_for_location_watch(options.accuracy);
    let distance_filter = distance_filter_for_location_watch(options.minimum_distance_meters);

    unsafe {
        manager.setDesiredAccuracy(desired_accuracy);
        manager.setDistanceFilter(distance_filter);
        manager.setPausesLocationUpdatesAutomatically(false);
    }
}

/// Return the desired Core Location accuracy for one runtime accuracy selector.
fn desired_accuracy_for_location_watch(accuracy: LocationAccuracy) -> f64 {
    unsafe {
        match accuracy {
            LocationAccuracy::Passive => kCLLocationAccuracyKilometer,
            LocationAccuracy::Low => kCLLocationAccuracyKilometer,
            LocationAccuracy::Balanced => kCLLocationAccuracyHundredMeters,
            LocationAccuracy::High => kCLLocationAccuracyNearestTenMeters,
            LocationAccuracy::Best => kCLLocationAccuracyBest,
        }
    }
}

/// Return the distance filter for one runtime watch option payload.
fn distance_filter_for_location_watch(minimum_distance_meters: f64) -> f64 {
    if minimum_distance_meters <= 0.0 {
        return unsafe { kCLDistanceFilterNone };
    }

    minimum_distance_meters
}

/// Stop one active Core Location watch registration.
fn stop_active_watch(watch: ActiveLocationWatch) {
    unsafe {
        if watch.delegate.ivars().include_heading {
            watch.manager.stopUpdatingHeading();
        }

        watch.manager.stopUpdatingLocation();
        watch.manager.setDelegate(None);
    }
}

/// Execute one callback with the main-thread-local location service.
fn with_location_service<R>(
    callback: impl FnOnce(&mut MacosLocationService) -> RuntimeResult<R>,
) -> RuntimeResult<R> {
    MACOS_LOCATION_SERVICE.with(|service| callback(&mut service.borrow_mut()))
}
