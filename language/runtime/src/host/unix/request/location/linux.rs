use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use zbus::Error as ZbusError;
use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::OwnedObjectPath;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::os::linux::ingress::notify::linux_notify_location_sample;
use crate::host::os::unix::identity::resolved_application_identifier;
use crate::host::{
    HostRequest, HostRequestOutcome, HostRequestResult, HostSessionId, RequestContext,
};
use crate::platform::core::{io_not_found, io_operation_error};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    LocationAccuracy, LocationSampleValue, LocationWatchOptionsValue,
};
use crate::runtime::action::ActionSet;
use crate::runtime::service::Service;
use crate::runtime::{ExecutionMode, ExecutionPolicy, WorkerLoop};

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

/// The maximum wait for one last-known location sample.
const GEOCLUE_LAST_KNOWN_TIMEOUT: Duration = Duration::from_secs(15);

/// The sleep slice used while polling GeoClue location updates.
const GEOCLUE_POLL_SLICE: Duration = Duration::from_millis(250);

struct LinuxLocationService {
    /// Active runtimes keyed by runtime id.
    runtimes: Mutex<HashMap<HostSessionId, LinuxLocationRuntime>>,
}

/// One runtime-owned Linux location service loop.
struct LinuxLocationRuntime {
    /// Shared stop flag for the runtime loop.
    stop: Arc<AtomicBool>,
    /// Shared runtime watch state.
    state: Arc<Mutex<LinuxLocationRuntimeState>>,
    /// Worker loop that owns the active GeoClue client.
    worker: WorkerLoop,
}

/// One runtime-owned Linux location state.
struct LinuxLocationRuntimeState {
    /// Stable desktop id used for GeoClue clients.
    desktop_id: String,
    /// Active watches keyed by runtime watch id.
    watches: HashMap<String, LinuxLocationWatchState>,
    /// Monotonic watch-configuration revision.
    configuration_revision: u64,
}

/// One runtime-owned Linux location watch state.
struct LinuxLocationWatchState {
    /// Watch option payload.
    options: LocationWatchOptionsValue,
    /// The last sample delivered to this watch.
    last_sample: Option<LocationSampleValue>,
}

/// One active GeoClue client session.
struct GeoClueClient {
    /// System bus connection for GeoClue calls.
    connection: Connection,
    /// Object path for this client.
    client_path: OwnedObjectPath,
}

impl LinuxLocationService {
    /// Create one empty Linux location service.
    fn new() -> Self {
        Self {
            runtimes: Mutex::new(HashMap::new()),
        }
    }

    /// Open one long-lived location watch worker.
    fn open_watch(
        &self,
        context: &RequestContext,
        watch_id: String,
        options: LocationWatchOptionsValue,
    ) -> RuntimeResult<()> {
        let mut runtimes = self.runtimes.lock();

        // attach one new watch to the active runtime
        if let Some(runtime) = runtimes.get_mut(&context.host_session_id) {
            let mut state = runtime.state.lock();

            if state.watches.contains_key(&watch_id) {
                return Err(io_operation_error(
                    LOCATION_WATCH_OPEN_OPERATION,
                    Some(PlatformErrorCode::IoAlreadyExists),
                    format!("location watch `{watch_id}` is already open"),
                ));
            }

            state.watches.insert(
                watch_id,
                LinuxLocationWatchState {
                    options,
                    last_sample: None,
                },
            );
            state.configuration_revision = state.configuration_revision.saturating_add(1);

            return Ok(());
        }

        let desktop_id = resolved_application_identifier(context)?;
        let runtime_state = Arc::new(Mutex::new(LinuxLocationRuntimeState {
            desktop_id: desktop_id.clone(),
            watches: HashMap::from([(
                watch_id,
                LinuxLocationWatchState {
                    options: options.clone(),
                    last_sample: None,
                },
            )]),
            configuration_revision: 1,
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let worker_state = Arc::clone(&runtime_state);
        let worker_shutdown = Arc::clone(&stop);
        let worker_stop = Arc::clone(&stop);
        let host_session_id = context.host_session_id;

        let worker = WorkerLoop::open(
            &format!("destack-linux-location-{}", host_session_id.0),
            LOCATION_WATCH_OPEN_OPERATION,
            ExecutionPolicy::process(ExecutionMode::Loop),
            move || {
                let configuration =
                    runtime_watch_configuration(&worker_state).ok_or_else(|| {
                        io_operation_error(
                            LOCATION_WATCH_OPEN_OPERATION,
                            Some(PlatformErrorCode::IoInvalidData),
                            "linux location runtime has no active watches",
                        )
                    })?;
                let client = geo_clue_client(
                    LOCATION_WATCH_OPEN_OPERATION,
                    configuration.desktop_id.as_str(),
                    Some(configuration.options),
                )?;
                client.start(LOCATION_WATCH_OPEN_OPERATION)?;

                Ok((
                    Box::new(move || {
                        worker_shutdown.store(true, Ordering::SeqCst);
                    }),
                    Box::new(move || {
                        run_location_runtime(
                            host_session_id,
                            worker_state,
                            stop,
                            client,
                            configuration.revision,
                            location_poll_interval(&configuration.options),
                        )
                    }),
                ))
            },
        )?;

        runtimes.insert(
            host_session_id,
            LinuxLocationRuntime {
                stop,
                state: runtime_state,
                worker,
            },
        );

        Ok(())
    }

    /// Close one active Linux location watch.
    fn close_watch(&self, host_session_id: HostSessionId, watch_id: &str) -> RuntimeResult<()> {
        let runtime = {
            let mut runtimes = self.runtimes.lock();
            let Some(runtime) = runtimes.get_mut(&host_session_id) else {
                return Err(io_not_found(
                    LOCATION_WATCH_CLOSE_OPERATION,
                    "location watch was not found",
                ));
            };
            let mut state = runtime.state.lock();
            let removed = state.watches.remove(watch_id);

            if removed.is_none() {
                return Err(io_not_found(
                    LOCATION_WATCH_CLOSE_OPERATION,
                    "location watch was not found",
                ));
            }

            if !state.watches.is_empty() {
                state.configuration_revision = state.configuration_revision.saturating_add(1);
                return Ok(());
            }

            drop(state);
            runtimes.remove(&host_session_id)
        };

        if let Some(runtime) = runtime {
            stop_location_runtime(runtime);
        }

        Ok(())
    }

    /// Remove all active Linux location watches for one runtime.
    fn unregister_runtime(&self, host_session_id: HostSessionId) {
        let runtime = {
            let mut runtimes = self.runtimes.lock();
            runtimes.remove(&host_session_id)
        };

        if let Some(runtime) = runtime {
            stop_location_runtime(runtime);
        }
    }
}

impl Service for LinuxLocationService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Inline);
}

/// Return dynamic Unix location actions for Linux hosts.
pub(crate) fn request_actions() -> ActionSet {
    ActionSet::new()
}

/// Submit one Linux location request through GeoClue.
pub(crate) fn submit_location_request(
    context: &RequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        // report whether GeoClue is reachable on this host
        HostRequest::OsLocationServicesEnabled => Ok(Some(HostRequestOutcome::immediate(
            HostRequestResult::Bool(location_services_enabled()?),
        ))),

        // read the most recent sample the provider yields within the timeout window
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
            close_location_watch(context.host_session_id, watch_id)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }

        _ => Ok(None),
    }
}

/// Remove all active Linux location watches for one runtime.
pub(crate) fn unregister_location_runtime(host_session_id: HostSessionId) {
    if let Some(service) = LinuxLocationService::active() {
        service.unregister_runtime(host_session_id);
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

/// Read the most recent location sample from GeoClue.
fn read_last_known_location(context: &RequestContext) -> RuntimeResult<LocationSampleValue> {
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

/// Return the shared Linux location service.
fn linux_location_service() -> RuntimeResult<Arc<LinuxLocationService>> {
    LinuxLocationService::global(|| Ok(LinuxLocationService::new()))
}

/// Open one long-lived location watch worker.
fn open_location_watch(
    context: &RequestContext,
    watch_id: String,
    options: LocationWatchOptionsValue,
) -> RuntimeResult<()> {
    let service = linux_location_service()?;

    service.open_watch(context, watch_id, options)
}

/// Close one active Linux location watch worker.
fn close_location_watch(host_session_id: HostSessionId, watch_id: &str) -> RuntimeResult<()> {
    let service = linux_location_service()?;

    service.close_watch(host_session_id, watch_id)
}

/// Stop one active Linux location runtime.
fn stop_location_runtime(runtime: LinuxLocationRuntime) {
    runtime.stop.store(true, Ordering::SeqCst);
    drop(runtime.worker);
}

/// Run one Linux location runtime loop on a dedicated worker.
fn run_location_runtime(
    host_session_id: HostSessionId,
    runtime_state: Arc<Mutex<LinuxLocationRuntimeState>>,
    stop: Arc<AtomicBool>,
    mut client: GeoClueClient,
    mut configuration_revision: u64,
    mut poll_interval: Duration,
) -> RuntimeResult<()> {
    // keep polling GeoClue until the runtime closes
    loop {
        if stop.load(Ordering::SeqCst) {
            stop_geoclue_client(&client, LOCATION_WATCH_OPEN_OPERATION);
            return Ok(());
        }

        if let Some(configuration) =
            runtime_watch_configuration_if_changed(&runtime_state, configuration_revision)
        {
            stop_geoclue_client(&client, LOCATION_WATCH_OPEN_OPERATION);

            client = geo_clue_client(
                LOCATION_WATCH_OPEN_OPERATION,
                configuration.desktop_id.as_str(),
                Some(configuration.options),
            )?;
            client.start(LOCATION_WATCH_OPEN_OPERATION)?;
            configuration_revision = configuration.revision;
            poll_interval = location_poll_interval(&configuration.options);
        }

        match client.read_sample(LOCATION_WATCH_OPEN_OPERATION) {
            Ok(Some(sample)) => publish_location_sample(host_session_id, &runtime_state, &sample),
            Ok(None) => {}
            Err(error) => {
                stop_geoclue_client(&client, LOCATION_WATCH_OPEN_OPERATION);
                return Err(error);
            }
        }

        thread::sleep(poll_interval);
    }
}

/// Return the effective polling interval for one location watch configuration.
fn location_poll_interval(options: &LocationWatchOptionsValue) -> Duration {
    let requested_ns = options
        .minimum_interval_ns
        .max(GEOCLUE_POLL_SLICE.as_nanos() as u64);
    let clamped_ns = requested_ns.min(Duration::from_secs(5).as_nanos() as u64);

    Duration::from_nanos(clamped_ns)
}

/// Return one merged runtime watch configuration when any watches are active.
fn runtime_watch_configuration(
    runtime_state: &Arc<Mutex<LinuxLocationRuntimeState>>,
) -> Option<LinuxLocationWatchConfiguration> {
    let state = runtime_state.lock();
    let mut watches = state.watches.values();
    let first = watches.next()?;
    let mut options = first.options;

    for watch in watches {
        if watch.options.accuracy as u8 > options.accuracy as u8 {
            options.accuracy = watch.options.accuracy;
        }

        options.minimum_interval_ns = options
            .minimum_interval_ns
            .min(watch.options.minimum_interval_ns);
        options.minimum_distance_meters = options
            .minimum_distance_meters
            .min(watch.options.minimum_distance_meters);
        options.include_heading |= watch.options.include_heading;
    }

    Some(LinuxLocationWatchConfiguration {
        desktop_id: state.desktop_id.clone(),
        options,
        revision: state.configuration_revision,
    })
}

/// Return one merged runtime watch configuration when the revision changed.
fn runtime_watch_configuration_if_changed(
    runtime_state: &Arc<Mutex<LinuxLocationRuntimeState>>,
    revision: u64,
) -> Option<LinuxLocationWatchConfiguration> {
    let configuration = runtime_watch_configuration(runtime_state)?;

    (configuration.revision != revision).then_some(configuration)
}

/// Publish one raw location sample to every matching runtime watch.
fn publish_location_sample(
    host_session_id: HostSessionId,
    runtime_state: &Arc<Mutex<LinuxLocationRuntimeState>>,
    sample: &LocationSampleValue,
) {
    let notifications = {
        let mut state = runtime_state.lock();
        let mut notifications = Vec::new();

        // evaluate one sample against each runtime watch filter
        for (watch_id, watch) in &mut state.watches {
            if !watch_should_receive_sample(watch, sample) {
                continue;
            }

            let mut sample = *sample;
            if !watch.options.include_heading {
                sample.heading_degrees = None;
            }

            watch.last_sample = Some(sample);
            notifications.push((watch_id.clone(), sample));
        }

        notifications
    };

    for (watch_id, sample) in notifications {
        if let Err(error) = linux_notify_location_sample(host_session_id.0, &watch_id, sample) {
            tracing::warn!(
                target: "destack.runtime.host.linux.location",
                ?error,
                "failed to publish one linux location sample",
            );
        }
    }
}

/// Return whether one watch should receive one sample.
fn watch_should_receive_sample(
    watch: &LinuxLocationWatchState,
    sample: &LocationSampleValue,
) -> bool {
    let Some(last_sample) = watch.last_sample.as_ref() else {
        return true;
    };

    if sample.timestamp_unix_ns <= last_sample.timestamp_unix_ns {
        return false;
    }

    if sample
        .timestamp_unix_ns
        .saturating_sub(last_sample.timestamp_unix_ns)
        < watch.options.minimum_interval_ns
    {
        return false;
    }

    if watch.options.minimum_distance_meters > 0.0
        && location_distance_meters(last_sample, sample) < watch.options.minimum_distance_meters
    {
        return false;
    }

    true
}

/// Return the approximate distance between two samples in meters.
fn location_distance_meters(previous: &LocationSampleValue, current: &LocationSampleValue) -> f64 {
    const EARTH_RADIUS_METERS: f64 = 6_371_000.0;

    let previous_latitude = previous.latitude_degrees.to_radians();
    let current_latitude = current.latitude_degrees.to_radians();
    let delta_latitude = current_latitude - previous_latitude;
    let delta_longitude = (current.longitude_degrees - previous.longitude_degrees).to_radians();
    let sin_latitude = (delta_latitude / 2.0).sin();
    let sin_longitude = (delta_longitude / 2.0).sin();
    let haversine = sin_latitude * sin_latitude
        + previous_latitude.cos() * current_latitude.cos() * sin_longitude * sin_longitude;
    let central_angle = 2.0 * haversine.sqrt().asin();

    EARTH_RADIUS_METERS * central_angle
}

/// Stop one active GeoClue client session loudly when teardown fails.
fn stop_geoclue_client(client: &GeoClueClient, operation: &str) {
    if let Err(error) = client.stop(operation) {
        tracing::warn!(
            target: "destack.runtime.host.linux.location",
            ?error,
            "failed to stop linux location client after watch shutdown",
        );
    }
}

/// One merged Linux runtime watch configuration.
struct LinuxLocationWatchConfiguration {
    /// Stable desktop id used for GeoClue clients.
    desktop_id: String,
    /// Merged runtime watch options.
    options: LocationWatchOptionsValue,
    /// Current runtime watch configuration revision.
    revision: u64,
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
fn geoclue_error(operation: &str, action: &str, error: &ZbusError) -> Box<RuntimeError> {
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
