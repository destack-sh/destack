use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{RecvTimeoutError, sync_channel};
use std::sync::{Arc, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use zbus::Error as ZbusError;
use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::OwnedObjectPath;

use crate::diagnostic::RuntimeResult;
use crate::host::app::identity::resolved_application_identifier;
use crate::host::core::{
    HostRequest, HostRequestContext, HostRequestOutcome, HostRequestResult, HostRuntimeId,
};
use crate::host::linux::linux_notify_location_sample;
use crate::platform::core::{io_not_found, io_operation_error};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    LocationAccuracy, LocationSampleValue, LocationWatchOptionsValue,
};
use crate::runtime::capability::{PlatformCapability, PlatformCapabilitySet};

/// The Linux location services-enabled operation name.
const LOCATION_SERVICES_ENABLED_OPERATION: &str = "destack.os.location.servicesEnabled";

/// The Linux location last-known operation name.
const LOCATION_LAST_KNOWN_OPERATION: &str = "destack.os.location.lastKnown";

/// The Linux location watch-open operation name.
const LOCATION_WATCH_OPEN_OPERATION: &str = "destack.os.location.watchOpen";

/// The Linux location watch-close operation name.
const LOCATION_WATCH_CLOSE_OPERATION: &str = "destack.os.location.watchClose";

/// The GeoClue bus name.
const GEOCLUE_BUS_NAME: &str = "org.freedesktop.GeoClue2";

/// The GeoClue manager object path.
const GEOCLUE_MANAGER_PATH: &str = "/org/freedesktop/GeoClue2/Manager";

/// The GeoClue manager interface name.
const GEOCLUE_MANAGER_INTERFACE: &str = "org.freedesktop.GeoClue2.Manager";

/// The GeoClue client interface name.
const GEOCLUE_CLIENT_INTERFACE: &str = "org.freedesktop.GeoClue2.Client";

/// The GeoClue location interface name.
const GEOCLUE_LOCATION_INTERFACE: &str = "org.freedesktop.GeoClue2.Location";

/// The maximum wait for one initial GeoClue watch start.
const GEOCLUE_START_TIMEOUT: Duration = Duration::from_secs(10);

/// The maximum wait for one last-known location sample.
const GEOCLUE_LAST_KNOWN_TIMEOUT: Duration = Duration::from_secs(15);

/// The sleep slice used while polling GeoClue location updates.
const GEOCLUE_POLL_SLICE: Duration = Duration::from_millis(250);

/// One active Linux location watch.
struct LinuxLocationWatch {
    /// Shared stop flag for the watch worker.
    stop: Arc<AtomicBool>,
    /// Worker thread that owns the active GeoClue client.
    join_handle: JoinHandle<()>,
}

/// One runtime-scoped Linux location watch registry.
#[derive(Default)]
struct LinuxLocationService {
    /// Active watches keyed by runtime id and watch id.
    watches: HashMap<HostRuntimeId, HashMap<String, LinuxLocationWatch>>,
}

/// One active GeoClue client session.
struct GeoClueClient {
    /// System bus connection for GeoClue calls.
    connection: Connection,
    /// Object path for this client.
    client_path: OwnedObjectPath,
}

/// Return the process-global Linux location service.
fn linux_location_service() -> &'static Mutex<LinuxLocationService> {
    static SERVICE: OnceLock<Mutex<LinuxLocationService>> = OnceLock::new();

    SERVICE.get_or_init(|| Mutex::new(LinuxLocationService::default()))
}

/// Return dynamic Unix location capabilities for Linux hosts.
pub(crate) fn request_capabilities() -> PlatformCapabilitySet {
    let mut capabilities = PlatformCapabilitySet::default();

    // expose location reads and watches only when GeoClue is reachable
    if geoclue_is_available() {
        capabilities.extend_capabilities([
            PlatformCapability::OsLocationRead,
            PlatformCapability::OsLocationWatch,
        ]);
    }

    capabilities
}

/// Submit one Linux location request through GeoClue.
pub(crate) fn submit_location_request(
    context: &HostRequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        // report whether GeoClue is reachable on this host
        HostRequest::OsLocationServicesEnabled => Ok(Some(HostRequestOutcome::immediate(
            HostRequestResult::Bool(location_services_enabled()?),
        ))),

        // read one best-effort cached sample
        HostRequest::OsLocationLastKnown => {
            let sample = read_last_known_location(context)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::LocationSample(sample),
            )))
        }

        // open one runtime-scoped watch
        HostRequest::OsLocationWatchOpen { watch_id, options } => {
            open_location_watch(context, watch_id.clone(), *options)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }

        // close one runtime-scoped watch
        HostRequest::OsLocationWatchClose { watch_id } => {
            close_location_watch(context.host_runtime_id, watch_id)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }

        _ => Ok(None),
    }
}

/// Remove all active Linux location watches for one runtime.
pub(crate) fn unregister_location_runtime(host_runtime_id: HostRuntimeId) {
    let watches = {
        let service = linux_location_service();
        let mut service = service.lock();

        service
            .watches
            .remove(&host_runtime_id)
            .unwrap_or_default()
            .into_values()
            .collect::<Vec<_>>()
    };

    // stop one active worker at a time outside the registry lock
    for watch in watches {
        stop_location_watch(watch);
    }
}

/// Return whether GeoClue is reachable for location requests.
fn geoclue_is_available() -> bool {
    let Ok(connection) = Connection::system() else {
        return false;
    };
    Proxy::new(
        &connection,
        GEOCLUE_BUS_NAME,
        GEOCLUE_MANAGER_PATH,
        GEOCLUE_MANAGER_INTERFACE,
    )
    .is_ok()
}

/// Return whether host location services are currently available.
fn location_services_enabled() -> RuntimeResult<bool> {
    Ok(geoclue_is_available())
}

/// Read one last-known location sample from GeoClue.
fn read_last_known_location(context: &HostRequestContext) -> RuntimeResult<LocationSampleValue> {
    let desktop_id = resolved_application_identifier(context)?;
    let client = geo_clue_client(LOCATION_LAST_KNOWN_OPERATION, &desktop_id, None)?;
    client.start(LOCATION_LAST_KNOWN_OPERATION)?;

    // keep polling until GeoClue materializes one concrete location
    let deadline = Instant::now() + GEOCLUE_LAST_KNOWN_TIMEOUT;
    let result = loop {
        if let Some(sample) = client.read_sample(LOCATION_LAST_KNOWN_OPERATION)? {
            break Ok(sample);
        }

        if Instant::now() >= deadline {
            break Err(io_not_found(
                LOCATION_LAST_KNOWN_OPERATION,
                "linux location provider has not produced one sample yet",
            ));
        }

        thread::sleep(GEOCLUE_POLL_SLICE);
    };

    if let Err(error) = client.stop(LOCATION_LAST_KNOWN_OPERATION) {
        tracing::warn!(
            target: "destack.runtime.host.linux.location",
            ?error,
            "failed to stop linux location client after last-known read",
        );
    }

    result
}

/// Open one long-lived location watch worker.
fn open_location_watch(
    context: &HostRequestContext,
    watch_id: String,
    options: LocationWatchOptionsValue,
) -> RuntimeResult<()> {
    let host_runtime_id = context.host_runtime_id;
    let desktop_id = resolved_application_identifier(context)?;
    let stop = Arc::new(AtomicBool::new(false));
    let thread_stop = Arc::clone(&stop);
    let thread_watch_id = watch_id.clone();
    let thread_desktop_id = desktop_id.clone();
    let (ready_send, ready_recv) = sync_channel(1);
    let join_handle = thread::Builder::new()
        .name(format!("destack-linux-location-{watch_id}"))
        .spawn(move || {
            run_location_watch(
                host_runtime_id,
                thread_watch_id,
                thread_desktop_id,
                options,
                thread_stop,
                ready_send,
            );
        })
        .map_err(|error| {
            io_operation_error(
                LOCATION_WATCH_OPEN_OPERATION,
                Some(PlatformErrorCode::IoInvalidData),
                format!("failed to spawn one linux location worker: {error}"),
            )
        })?;

    // wait for worker initialization before publishing success
    match ready_recv.recv_timeout(GEOCLUE_START_TIMEOUT) {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            if join_handle.join().is_err() {
                tracing::warn!(
                    target: "destack.runtime.host.linux.location",
                    "linux location worker panicked during startup error handling",
                );
            }

            return Err(error);
        }
        Err(RecvTimeoutError::Timeout) => {
            stop.store(true, Ordering::SeqCst);
            if join_handle.join().is_err() {
                tracing::warn!(
                    target: "destack.runtime.host.linux.location",
                    "linux location worker panicked after startup timeout",
                );
            }

            return Err(io_operation_error(
                LOCATION_WATCH_OPEN_OPERATION,
                Some(PlatformErrorCode::IoWouldBlock),
                "linux location watch start timed out",
            ));
        }
        Err(RecvTimeoutError::Disconnected) => {
            if join_handle.join().is_err() {
                tracing::warn!(
                    target: "destack.runtime.host.linux.location",
                    "linux location worker panicked before reporting readiness",
                );
            }

            return Err(io_operation_error(
                LOCATION_WATCH_OPEN_OPERATION,
                Some(PlatformErrorCode::IoInvalidData),
                "linux location worker exited before reporting readiness",
            ));
        }
    }

    let service = linux_location_service();
    let mut service = service.lock();
    let runtime_watches = service.watches.entry(host_runtime_id).or_default();

    // reject duplicate watch identifiers per runtime
    if runtime_watches.contains_key(&watch_id) {
        stop.store(true, Ordering::SeqCst);
        if join_handle.join().is_err() {
            tracing::warn!(
                target: "destack.runtime.host.linux.location",
                "linux location worker panicked after duplicate watch rejection",
            );
        }

        return Err(io_operation_error(
            LOCATION_WATCH_OPEN_OPERATION,
            Some(PlatformErrorCode::IoAlreadyExists),
            format!("location watch `{watch_id}` is already open"),
        ));
    }

    runtime_watches.insert(watch_id, LinuxLocationWatch { stop, join_handle });

    Ok(())
}

/// Close one active Linux location watch worker.
fn close_location_watch(host_runtime_id: HostRuntimeId, watch_id: &str) -> RuntimeResult<()> {
    let watch = {
        let service = linux_location_service();
        let mut service = service.lock();
        let Some(runtime_watches) = service.watches.get_mut(&host_runtime_id) else {
            return Err(io_not_found(
                LOCATION_WATCH_CLOSE_OPERATION,
                "location watch was not found",
            ));
        };
        let Some(watch) = runtime_watches.remove(watch_id) else {
            return Err(io_not_found(
                LOCATION_WATCH_CLOSE_OPERATION,
                "location watch was not found",
            ));
        };

        if runtime_watches.is_empty() {
            service.watches.remove(&host_runtime_id);
        }

        watch
    };

    stop_location_watch(watch);

    Ok(())
}

/// Stop one active Linux location worker.
fn stop_location_watch(watch: LinuxLocationWatch) {
    watch.stop.store(true, Ordering::SeqCst);

    if watch.join_handle.join().is_err() {
        tracing::warn!(
            target: "destack.runtime.host.linux.location",
            "linux location worker panicked during shutdown",
        );
    }
}

/// Run one Linux location watch loop on a dedicated worker.
fn run_location_watch(
    host_runtime_id: HostRuntimeId,
    watch_id: String,
    desktop_id: String,
    options: LocationWatchOptionsValue,
    stop: Arc<AtomicBool>,
    ready_send: std::sync::mpsc::SyncSender<RuntimeResult<()>>,
) {
    let client = match geo_clue_client(LOCATION_WATCH_OPEN_OPERATION, &desktop_id, Some(options)) {
        Ok(client) => client,
        Err(error) => {
            if ready_send.send(Err(error)).is_err() {
                tracing::warn!(
                    target: "destack.runtime.host.linux.location",
                    "linux location readiness receiver dropped during client setup",
                );
            }

            return;
        }
    };

    if let Err(error) = client.start(LOCATION_WATCH_OPEN_OPERATION) {
        if ready_send.send(Err(error)).is_err() {
            tracing::warn!(
                target: "destack.runtime.host.linux.location",
                "linux location readiness receiver dropped during client start",
            );
        }

        return;
    }

    if ready_send.send(Ok(())).is_err() {
        tracing::warn!(
            target: "destack.runtime.host.linux.location",
            "linux location readiness receiver dropped after startup",
        );
    }
    let poll_interval = location_poll_interval(options);
    let mut last_timestamp_ns = None;

    // keep polling GeoClue until the watch closes
    while !stop.load(Ordering::SeqCst) {
        match client.read_sample(LOCATION_WATCH_OPEN_OPERATION) {
            Ok(Some(sample)) => {
                if last_timestamp_ns != Some(sample.timestamp_unix_ns) {
                    last_timestamp_ns = Some(sample.timestamp_unix_ns);
                    if let Err(error) =
                        linux_notify_location_sample(host_runtime_id.0, &watch_id, sample)
                    {
                        tracing::warn!(
                            target: "destack.runtime.host.linux.location",
                            ?error,
                            "failed to publish one linux location sample",
                        );
                    }
                }
            }
            Ok(None) => {}
            Err(_) => break,
        }

        thread::sleep(poll_interval);
    }

    if let Err(error) = client.stop(LOCATION_WATCH_OPEN_OPERATION) {
        tracing::warn!(
            target: "destack.runtime.host.linux.location",
            ?error,
            "failed to stop linux location client after watch shutdown",
        );
    }
}

/// Return the effective polling interval for one location watch.
fn location_poll_interval(options: LocationWatchOptionsValue) -> Duration {
    let requested_ns = options
        .minimum_interval_ns
        .max(GEOCLUE_POLL_SLICE.as_nanos() as u64);
    let clamped_ns = requested_ns.min(Duration::from_secs(5).as_nanos() as u64);

    Duration::from_nanos(clamped_ns)
}

impl GeoClueClient {
    /// Start one GeoClue client session.
    fn start(&self, operation: &str) -> RuntimeResult<()> {
        let proxy = self.client_proxy(operation)?;
        let _: () = proxy
            .call("Start", &())
            .map_err(|error| geoclue_error(operation, "Start", &error))?;

        Ok(())
    }

    /// Stop one GeoClue client session.
    fn stop(&self, operation: &str) -> RuntimeResult<()> {
        let proxy = self.client_proxy(operation)?;
        let _: () = proxy
            .call("Stop", &())
            .map_err(|error| geoclue_error(operation, "Stop", &error))?;

        Ok(())
    }

    /// Read the current GeoClue location sample when available.
    fn read_sample(&self, operation: &str) -> RuntimeResult<Option<LocationSampleValue>> {
        let proxy = self.client_proxy(operation)?;
        let location_path: OwnedObjectPath = proxy
            .get_property("Location")
            .map_err(|error| geoclue_error(operation, "Client::Location", &error))?;

        if location_path.as_str() == "/" {
            return Ok(None);
        }

        let proxy = Proxy::new(
            &self.connection,
            GEOCLUE_BUS_NAME,
            location_path.as_str(),
            GEOCLUE_LOCATION_INTERFACE,
        )
        .map_err(|error| geoclue_error(operation, "Location proxy", &error))?;
        let latitude_degrees: f64 = proxy
            .get_property("Latitude")
            .map_err(|error| geoclue_error(operation, "Location::Latitude", &error))?;
        let longitude_degrees: f64 = proxy
            .get_property("Longitude")
            .map_err(|error| geoclue_error(operation, "Location::Longitude", &error))?;
        let altitude_meters: f64 = proxy
            .get_property("Altitude")
            .map_err(|error| geoclue_error(operation, "Location::Altitude", &error))?;
        let horizontal_accuracy_meters: f64 = proxy
            .get_property("Accuracy")
            .map_err(|error| geoclue_error(operation, "Location::Accuracy", &error))?;
        let speed_meters_per_second: f64 = proxy
            .get_property("Speed")
            .map_err(|error| geoclue_error(operation, "Location::Speed", &error))?;
        let heading_degrees: f64 = proxy
            .get_property("Heading")
            .map_err(|error| geoclue_error(operation, "Location::Heading", &error))?;
        let timestamp: (u64, u64) = proxy
            .get_property("Timestamp")
            .map_err(|error| geoclue_error(operation, "Location::Timestamp", &error))?;
        let timestamp_unix_ns = timestamp
            .0
            .saturating_mul(1_000_000_000)
            .saturating_add(timestamp.1.saturating_mul(1_000));

        Ok(Some(LocationSampleValue {
            latitude_degrees,
            longitude_degrees,
            altitude_meters: altitude_meters.is_finite().then_some(altitude_meters),
            horizontal_accuracy_meters: horizontal_accuracy_meters
                .is_finite()
                .then_some(horizontal_accuracy_meters),
            vertical_accuracy_meters: None,
            speed_meters_per_second: (speed_meters_per_second.is_finite()
                && speed_meters_per_second >= 0.0)
                .then_some(speed_meters_per_second),
            heading_degrees: (heading_degrees.is_finite() && heading_degrees >= 0.0)
                .then_some(heading_degrees),
            timestamp_unix_ns,
        }))
    }

    /// Return the active GeoClue client proxy.
    fn client_proxy(&self, operation: &str) -> RuntimeResult<Proxy<'_>> {
        Proxy::new(
            &self.connection,
            GEOCLUE_BUS_NAME,
            self.client_path.as_str(),
            GEOCLUE_CLIENT_INTERFACE,
        )
        .map_err(|error| geoclue_error(operation, "Client proxy", &error))
    }
}

/// Create one configured GeoClue client.
fn geo_clue_client(
    operation: &str,
    desktop_id: &str,
    options: Option<LocationWatchOptionsValue>,
) -> RuntimeResult<GeoClueClient> {
    let connection =
        Connection::system().map_err(|error| geoclue_error(operation, "system bus", &error))?;
    let manager = Proxy::new(
        &connection,
        GEOCLUE_BUS_NAME,
        GEOCLUE_MANAGER_PATH,
        GEOCLUE_MANAGER_INTERFACE,
    )
    .map_err(|error| geoclue_error(operation, "Manager proxy", &error))?;
    let client_path: OwnedObjectPath = manager
        .call("GetClient", &())
        .map_err(|error| geoclue_error(operation, "Manager::GetClient", &error))?;
    let accuracy_level = geoclue_accuracy_level(options);
    let time_threshold = geoclue_time_threshold(options);
    let distance_threshold = geoclue_distance_threshold(options);

    // configure the freshly allocated geoclue client
    {
        let client = Proxy::new(
            &connection,
            GEOCLUE_BUS_NAME,
            client_path.as_str(),
            GEOCLUE_CLIENT_INTERFACE,
        )
        .map_err(|error| geoclue_error(operation, "Client proxy", &error))?;

        // identify the unsandboxed host runtime to GeoClue
        client
            .set_property("DesktopId", desktop_id)
            .map_err(|error| geoclue_error(operation, "Client::DesktopId", &error))?;

        // then apply the selected accuracy and watch thresholds
        client
            .set_property("RequestedAccuracyLevel", accuracy_level)
            .map_err(|error| geoclue_error(operation, "Client::RequestedAccuracyLevel", &error))?;
        client
            .set_property("TimeThreshold", time_threshold)
            .map_err(|error| geoclue_error(operation, "Client::TimeThreshold", &error))?;
        client
            .set_property("DistanceThreshold", distance_threshold)
            .map_err(|error| geoclue_error(operation, "Client::DistanceThreshold", &error))?;
    }

    Ok(GeoClueClient {
        connection,
        client_path,
    })
}

/// Map one runtime location accuracy hint to one GeoClue accuracy level.
fn geoclue_accuracy_level(options: Option<LocationWatchOptionsValue>) -> u32 {
    let Some(options) = options else {
        return 8;
    };

    match options.accuracy {
        LocationAccuracy::Passive => 1,
        LocationAccuracy::Low => 3,
        LocationAccuracy::Balanced => 4,
        LocationAccuracy::High => 6,
        _ => 8,
    }
}

/// Map one runtime watch interval to one GeoClue time threshold in seconds.
fn geoclue_time_threshold(options: Option<LocationWatchOptionsValue>) -> u32 {
    let Some(options) = options else {
        return 1;
    };

    let seconds = options.minimum_interval_ns / 1_000_000_000;

    seconds.max(1).min(u32::MAX as u64) as u32
}

/// Map one runtime watch distance threshold to one GeoClue distance threshold in meters.
fn geoclue_distance_threshold(options: Option<LocationWatchOptionsValue>) -> u32 {
    let Some(options) = options else {
        return 0;
    };

    options
        .minimum_distance_meters
        .max(0.0)
        .ceil()
        .min(u32::MAX as f64) as u32
}

/// Build one loud GeoClue backend error.
fn geoclue_error(
    operation: &str,
    action: &str,
    error: &ZbusError,
) -> Box<crate::diagnostic::RuntimeError> {
    // map missing GeoClue services to one honest unsupported error
    if let ZbusError::MethodError(name, detail, _) = error
        && (**name == *"org.freedesktop.DBus.Error.ServiceUnknown"
            || **name == *"org.freedesktop.DBus.Error.NameHasNoOwner")
    {
        let detail = detail
            .as_deref()
            .unwrap_or("GeoClue service is unavailable");

        return io_operation_error(
            operation,
            Some(PlatformErrorCode::NotSupported),
            format!("linux location {action} failed: {detail}"),
        );
    }

    // map explicit permission denials to ioPermissionDenied
    if let ZbusError::MethodError(name, detail, _) = error
        && (**name == *"org.freedesktop.DBus.Error.AccessDenied"
            || **name == *"org.freedesktop.DBus.Error.PermissionDenied")
    {
        let detail = detail
            .as_deref()
            .unwrap_or("GeoClue denied location access");

        return io_operation_error(
            operation,
            Some(PlatformErrorCode::IoPermissionDenied),
            format!("linux location {action} failed: {detail}"),
        );
    }

    io_operation_error(
        operation,
        Some(PlatformErrorCode::IoInvalidData),
        format!("linux location {action} failed: {error}"),
    )
}
