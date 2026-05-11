use std::collections::HashMap;
use std::sync::Arc;

use windows::Devices::Geolocation::{
    GeolocationAccessStatus, Geolocator, Geoposition, PositionAccuracy, PositionChangedEventArgs,
    PositionStatus, StatusChangedEventArgs,
};
use windows::Foundation::{DateTime, TimeSpan, TypedEventHandler};
use windows::core::{Error as WindowsError, Ref};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::HostSessionId;
use crate::host::os::windows::ingress::notify::{
    windows_notify_location_sample, windows_notify_permission_result,
};
use crate::platform::core::{io_not_found, io_operation_error};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{LocationSampleValue, LocationWatchOptionsValue};
use crate::platform::os::{LocationAccuracy, Permission, PermissionState};
use crate::runtime::service::executor::thread::ServiceThreadExecutor;
use crate::runtime::service::registry::{global_service, global_service_if_initialized};
use crate::runtime::{ExecutionAffinity, ExecutionMode, ExecutionPolicy};

/// The location last-known maximum age.
const LOCATION_LAST_KNOWN_MAXIMUM_AGE_NS: u64 = 300_000_000_000;

/// The location last-known timeout.
const LOCATION_LAST_KNOWN_TIMEOUT_NS: u64 = 5_000_000_000;

/// The Windows epoch offset from 1601 to 1970 in 100ns ticks.
const WINDOWS_EPOCH_OFFSET_100NS: u64 = 116_444_736_000_000_000;

/// One process-global Windows location service.
pub(crate) struct WindowsLocationService {
    /// Dedicated WinRT executor for Geolocator work.
    executor: ServiceThreadExecutor<WindowsLocationServiceState>,
}

/// One host-owned Windows location service state.
struct WindowsLocationServiceState {
    /// Active runtime watches keyed by host runtime id.
    runtimes: HashMap<HostSessionId, WindowsLocationRuntime>,
}

/// Active Windows location state for one runtime.
#[derive(Default)]
struct WindowsLocationRuntime {
    /// Active runtime watches keyed by runtime-scoped watch id.
    watches: HashMap<String, ActiveLocationWatch>,
}

/// Active WinRT location watch registration.
struct ActiveLocationWatch {
    /// Native geolocator for the active watch.
    geolocator: Geolocator,
    /// Registered position callback token.
    position_changed_token: i64,
    /// Registered status callback token.
    status_changed_token: i64,
}

impl Drop for ActiveLocationWatch {
    /// Tear down one live WinRT location watch.
    fn drop(&mut self) {
        if let Err(error) = self
            .geolocator
            .RemovePositionChanged(self.position_changed_token)
        {
            tracing::warn!(
                target: "destack.runtime.host.windows.location",
                ?error,
                "failed to remove Windows location position callback",
            );
        }

        if let Err(error) = self
            .geolocator
            .RemoveStatusChanged(self.status_changed_token)
        {
            tracing::warn!(
                target: "destack.runtime.host.windows.location",
                ?error,
                "failed to remove Windows location status callback",
            );
        }
    }
}

impl WindowsLocationService {
    /// The execution policy for the Windows location service.
    pub(crate) const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Thread)
        .with_affinity(ExecutionAffinity::WindowsMta);

    /// Return whether host location services are currently enabled.
    pub(crate) fn location_services_enabled(&self, operation: &'static str) -> RuntimeResult<bool> {
        self.executor.call(operation, move |_state| {
            let geolocator = Geolocator::new()
                .map_err(|error| winrt_location_error(operation, "Geolocator::new", &error))?;
            let status = geolocator.LocationStatus().map_err(|error| {
                winrt_location_error(operation, "Geolocator::LocationStatus", &error)
            })?;

            Ok(location_services_enabled_for_status(status))
        })
    }

    /// Request one Windows location permission state.
    pub(crate) fn request_location_permission(
        &self,
        host_session_id: HostSessionId,
        permission: Permission,
        operation: &'static str,
    ) -> RuntimeResult<PermissionState> {
        self.executor.call(operation, move |_state| {
            let access_status = request_location_access(host_session_id, operation)?;
            let permission_state = permission_state_from_access_status(access_status);

            if permission == Permission::Location {
                return Ok(permission_state);
            }

            Ok(PermissionState::Denied)
        })
    }

    /// Read the most recent Windows location sample within host age and timeout bounds.
    pub(crate) fn read_last_known_location(
        &self,
        host_session_id: HostSessionId,
        operation: &'static str,
    ) -> RuntimeResult<LocationSampleValue> {
        self.executor.call(operation, move |_state| {
            ensure_location_access_allowed(host_session_id, operation)?;

            let geolocator = geolocator_for_options(None, operation)?;
            let geoposition = geolocator
                .GetGeopositionAsyncWithAgeAndTimeout(
                    timespan_from_ns(LOCATION_LAST_KNOWN_MAXIMUM_AGE_NS),
                    timespan_from_ns(LOCATION_LAST_KNOWN_TIMEOUT_NS),
                )
                .map_err(|error| {
                    winrt_location_error(
                        operation,
                        "Geolocator::GetGeopositionAsyncWithAgeAndTimeout",
                        &error,
                    )
                })?
                .get()
                .map_err(|error| winrt_location_error(operation, "IAsyncOperation::get", &error))?;

            location_sample_from_geoposition(&geoposition, false, operation)
        })
    }

    /// Open one live Windows location watch.
    pub(crate) fn open_location_watch(
        &self,
        host_session_id: HostSessionId,
        watch_id: String,
        options: LocationWatchOptionsValue,
        operation: &'static str,
    ) -> RuntimeResult<()> {
        self.executor.call(operation, move |state| {
            ensure_location_access_allowed(host_session_id, operation)?;

            let geolocator = geolocator_for_options(Some(&options), operation)?;
            let position_watch_id = watch_id.clone();
            let include_heading = options.include_heading;
            let position_changed_token = geolocator
                .PositionChanged(&TypedEventHandler::new(
                    move |_sender: Ref<'_, Geolocator>, args: Ref<'_, PositionChangedEventArgs>| {
                        let Some(args) = args.as_ref() else {
                            return Ok(());
                        };
                        let geoposition = args.Position()?;

                        match location_sample_from_geoposition(
                            &geoposition,
                            include_heading,
                            "destack.os.location.watchOpen",
                        ) {
                            Ok(sample) => {
                                if let Err(error) = windows_notify_location_sample(
                                    host_session_id.0,
                                    &position_watch_id,
                                    sample,
                                ) {
                                    tracing::warn!(
                                        target: "destack.runtime.host.windows.location",
                                        ?error,
                                        "failed to publish one Windows location sample",
                                    );
                                }
                            }
                            Err(error) => {
                                tracing::warn!(
                                    target: "destack.runtime.host.windows.location",
                                    ?error,
                                    "failed to decode one Windows location sample",
                                );
                            }
                        }

                        Ok(())
                    },
                ))
                .map_err(|error| {
                    winrt_location_error(operation, "Geolocator::PositionChanged", &error)
                })?;

            let status_changed_token = geolocator
                .StatusChanged(&TypedEventHandler::new(
                    move |_sender: Ref<'_, Geolocator>, args: Ref<'_, StatusChangedEventArgs>| {
                        let Some(args) = args.as_ref() else {
                            return Ok(());
                        };
                        let status = args.Status()?;
                        let is_granted = location_permission_granted_for_status(status);

                        if let Err(error) = windows_notify_permission_result(
                            host_session_id.0,
                            "location",
                            is_granted,
                        ) {
                            tracing::warn!(
                                target: "destack.runtime.host.windows.location",
                                ?error,
                                "failed to publish one Windows location permission update",
                            );
                        }

                        Ok(())
                    },
                ))
                .map_err(|error| {
                    winrt_location_error(operation, "Geolocator::StatusChanged", &error)
                })?;

            let runtime = state.runtimes.entry(host_session_id).or_default();

            if runtime.watches.contains_key(&watch_id) {
                return Err(io_operation_error(
                    operation,
                    Some(PlatformErrorCode::IoAlreadyExists),
                    format!("location watch `{watch_id}` is already open"),
                ));
            }

            runtime.watches.insert(
                watch_id.clone(),
                ActiveLocationWatch {
                    geolocator: geolocator.clone(),
                    position_changed_token,
                    status_changed_token,
                },
            );

            // publish one immediately available sample when the provider already has one
            if let Ok(geoposition) = geolocator
                .GetGeopositionAsyncWithAgeAndTimeout(
                    timespan_from_ns(LOCATION_LAST_KNOWN_MAXIMUM_AGE_NS),
                    timespan_from_ns(LOCATION_LAST_KNOWN_TIMEOUT_NS),
                )
                .and_then(|operation| operation.get())
                && let Ok(sample) =
                    location_sample_from_geoposition(&geoposition, include_heading, operation)
            {
                if let Err(error) =
                    windows_notify_location_sample(host_session_id.0, &watch_id, sample)
                {
                    tracing::warn!(
                        target: "destack.runtime.host.windows.location",
                        ?error,
                        "failed to publish one immediate Windows location sample",
                    );
                }
            }

            Ok(())
        })
    }

    /// Close one live Windows location watch.
    pub(crate) fn close_location_watch(
        &self,
        host_session_id: HostSessionId,
        watch_id: &str,
        operation: &'static str,
    ) -> RuntimeResult<()> {
        let watch_id = watch_id.to_string();

        self.executor.call(operation, move |state| {
            let Some(runtime) = state.runtimes.get_mut(&host_session_id) else {
                return Err(io_not_found(operation, "location runtime was not found"));
            };

            let removed = runtime.watches.remove(&watch_id);
            if removed.is_none() {
                return Err(io_not_found(operation, "location watch was not found"));
            }

            if runtime.watches.is_empty() {
                state.runtimes.remove(&host_session_id);
            }

            Ok(())
        })
    }

    /// Remove one runtime from the active Windows location backend.
    pub(crate) fn unregister_runtime(&self, host_session_id: HostSessionId) {
        if let Err(error) =
            self.executor
                .call("destack.host.windows.location.unregister", move |state| {
                    state.runtimes.remove(&host_session_id);
                    Ok(())
                })
        {
            tracing::warn!(
                target: "destack.runtime.host.windows.location",
                ?error,
                "failed to unregister Windows location runtime",
            );
        }
    }
}

impl WindowsLocationServiceState {
    /// Create one empty Windows location service state.
    fn new() -> Self {
        Self {
            runtimes: HashMap::new(),
        }
    }
}

/// Return the shared Windows location service.
pub(crate) fn windows_location_service(
    _operation: &'static str,
) -> RuntimeResult<Arc<WindowsLocationService>> {
    global_service(|| {
        let executor = ServiceThreadExecutor::spawn(
            "destack-windows-location",
            WindowsLocationService::POLICY,
            || Ok(WindowsLocationServiceState::new()),
        )?;

        Ok(WindowsLocationService { executor })
    })
}

/// Remove one runtime from the active Windows location backend.
pub(crate) fn unregister_location_runtime(host_session_id: HostSessionId) {
    if let Some(service) = global_service_if_initialized::<WindowsLocationService>() {
        service.unregister_runtime(host_session_id);
    }
}

/// Request Windows location access and publish the resulting permission state.
fn request_location_access(
    host_session_id: HostSessionId,
    operation: &'static str,
) -> RuntimeResult<GeolocationAccessStatus> {
    let access_status = Geolocator::RequestAccessAsync()
        .map_err(|error| winrt_location_error(operation, "Geolocator::RequestAccessAsync", &error))?
        .get()
        .map_err(|error| winrt_location_error(operation, "IAsyncOperation::get", &error))?;
    let permission_state = permission_state_from_access_status(access_status);

    windows_notify_permission_result(
        host_session_id.0,
        "location",
        permission_state == PermissionState::Granted,
    )?;

    Ok(access_status)
}

/// Require one granted Windows location permission.
fn ensure_location_access_allowed(
    host_session_id: HostSessionId,
    operation: &'static str,
) -> RuntimeResult<()> {
    let access_status = request_location_access(host_session_id, operation)?;

    if access_status == GeolocationAccessStatus::Allowed {
        return Ok(());
    }

    Err(io_operation_error(
        operation,
        Some(PlatformErrorCode::IoPermissionDenied),
        "windows location permission was denied",
    ))
}

/// Build one configured geolocator for one optional watch policy.
fn geolocator_for_options(
    options: Option<&LocationWatchOptionsValue>,
    operation: &'static str,
) -> RuntimeResult<Geolocator> {
    let geolocator = Geolocator::new()
        .map_err(|error| winrt_location_error(operation, "Geolocator::new", &error))?;

    if let Some(options) = options {
        let desired_accuracy = desired_accuracy_for_options(options);
        geolocator
            .SetDesiredAccuracy(desired_accuracy)
            .map_err(|error| {
                winrt_location_error(operation, "Geolocator::SetDesiredAccuracy", &error)
            })?;

        let movement_threshold = options.minimum_distance_meters.max(0.0);
        geolocator
            .SetMovementThreshold(movement_threshold)
            .map_err(|error| {
                winrt_location_error(operation, "Geolocator::SetMovementThreshold", &error)
            })?;

        let report_interval_ms = report_interval_ms(options.minimum_interval_ns);
        geolocator
            .SetReportInterval(report_interval_ms)
            .map_err(|error| {
                winrt_location_error(operation, "Geolocator::SetReportInterval", &error)
            })?;
    }

    Ok(geolocator)
}

/// Decode one runtime location sample from one WinRT geoposition payload.
fn location_sample_from_geoposition(
    geoposition: &Geoposition,
    include_heading: bool,
    operation: &'static str,
) -> RuntimeResult<LocationSampleValue> {
    let coordinate = geoposition
        .Coordinate()
        .map_err(|error| winrt_location_error(operation, "Geoposition::Coordinate", &error))?;
    let point = coordinate
        .Point()
        .map_err(|error| winrt_location_error(operation, "Geocoordinate::Point", &error))?;
    let position = point
        .Position()
        .map_err(|error| winrt_location_error(operation, "Geopoint::Position", &error))?;
    let timestamp = coordinate
        .Timestamp()
        .map_err(|error| winrt_location_error(operation, "Geocoordinate::Timestamp", &error))?;
    let altitude_meters = optional_finite(position.Altitude);
    let vertical_accuracy_meters = optional_f64_from_reference(coordinate.AltitudeAccuracy());
    let speed_meters_per_second = optional_f64_from_reference(coordinate.Speed());
    let heading_degrees = if include_heading {
        optional_f64_from_reference(coordinate.Heading())
    } else {
        None
    };

    Ok(LocationSampleValue {
        latitude_degrees: position.Latitude,
        longitude_degrees: position.Longitude,
        altitude_meters: altitude_meters.unwrap_or(f64::NAN),
        horizontal_accuracy_meters: coordinate
            .Accuracy()
            .map_err(|error| winrt_location_error(operation, "Geocoordinate::Accuracy", &error))?,
        vertical_accuracy_meters: vertical_accuracy_meters.unwrap_or(f64::NAN),
        speed_meters_per_second: speed_meters_per_second.unwrap_or(f64::NAN),
        heading_degrees: heading_degrees.unwrap_or(f64::NAN),
        timestamp_unix_ns: unix_ns_from_windows_datetime(timestamp),
    })
}

/// Map one WinRT access result into one runtime permission state.
fn permission_state_from_access_status(access_status: GeolocationAccessStatus) -> PermissionState {
    match access_status {
        GeolocationAccessStatus::Allowed => PermissionState::Granted,
        GeolocationAccessStatus::Denied | GeolocationAccessStatus::Unspecified => {
            PermissionState::Denied
        }
        _ => PermissionState::Denied,
    }
}

/// Return whether one Windows location status implies enabled services.
fn location_services_enabled_for_status(status: PositionStatus) -> bool {
    !matches!(
        status,
        PositionStatus::Disabled | PositionStatus::NotAvailable
    )
}

/// Return whether one Windows location status implies granted access.
fn location_permission_granted_for_status(status: PositionStatus) -> bool {
    matches!(
        status,
        PositionStatus::Ready | PositionStatus::Initializing | PositionStatus::NoData
    )
}

/// Map one abstract location accuracy into one Windows accuracy preference.
fn desired_accuracy_for_options(options: &LocationWatchOptionsValue) -> PositionAccuracy {
    match options.accuracy {
        LocationAccuracy::High | LocationAccuracy::Best => PositionAccuracy::High,
        LocationAccuracy::Passive | LocationAccuracy::Low | LocationAccuracy::Balanced => {
            PositionAccuracy::Default
        }
    }
}

/// Convert one nanosecond interval into one WinRT report interval in milliseconds.
fn report_interval_ms(interval_ns: u64) -> u32 {
    if interval_ns == 0 {
        return 0;
    }

    let interval_ms = interval_ns.saturating_add(999_999) / 1_000_000;

    interval_ms.max(1).min(u32::MAX as u64) as u32
}

/// Convert one nanosecond duration into one WinRT `TimeSpan`.
fn timespan_from_ns(duration_ns: u64) -> TimeSpan {
    let duration_100ns = duration_ns / 100;
    let duration_100ns = duration_100ns.min(i64::MAX as u64) as i64;

    TimeSpan {
        Duration: duration_100ns,
    }
}

/// Convert one optional WinRT numeric reference into one runtime scalar.
fn optional_f64_from_reference(
    reference: windows::core::Result<windows::Foundation::IReference<f64>>,
) -> Option<f64> {
    reference
        .ok()
        .and_then(|value| value.Value().ok())
        .filter(|value| value.is_finite())
}

/// Convert one required numeric scalar into one optional runtime scalar.
fn optional_finite(value: f64) -> Option<f64> {
    value.is_finite().then_some(value)
}

/// Convert one WinRT `DateTime` into unix nanoseconds.
fn unix_ns_from_windows_datetime(datetime: DateTime) -> u64 {
    if datetime.UniversalTime <= 0 {
        return 0;
    }

    let ticks_100ns = datetime.UniversalTime as u64;
    if ticks_100ns < WINDOWS_EPOCH_OFFSET_100NS {
        return 0;
    }

    ticks_100ns
        .saturating_sub(WINDOWS_EPOCH_OFFSET_100NS)
        .saturating_mul(100)
}

/// Map one WinRT location error into one runtime IO error.
fn winrt_location_error(
    operation: &'static str,
    stage: &str,
    error: &WindowsError,
) -> Box<RuntimeError> {
    match error.code().0 as u32 {
        0x80070490 => io_not_found(operation, "windows location data was not found"),
        0x800704C7 => io_operation_error(
            operation,
            Some(PlatformErrorCode::IoWouldBlock),
            "windows location request was cancelled",
        ),
        0x80070005 => io_operation_error(
            operation,
            Some(PlatformErrorCode::IoPermissionDenied),
            "windows location access was denied",
        ),
        _ => io_operation_error(operation, None, format!("{stage} failed: {error}")),
    }
}
