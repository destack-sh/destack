use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock, Weak};

use parking_lot::Mutex;
use windows::Devices::Enumeration::{
    DeviceInformation, DeviceInformationUpdate, DeviceWatcher, DeviceWatcherStatus,
};
use windows::Devices::Midi::{MidiInPort, MidiOutPort};
use windows::Foundation::TypedEventHandler;
use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
use windows::Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize};
use windows::core::{Error as WinError, HSTRING};

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
use crate::platform::midi::{MidiEventSource, MidiPortDirection};

use super::core::{
    WinRtEndpointInfo, WinRtEventDeliveryKind, WinRtEventSession, WinRtTopologyState,
};
use super::descriptor::device_descriptor;
use super::event::{queue_backend_disconnected_events, refresh_native_event_sessions};

/// Global WinRT service shared by all sessions.
static WINRT_SERVICE: OnceLock<Arc<WinRtService>> = OnceLock::new();

thread_local! {
    /// Per-thread apartment guard for WinRT callers outside the bootstrap thread.
    static WINRT_THREAD_APARTMENT: RefCell<Option<WinRtApartment>> = const { RefCell::new(None) };
}

/// One process-global WinRT runtime service.
pub(super) struct WinRtService {
    /// Apartment initialization guard for WinRT access.
    _apartment: WinRtApartment,
    /// Shared topology cache.
    pub(super) topology: Arc<Mutex<WinRtTopologyState>>,
    /// Registered native event subscriptions.
    pub(super) native_event_registry: Arc<Mutex<WinRtNativeEventRegistry>>,
    /// Live input watcher and its event registrations.
    _input_watcher: WinRtWatcherRegistration,
    /// Live output watcher and its event registrations.
    _output_watcher: WinRtWatcherRegistration,
}

/// One registry of native event subscriptions.
pub(super) struct WinRtNativeEventRegistry {
    /// Next registration id.
    pub(super) next_registration_id: u64,
    /// Registered native event subscriptions.
    pub(super) sessions: BTreeMap<u64, Weak<Mutex<WinRtEventSession>>>,
}

/// One apartment initialization guard.
struct WinRtApartment {
    /// Whether this guard owns one matching uninitialize call.
    should_uninitialize: bool,
}

/// One watcher registration bundle.
struct WinRtWatcherRegistration {
    /// Live watcher object.
    watcher: DeviceWatcher,
    /// Added callback token.
    added_token: i64,
    /// Updated callback token.
    updated_token: i64,
    /// Removed callback token.
    removed_token: i64,
    /// Enumeration-completed callback token.
    enumeration_completed_token: i64,
    /// Stopped callback token.
    stopped_token: i64,
}

impl Drop for WinRtApartment {
    /// Uninitialize the owned WinRT apartment.
    fn drop(&mut self) {
        if self.should_uninitialize {
            unsafe {
                RoUninitialize();
            }
        }
    }
}

impl Drop for WinRtWatcherRegistration {
    /// Tear down one WinRT device watcher.
    fn drop(&mut self) {
        let _ = self.watcher.RemoveAdded(self.added_token);
        let _ = self.watcher.RemoveUpdated(self.updated_token);
        let _ = self.watcher.RemoveRemoved(self.removed_token);
        let _ = self
            .watcher
            .RemoveEnumerationCompleted(self.enumeration_completed_token);
        let _ = self.watcher.RemoveStopped(self.stopped_token);

        if let Ok(status) = self.watcher.Status()
            && status == DeviceWatcherStatus::Started
        {
            let _ = self.watcher.Stop();
        }
    }
}

/// Map one WinRT error into one runtime error.
pub(super) fn winrt_error(
    operation: &'static str,
    action: &str,
    error: &WinError,
) -> Box<crate::diagnostic::RuntimeError> {
    core_platform::io_operation_error(operation, None, format!("{action}: {error}"))
}

/// Ensure the current thread can use WinRT APIs.
fn initialize_winrt_apartment(operation: &'static str) -> RuntimeResult<WinRtApartment> {
    match unsafe { RoInitialize(RO_INIT_MULTITHREADED) } {
        Ok(()) => Ok(WinRtApartment {
            should_uninitialize: true,
        }),
        Err(error) if error.code() == RPC_E_CHANGED_MODE => Ok(WinRtApartment {
            should_uninitialize: false,
        }),
        Err(error) => Err(winrt_error(operation, "RoInitialize", &error)),
    }
}

/// Ensure the current thread can interact with WinRT objects.
pub(super) fn ensure_current_thread_winrt_apartment(operation: &'static str) -> RuntimeResult<()> {
    WINRT_THREAD_APARTMENT.with(|slot| {
        // keep one apartment guard per calling thread
        if slot.borrow().is_some() {
            return Ok(());
        }

        let apartment = initialize_winrt_apartment(operation)?;
        *slot.borrow_mut() = Some(apartment);

        Ok(())
    })
}

/// Best-effort apartment initialization for drop paths.
pub(super) fn ensure_current_thread_winrt_apartment_for_drop() {
    let _ = ensure_current_thread_winrt_apartment("destack.midi.winrt.drop");
}

/// Enumerate one direction of WinRT endpoints.
fn enumerate_direction(
    direction: MidiPortDirection,
    selector: &HSTRING,
    operation: &'static str,
) -> RuntimeResult<BTreeMap<String, WinRtEndpointInfo>> {
    let collection = DeviceInformation::FindAllAsyncAqsFilter(selector)
        .map_err(|error| {
            winrt_error(
                operation,
                "DeviceInformation::FindAllAsyncAqsFilter",
                &error,
            )
        })?
        .get()
        .map_err(|error| winrt_error(operation, "IAsyncOperation::get", &error))?;

    let mut descriptors = BTreeMap::new();

    for device in &collection {
        let descriptor = device_descriptor(direction, &device)
            .map_err(|error| winrt_error(operation, "device_descriptor", &error))?;
        descriptors.insert(
            descriptor.id.clone(),
            WinRtEndpointInfo {
                backend_id: descriptor.backend_id.clone().unwrap_or_default(),
                descriptor,
            },
        );
    }

    Ok(descriptors)
}

/// Refresh one direction in the shared WinRT topology cache.
fn refresh_direction_cache(
    topology: &Arc<Mutex<WinRtTopologyState>>,
    direction: MidiPortDirection,
    selector: &HSTRING,
    operation: &'static str,
) -> RuntimeResult<()> {
    let descriptors = enumerate_direction(direction, selector, operation)?;
    let mut topology = topology.lock();

    match direction {
        MidiPortDirection::Input => topology.inputs = descriptors,
        MidiPortDirection::Output => topology.outputs = descriptors,
    }

    Ok(())
}

/// Refresh both directions in the shared WinRT topology cache.
fn refresh_topology_cache(
    topology: &Arc<Mutex<WinRtTopologyState>>,
    input_selector: &HSTRING,
    output_selector: &HSTRING,
    operation: &'static str,
) -> RuntimeResult<()> {
    refresh_direction_cache(
        topology,
        MidiPortDirection::Input,
        input_selector,
        operation,
    )?;
    refresh_direction_cache(
        topology,
        MidiPortDirection::Output,
        output_selector,
        operation,
    )?;

    Ok(())
}

/// Build one watcher callback that refreshes one direction and notifies subscriptions.
fn watcher_refresh_handler<T>(
    topology: Arc<Mutex<WinRtTopologyState>>,
    registry: Arc<Mutex<WinRtNativeEventRegistry>>,
    direction: MidiPortDirection,
    selector: HSTRING,
    operation: &'static str,
) -> TypedEventHandler<DeviceWatcher, T>
where
    T: windows::core::RuntimeType + 'static,
{
    TypedEventHandler::new(move |_watcher, _args| {
        let _ = refresh_direction_cache(&topology, direction, &selector, operation);

        refresh_native_event_sessions(&topology, &registry, MidiEventSource::Native);
        Ok(())
    })
}

/// Build one watcher callback that only notifies subscriptions on watcher stop.
fn watcher_stopped_handler(
    registry: Arc<Mutex<WinRtNativeEventRegistry>>,
) -> TypedEventHandler<DeviceWatcher, windows::core::IInspectable> {
    TypedEventHandler::new(move |_watcher, _args| {
        queue_backend_disconnected_events(&registry, MidiEventSource::Native, 0);
        Ok(())
    })
}

/// Create and start one direction-specific device watcher.
fn create_watcher(
    direction: MidiPortDirection,
    selector: &HSTRING,
    topology: Arc<Mutex<WinRtTopologyState>>,
    registry: Arc<Mutex<WinRtNativeEventRegistry>>,
    operation: &'static str,
) -> RuntimeResult<WinRtWatcherRegistration> {
    let watcher = DeviceInformation::CreateWatcherAqsFilter(selector).map_err(|error| {
        winrt_error(
            operation,
            "DeviceInformation::CreateWatcherAqsFilter",
            &error,
        )
    })?;

    let added_token = watcher
        .Added(&watcher_refresh_handler::<DeviceInformation>(
            topology.clone(),
            registry.clone(),
            direction,
            selector.clone(),
            operation,
        ))
        .map_err(|error| winrt_error(operation, "DeviceWatcher::Added", &error))?;
    let updated_token = watcher
        .Updated(&watcher_refresh_handler::<DeviceInformationUpdate>(
            topology.clone(),
            registry.clone(),
            direction,
            selector.clone(),
            operation,
        ))
        .map_err(|error| winrt_error(operation, "DeviceWatcher::Updated", &error))?;
    let removed_token = watcher
        .Removed(&watcher_refresh_handler::<DeviceInformationUpdate>(
            topology.clone(),
            registry.clone(),
            direction,
            selector.clone(),
            operation,
        ))
        .map_err(|error| winrt_error(operation, "DeviceWatcher::Removed", &error))?;
    let enumeration_completed_token = watcher
        .EnumerationCompleted(&watcher_refresh_handler::<windows::core::IInspectable>(
            topology.clone(),
            registry.clone(),
            direction,
            selector.clone(),
            operation,
        ))
        .map_err(|error| winrt_error(operation, "DeviceWatcher::EnumerationCompleted", &error))?;
    let stopped_token = watcher
        .Stopped(&watcher_stopped_handler(registry))
        .map_err(|error| winrt_error(operation, "DeviceWatcher::Stopped", &error))?;

    watcher
        .Start()
        .map_err(|error| winrt_error(operation, "DeviceWatcher::Start", &error))?;

    Ok(WinRtWatcherRegistration {
        watcher,
        added_token,
        updated_token,
        removed_token,
        enumeration_completed_token,
        stopped_token,
    })
}

/// Return the shared WinRT service.
pub(super) fn winrt_service(operation: &'static str) -> RuntimeResult<Arc<WinRtService>> {
    // all WinRT callers must enter one initialized apartment
    ensure_current_thread_winrt_apartment(operation)?;

    // reuse the live process global service when it already exists
    if let Some(service) = WINRT_SERVICE.get() {
        return Ok(service.clone());
    }

    // bootstrap selectors and shared service state
    let apartment = initialize_winrt_apartment(operation)?;
    let input_selector = MidiInPort::GetDeviceSelector()
        .map_err(|error| winrt_error(operation, "MidiInPort::GetDeviceSelector", &error))?;
    let output_selector = MidiOutPort::GetDeviceSelector()
        .map_err(|error| winrt_error(operation, "MidiOutPort::GetDeviceSelector", &error))?;
    let topology = Arc::new(Mutex::new(WinRtTopologyState::default()));
    let native_event_registry = Arc::new(Mutex::new(WinRtNativeEventRegistry {
        next_registration_id: 1,
        sessions: BTreeMap::new(),
    }));

    refresh_topology_cache(&topology, &input_selector, &output_selector, operation)?;

    // start live device watchers after the initial topology snapshot
    let input_watcher = create_watcher(
        MidiPortDirection::Input,
        &input_selector,
        topology.clone(),
        native_event_registry.clone(),
        operation,
    )?;
    let output_watcher = create_watcher(
        MidiPortDirection::Output,
        &output_selector,
        topology.clone(),
        native_event_registry.clone(),
        operation,
    )?;

    // publish the initialized service once all host resources are live
    let service = Arc::new(WinRtService {
        _apartment: apartment,
        topology,
        native_event_registry,
        _input_watcher: input_watcher,
        _output_watcher: output_watcher,
    });
    let _ = WINRT_SERVICE.set(service.clone());

    Ok(WINRT_SERVICE.get().cloned().unwrap_or(service))
}

/// Register one native event subscription.
pub(super) fn register_native_event_session(
    service: &Arc<WinRtService>,
    session: &Arc<Mutex<WinRtEventSession>>,
) -> WinRtEventDeliveryKind {
    let mut registry = service.native_event_registry.lock();
    let registration_id = registry.next_registration_id;
    registry.next_registration_id = registry.next_registration_id.saturating_add(1);
    registry
        .sessions
        .insert(registration_id, Arc::downgrade(session));

    WinRtEventDeliveryKind::Native {
        registry: service.native_event_registry.clone(),
        registration_id,
    }
}

/// Remove one native event subscription from the shared registry.
pub(super) fn unregister_native_event_session(delivery_kind: &WinRtEventDeliveryKind) {
    let WinRtEventDeliveryKind::Native {
        registry,
        registration_id,
    } = delivery_kind
    else {
        return;
    };

    registry.lock().sessions.remove(registration_id);
}

/// Return one cached input endpoint descriptor snapshot.
pub(super) fn input_descriptors(service: &Arc<WinRtService>) -> Vec<WinRtEndpointInfo> {
    service.topology.lock().inputs.values().cloned().collect()
}

/// Return one cached output endpoint descriptor snapshot.
pub(super) fn output_descriptors(service: &Arc<WinRtService>) -> Vec<WinRtEndpointInfo> {
    service.topology.lock().outputs.values().cloned().collect()
}
