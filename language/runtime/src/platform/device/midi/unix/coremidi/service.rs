use std::collections::BTreeMap;
use std::sync::{Arc, Weak};

use block2::{Block, RcBlock};
use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
use crate::runtime::service::Service;
use crate::runtime::service::executor::inline::InlineExecutor;
use crate::runtime::{ExecutionMode, ExecutionPolicy};

use super::abi::{
    MIDIClientCreateWithBlock, MIDIClientDispose, MIDIClientRef, MIDIGetNumberOfDestinations,
    MIDIGetNumberOfSources, MIDINotification, create_cf_string, release_cf,
};
use super::core::{CoreMidiEndpointOverride, CoreMidiEventDeliveryKind, CoreMidiEventRepository};
use super::event::dispatch_native_notification;

/// One process-global virtual endpoint override table.
type CoreMidiEndpointOverrideTable = BTreeMap<i32, CoreMidiEndpointOverride>;

/// One global CoreMIDI client service.
pub(crate) struct CoreMidiService {
    /// Direct executor for this service.
    executor: InlineExecutor,
    /// Shared CoreMIDI client used for ports and endpoints.
    operation_client: MIDIClientRef,
    /// Shared CoreMIDI client used for topology notifications.
    notify_client: MIDIClientRef,
    /// Registered native event subscriptions.
    pub(super) native_event_registry: Arc<Mutex<CoreMidiNativeEventRegistry>>,
    /// Runtime-owned virtual endpoint transport overrides.
    endpoint_overrides: Mutex<CoreMidiEndpointOverrideTable>,
    /// Retained notification block.
    _notify_block: CoreMidiNotifyBlock,
}

/// One retained CoreMIDI notification block.
struct CoreMidiNotifyBlock {
    /// Raw retained block pointer.
    raw: *mut Block<dyn Fn(*const MIDINotification)>,
}

/// One registry of native event subscriptions.
pub(super) struct CoreMidiNativeEventRegistry {
    /// Next registration id.
    pub(super) next_registration_id: u64,
    /// Registered native event subscriptions.
    pub(super) sessions: BTreeMap<u64, Weak<Mutex<CoreMidiEventRepository>>>,
}

unsafe impl Send for CoreMidiNotifyBlock {}
unsafe impl Sync for CoreMidiNotifyBlock {}

impl Drop for CoreMidiNotifyBlock {
    /// Release the retained notification block.
    fn drop(&mut self) {
        if self.raw.is_null() {
            return;
        }

        unsafe {
            drop(RcBlock::from_raw(self.raw));
        }
    }
}

impl Drop for CoreMidiService {
    /// Dispose the shared CoreMIDI client on process teardown.
    fn drop(&mut self) {
        // notification client disposal
        unsafe {
            let _ = MIDIClientDispose(self.notify_client);
        }

        // operational client disposal
        unsafe {
            let _ = MIDIClientDispose(self.operation_client);
        }
    }
}

impl CoreMidiService {
    /// Return one runtime-owned virtual endpoint transport override.
    pub(super) fn endpoint_override(&self, unique_id: i32) -> Option<CoreMidiEndpointOverride> {
        let _ = &self.executor;

        self.endpoint_overrides.lock().get(&unique_id).copied()
    }

    /// Register one runtime-owned virtual endpoint transport override.
    pub(super) fn insert_endpoint_override(
        &self,
        unique_id: i32,
        override_value: CoreMidiEndpointOverride,
    ) {
        let _ = &self.executor;

        self.endpoint_overrides
            .lock()
            .insert(unique_id, override_value);
    }

    /// Remove one runtime-owned virtual endpoint transport override.
    pub(super) fn remove_endpoint_override(&self, unique_id: i32) {
        let _ = &self.executor;

        self.endpoint_overrides.lock().remove(&unique_id);
    }

    /// Return the shared CoreMIDI operational client.
    pub(super) fn operation_client(&self) -> MIDIClientRef {
        let _ = &self.executor;

        self.operation_client
    }
}

impl Service for CoreMidiService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Inline);
}

/// Check whether the CoreMIDI API surface is reachable on this host.
pub(crate) fn check_core_midi_support(operation: &'static str) -> RuntimeResult<()> {
    let _ = operation;
    let _source_count = unsafe { MIDIGetNumberOfSources() };
    let _destination_count = unsafe { MIDIGetNumberOfDestinations() };

    Ok(())
}

/// Create one CoreMIDI client reference.
fn create_core_midi_client(
    name: &str,
    operation: &'static str,
    notify_block: Option<&Block<dyn Fn(*const MIDINotification)>>,
) -> RuntimeResult<MIDIClientRef> {
    let Some(name) = create_cf_string(name) else {
        return Err(core_platform::io_operation_error(
            operation,
            None,
            "failed to encode one CoreMIDI client name",
        ));
    };

    let mut client = 0u32;

    // client creation
    let status = unsafe { MIDIClientCreateWithBlock(name, &mut client, notify_block) };
    release_cf(name.cast());
    if status != 0 || client == 0 {
        return Err(core_platform::io_operation_error(
            operation,
            None,
            format!("MIDIClientCreateWithBlock failed with status {status}"),
        ));
    }

    Ok(client)
}

/// Return the shared CoreMIDI client service.
pub(crate) fn core_midi_service(operation: &'static str) -> RuntimeResult<Arc<CoreMidiService>> {
    CoreMidiService::global(|| {
        let native_event_registry = Arc::new(Mutex::new(CoreMidiNativeEventRegistry {
            next_registration_id: 1,
            sessions: BTreeMap::new(),
        }));
        let notify_block = core_midi_notify_block(native_event_registry.clone());
        let notify_client = create_core_midi_client(
            "Destack MIDI Notify",
            operation,
            Some(unsafe { &*notify_block.raw }),
        )?;
        let operation_client = create_core_midi_client("Destack MIDI", operation, None)?;

        Ok(CoreMidiService {
            executor: InlineExecutor::new("platform.device.midi.coremidi"),
            operation_client,
            notify_client,
            native_event_registry,
            endpoint_overrides: Mutex::new(BTreeMap::new()),
            _notify_block: notify_block,
        })
    })
}

/// Build one retained CoreMIDI notification block.
fn core_midi_notify_block(
    registry: Arc<Mutex<CoreMidiNativeEventRegistry>>,
) -> CoreMidiNotifyBlock {
    let block: RcBlock<dyn Fn(*const MIDINotification)> =
        RcBlock::new(move |notification: *const MIDINotification| {
            dispatch_native_notification(notification, &registry);
        });
    let raw = RcBlock::into_raw(block);

    CoreMidiNotifyBlock { raw }
}

/// Register one native event subscription.
pub(super) fn register_native_event_session(
    service: &Arc<CoreMidiService>,
    session: &Arc<Mutex<CoreMidiEventRepository>>,
) -> CoreMidiEventDeliveryKind {
    let mut registry = service.native_event_registry.lock();
    let registration_id = registry.next_registration_id;
    registry.next_registration_id = registry.next_registration_id.saturating_add(1);
    registry
        .sessions
        .insert(registration_id, Arc::downgrade(session));

    CoreMidiEventDeliveryKind::Native {
        registry: service.native_event_registry.clone(),
        registration_id,
    }
}

/// Remove one native event subscription from the shared registry.
pub(super) fn unregister_native_event_session(delivery_kind: &CoreMidiEventDeliveryKind) {
    let CoreMidiEventDeliveryKind::Native {
        registry,
        registration_id,
    } = delivery_kind
    else {
        return;
    };

    registry.lock().sessions.remove(registration_id);
}
