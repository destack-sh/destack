use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Weak};

use parking_lot::Mutex;

use super::core::*;
use super::metadata::{
    bluez_error, bluez_object_manager_rule, bluez_properties_changed_rule_namespace,
    bluez_signal_connection, current_device_descriptor,
};
use super::watch::{
    connect_device, disconnected_event, handle_adapter_message, handle_device_message,
    handle_scan_message, handle_subscription_message, start_adapter_discovery, start_notify,
    stop_adapter_discovery, stop_notify,
};
use crate::runtime::service::Service;
use crate::runtime::{ExecutionMode, ExecutionPolicy, WorkerLoop};

/// Process-global BlueZ transport and signal service.
pub(crate) struct LinuxBluetoothService {
    /// Shared blocking system-bus connection for direct request-response operations.
    connection: Connection,
    /// Shared BlueZ signal ingress loop.
    _signal_loop: WorkerLoop,
    /// Shared BlueZ signal dispatcher runtime.
    signal_runtime: Arc<LinuxBluetoothSignalRuntime>,
    /// Shared host-side activity counts for refcounted BlueZ operations.
    state: Mutex<LinuxBluetoothServiceState>,
}

/// Mutable host-side activity counts for one BlueZ service.
#[derive(Default)]
struct LinuxBluetoothServiceState {
    /// Active scan counts keyed by adapter path.
    adapter_scans: HashMap<String, usize>,
    /// Active device-session counts keyed by device path.
    device_sessions: HashMap<String, usize>,
    /// Active notification counts keyed by characteristic path.
    characteristic_notifications: HashMap<String, usize>,
}

/// Shared signal dispatcher runtime for one BlueZ service.
struct LinuxBluetoothSignalRuntime {
    /// The next registration identifier.
    next_registration_id: AtomicU64,
    /// Live signal registrations.
    registrations: Mutex<LinuxBluetoothSignalRegistrations>,
}

/// Mutable signal registrations for one BlueZ service.
#[derive(Default)]
struct LinuxBluetoothSignalRegistrations {
    /// Live adapter-watch registrations keyed by registration identifier.
    adapters: HashMap<u64, LinuxBluetoothAdapterRegistration>,
    /// Live scan registrations keyed by registration identifier.
    scans: HashMap<u64, LinuxBluetoothScanRegistration>,
    /// Live device-session registrations keyed by registration identifier.
    devices: HashMap<u64, LinuxBluetoothDeviceRegistration>,
    /// Live notification registrations keyed by registration identifier.
    subscriptions: HashMap<u64, LinuxBluetoothSubscriptionRegistration>,
}

/// One live adapter-watch registration.
struct LinuxBluetoothAdapterRegistration {
    /// The registered adapter-watch resource.
    resource: Weak<LinuxBluetoothAdapterWatchResource>,
    /// The last known adapter snapshot for this watch.
    known_adapters: BTreeMap<String, BluetoothAdapterDescriptorValue>,
}

/// One live scan registration.
struct LinuxBluetoothScanRegistration {
    /// The registered scan resource.
    resource: Weak<LinuxBluetoothScanResource>,
    /// The last known device snapshot for this scan.
    known_devices: BTreeMap<String, BluetoothDeviceDescriptorValue>,
}

/// One live device-session registration.
struct LinuxBluetoothDeviceRegistration {
    /// The registered device-session resource.
    resource: Weak<LinuxBluetoothDeviceResource>,
    /// The previously observed pair state.
    previous_pair_state: Option<BluetoothPairState>,
    /// The previously observed GATT cache.
    previous_cache: LinuxBluetoothGattCache,
}

/// One live notification registration.
struct LinuxBluetoothSubscriptionRegistration {
    /// The registered notification resource.
    resource: Weak<LinuxBluetoothSubscriptionResource>,
}

impl LinuxBluetoothService {
    /// Create one process-global BlueZ transport service.
    fn new() -> RuntimeResult<Self> {
        let connection = Connection::system().map_err(|error| {
            bluez_error(
                "platform.device.bluetooth.linux",
                "Connection::system",
                error,
            )
        })?;
        let signal_runtime = Arc::new(LinuxBluetoothSignalRuntime::default());
        let signal_connection = bluez_signal_connection(
            "platform.device.bluetooth.linux",
            &[
                bluez_object_manager_rule()?,
                bluez_properties_changed_rule_namespace("/org/bluez", None)?,
            ],
        )?;

        let signal_loop = spawn_signal_dispatch_loop(signal_runtime.clone(), signal_connection)?;

        Ok(Self {
            connection,
            _signal_loop: signal_loop,
            signal_runtime,
            state: Mutex::new(LinuxBluetoothServiceState::default()),
        })
    }

    /// Clone the shared blocking system-bus connection.
    pub(super) fn connection(&self) -> Connection {
        self.connection.clone()
    }

    /// Register one scan resource with the shared signal dispatcher.
    pub(super) fn register_scan(
        &self,
        resource: &Arc<LinuxBluetoothScanResource>,
        known_devices: BTreeMap<String, BluetoothDeviceDescriptorValue>,
    ) -> u64 {
        self.signal_runtime.register_scan(resource, known_devices)
    }

    /// Register one adapter watch with the shared signal dispatcher.
    pub(super) fn register_adapter_watch(
        &self,
        resource: &Arc<LinuxBluetoothAdapterWatchResource>,
        known_adapters: BTreeMap<String, BluetoothAdapterDescriptorValue>,
    ) -> u64 {
        self.signal_runtime
            .register_adapter_watch(resource, known_adapters)
    }

    /// Remove one scan resource from the shared signal dispatcher.
    pub(super) fn unregister_scan(&self, registration_id: u64) {
        self.signal_runtime.unregister_scan(registration_id);
    }

    /// Remove one adapter watch from the shared signal dispatcher.
    pub(super) fn unregister_adapter_watch(&self, registration_id: u64) {
        self.signal_runtime
            .unregister_adapter_watch(registration_id);
    }

    /// Register one device session with the shared signal dispatcher.
    pub(super) fn register_device(
        &self,
        resource: &Arc<LinuxBluetoothDeviceResource>,
        previous_pair_state: Option<BluetoothPairState>,
        previous_cache: LinuxBluetoothGattCache,
    ) -> u64 {
        self.signal_runtime
            .register_device(resource, previous_pair_state, previous_cache)
    }

    /// Remove one device session from the shared signal dispatcher.
    pub(super) fn unregister_device(&self, registration_id: u64) {
        self.signal_runtime.unregister_device(registration_id);
    }

    /// Register one value subscription with the shared signal dispatcher.
    pub(super) fn register_subscription(
        &self,
        resource: &Arc<LinuxBluetoothSubscriptionResource>,
    ) -> u64 {
        self.signal_runtime.register_subscription(resource)
    }

    /// Remove one value subscription from the shared signal dispatcher.
    pub(super) fn unregister_subscription(&self, registration_id: u64) {
        self.signal_runtime.unregister_subscription(registration_id);
    }

    /// Retain one adapter discovery session on the shared BlueZ connection.
    pub(super) fn retain_adapter_scan(&self, adapter_path: &str) -> RuntimeResult<()> {
        let should_start = {
            let mut state = self.state.lock();
            let count = state
                .adapter_scans
                .entry(adapter_path.to_string())
                .or_insert(0);
            let should_start = *count == 0;
            *count += 1;
            should_start
        };

        if !should_start {
            return Ok(());
        }

        let connection = self.connection();
        if let Err(error) = start_adapter_discovery(&connection, adapter_path) {
            let mut state = self.state.lock();
            if let Some(count) = state.adapter_scans.get_mut(adapter_path) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    state.adapter_scans.remove(adapter_path);
                }
            }

            return Err(error);
        }

        Ok(())
    }

    /// Release one adapter discovery session on the shared BlueZ connection.
    pub(super) fn release_adapter_scan(&self, adapter_path: &str) {
        let should_stop = {
            let mut state = self.state.lock();
            let Some(count) = state.adapter_scans.get_mut(adapter_path) else {
                return;
            };

            *count = count.saturating_sub(1);
            let should_stop = *count == 0;
            if should_stop {
                state.adapter_scans.remove(adapter_path);
            }

            should_stop
        };

        if should_stop {
            let connection = self.connection();
            let _ = stop_adapter_discovery(&connection, adapter_path);
        }
    }

    /// Retain one connected device session on the shared BlueZ connection.
    pub(super) fn retain_device_connection(&self, device_path: &str) -> RuntimeResult<()> {
        let should_connect = {
            let mut state = self.state.lock();
            let count = state
                .device_sessions
                .entry(device_path.to_string())
                .or_insert(0);
            let should_connect = *count == 0;
            *count += 1;
            should_connect
        };

        if should_connect {
            let connection = self.connection();
            if let Err(error) = connect_device(&connection, device_path) {
                let mut state = self.state.lock();
                if let Some(count) = state.device_sessions.get_mut(device_path) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        state.device_sessions.remove(device_path);
                    }
                }

                return Err(error);
            }
        }
        // recover from external disconnects on later local opens
        else {
            let connection = self.connection();
            let descriptor = current_device_descriptor(
                &connection,
                device_path,
                "destack.device.bluetooth.session.open",
            )?;
            let is_connected = descriptor.is_some_and(|descriptor| descriptor.connected);
            if !is_connected {
                if let Err(error) = connect_device(&connection, device_path) {
                    let mut state = self.state.lock();
                    if let Some(count) = state.device_sessions.get_mut(device_path) {
                        *count = count.saturating_sub(1);
                        if *count == 0 {
                            state.device_sessions.remove(device_path);
                        }
                    }

                    return Err(error);
                }
            }
        }

        Ok(())
    }

    /// Release one connected device session on the shared BlueZ connection.
    pub(super) fn release_device_connection(&self, device_path: &str) {
        let should_disconnect = {
            let mut state = self.state.lock();
            let Some(count) = state.device_sessions.get_mut(device_path) else {
                return;
            };

            *count = count.saturating_sub(1);
            let should_disconnect = *count == 0;
            if should_disconnect {
                state.device_sessions.remove(device_path);
            }

            should_disconnect
        };

        if should_disconnect {
            let connection = self.connection();
            let _ = super::watch::disconnect_device(&connection, device_path);
        }
    }

    /// Retain one notification session on the shared BlueZ connection.
    pub(super) fn retain_characteristic_notify(
        &self,
        characteristic_path: &str,
    ) -> RuntimeResult<()> {
        let should_start = {
            let mut state = self.state.lock();
            let count = state
                .characteristic_notifications
                .entry(characteristic_path.to_string())
                .or_insert(0);
            let should_start = *count == 0;
            *count += 1;
            should_start
        };

        if !should_start {
            return Ok(());
        }

        let connection = self.connection();
        if let Err(error) = start_notify(&connection, characteristic_path) {
            let mut state = self.state.lock();
            if let Some(count) = state
                .characteristic_notifications
                .get_mut(characteristic_path)
            {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    state
                        .characteristic_notifications
                        .remove(characteristic_path);
                }
            }

            return Err(error);
        }

        Ok(())
    }

    /// Release one notification session on the shared BlueZ connection.
    pub(super) fn release_characteristic_notify(&self, characteristic_path: &str) {
        let should_stop = {
            let mut state = self.state.lock();
            let Some(count) = state
                .characteristic_notifications
                .get_mut(characteristic_path)
            else {
                return;
            };

            *count = count.saturating_sub(1);
            let should_stop = *count == 0;
            if should_stop {
                state
                    .characteristic_notifications
                    .remove(characteristic_path);
            }

            should_stop
        };

        if should_stop {
            let connection = self.connection();
            let _ = stop_notify(&connection, characteristic_path);
        }
    }
}

impl Service for LinuxBluetoothService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Loop);
}

impl Default for LinuxBluetoothSignalRuntime {
    /// Create one empty BlueZ signal runtime.
    fn default() -> Self {
        Self {
            next_registration_id: AtomicU64::new(1),
            registrations: Mutex::new(LinuxBluetoothSignalRegistrations::default()),
        }
    }
}

impl LinuxBluetoothSignalRuntime {
    /// Register one adapter watch with its initial snapshot.
    fn register_adapter_watch(
        &self,
        resource: &Arc<LinuxBluetoothAdapterWatchResource>,
        known_adapters: BTreeMap<String, BluetoothAdapterDescriptorValue>,
    ) -> u64 {
        let registration_id = self.next_registration_id.fetch_add(1, Ordering::AcqRel);
        let registration = LinuxBluetoothAdapterRegistration {
            resource: Arc::downgrade(resource),
            known_adapters,
        };

        self.registrations
            .lock()
            .adapters
            .insert(registration_id, registration);

        registration_id
    }

    /// Remove one adapter-watch registration.
    fn unregister_adapter_watch(&self, registration_id: u64) {
        self.registrations.lock().adapters.remove(&registration_id);
    }

    /// Register one scan resource with its initial snapshot.
    fn register_scan(
        &self,
        resource: &Arc<LinuxBluetoothScanResource>,
        known_devices: BTreeMap<String, BluetoothDeviceDescriptorValue>,
    ) -> u64 {
        let registration_id = self.next_registration_id.fetch_add(1, Ordering::AcqRel);
        let registration = LinuxBluetoothScanRegistration {
            resource: Arc::downgrade(resource),
            known_devices,
        };

        self.registrations
            .lock()
            .scans
            .insert(registration_id, registration);

        registration_id
    }

    /// Remove one scan registration.
    fn unregister_scan(&self, registration_id: u64) {
        self.registrations.lock().scans.remove(&registration_id);
    }

    /// Register one device session with its previous device state.
    fn register_device(
        &self,
        resource: &Arc<LinuxBluetoothDeviceResource>,
        previous_pair_state: Option<BluetoothPairState>,
        previous_cache: LinuxBluetoothGattCache,
    ) -> u64 {
        let registration_id = self.next_registration_id.fetch_add(1, Ordering::AcqRel);
        let registration = LinuxBluetoothDeviceRegistration {
            resource: Arc::downgrade(resource),
            previous_pair_state,
            previous_cache,
        };

        self.registrations
            .lock()
            .devices
            .insert(registration_id, registration);

        registration_id
    }

    /// Remove one device-session registration.
    fn unregister_device(&self, registration_id: u64) {
        self.registrations.lock().devices.remove(&registration_id);
    }

    /// Register one notification subscription.
    fn register_subscription(&self, resource: &Arc<LinuxBluetoothSubscriptionResource>) -> u64 {
        let registration_id = self.next_registration_id.fetch_add(1, Ordering::AcqRel);
        let registration = LinuxBluetoothSubscriptionRegistration {
            resource: Arc::downgrade(resource),
        };

        self.registrations
            .lock()
            .subscriptions
            .insert(registration_id, registration);

        registration_id
    }

    /// Remove one notification registration.
    fn unregister_subscription(&self, registration_id: u64) {
        self.registrations
            .lock()
            .subscriptions
            .remove(&registration_id);
    }

    /// Dispatch one BlueZ signal to all live registrations.
    fn dispatch(&self, connection: &Connection, message: &zbus::Message) {
        let mut registrations = self.registrations.lock();

        // adapter watches
        registrations.adapters.retain(|_, registration| {
            let Some(resource) = registration.resource.upgrade() else {
                return false;
            };

            handle_adapter_message(
                &resource,
                connection,
                message,
                &mut registration.known_adapters,
            )
        });

        // scan resources
        registrations.scans.retain(|_, registration| {
            let Some(resource) = registration.resource.upgrade() else {
                return false;
            };

            handle_scan_message(
                &resource,
                connection,
                message,
                &mut registration.known_devices,
            )
        });

        // device sessions
        registrations.devices.retain(|_, registration| {
            let Some(resource) = registration.resource.upgrade() else {
                return false;
            };

            handle_device_message(
                &resource,
                connection,
                message,
                &mut registration.previous_pair_state,
                &mut registration.previous_cache,
            )
        });

        // notification subscriptions
        registrations.subscriptions.retain(|_, registration| {
            let Some(resource) = registration.resource.upgrade() else {
                return false;
            };

            handle_subscription_message(&resource, message)
        });
    }

    /// Close all live queues when the shared signal thread exits.
    fn close_all(&self) {
        let mut registrations = self.registrations.lock();

        // adapter watches
        for registration in registrations.adapters.values() {
            let Some(resource) = registration.resource.upgrade() else {
                continue;
            };

            resource.event_queue.close();
        }

        // scan resources
        for registration in registrations.scans.values() {
            let Some(resource) = registration.resource.upgrade() else {
                continue;
            };

            resource.device_queue.close();
            resource.event_queue.close();
        }

        // device sessions
        for registration in registrations.devices.values() {
            let Some(resource) = registration.resource.upgrade() else {
                continue;
            };

            resource
                .event_queue
                .push_drop_oldest(disconnected_event(&resource.event_state));
            resource.event_queue.close();
        }

        // notification subscriptions
        for registration in registrations.subscriptions.values() {
            let Some(resource) = registration.resource.upgrade() else {
                continue;
            };

            resource.queue.close();
        }

        registrations.adapters.clear();
        registrations.scans.clear();
        registrations.devices.clear();
        registrations.subscriptions.clear();
    }
}

/// Spawn the shared BlueZ signal-dispatch loop.
fn spawn_signal_dispatch_loop(
    signal_runtime: Arc<LinuxBluetoothSignalRuntime>,
    connection: Connection,
) -> RuntimeResult<WorkerLoop> {
    WorkerLoop::open(
        "destack-bluez",
        "platform.service.spawn",
        LinuxBluetoothService::POLICY,
        move || {
            let shutdown_connection = connection.clone();
            let shutdown = Box::new(move || {
                let _ = shutdown_connection.close();
            });
            let run = Box::new(move || {
                let mut iterator = zbus::blocking::MessageIterator::from(&connection);

                while let Some(message) = iterator.next() {
                    let Ok(message) = message else {
                        continue;
                    };

                    signal_runtime.dispatch(&connection, &message);
                }

                signal_runtime.close_all();
                Ok(())
            });

            Ok((shutdown, run))
        },
    )
}

/// Resolve the shared BlueZ transport service or return a not-supported error.
pub(crate) fn linux_bluetooth_service(
    operation: &'static str,
) -> RuntimeResult<Arc<LinuxBluetoothService>> {
    LinuxBluetoothService::global(LinuxBluetoothService::new).map_err(|error| {
        RuntimeError::from(PlatformError::io_with(
            error
                .platform_error()
                .map(|platform_error| platform_error.code),
            None,
            None,
            Some(operation.to_string()),
            Some(String::from("linuxBluetoothService")),
            format!("linux bluetooth service unavailable: {error}"),
        ))
        .boxed()
    })
}
