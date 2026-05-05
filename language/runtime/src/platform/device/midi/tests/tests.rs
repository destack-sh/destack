#![cfg_attr(
    not(any(target_os = "macos", target_os = "linux", windows)),
    allow(dead_code, unused_imports)
)]
#![cfg_attr(windows, allow(dead_code, unused_imports))]

#[cfg(any(target_os = "macos", target_os = "linux", windows))]
use std::sync::{Mutex, OnceLock};

#[path = "harness.rs"]
mod harness;

use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::core::BackendSupport;
use crate::platform::device::{
    MidiBackend, MidiBackendCapabilityFlags, MidiBackendDescriptor, MidiBackendDescriptorVm,
    MidiBackendSelectionPolicy, MidiDataFormat, MidiDataFormatFlags, MidiEvent,
    MidiEventDeliveryMode, MidiEventOverflowPolicy, MidiEventSource, MidiEventSubscriptionFlags,
    MidiEventSubscriptionOptions, MidiEventSubscriptionOptionsVm, MidiEventVm,
    MidiInputPortOpenOptions, MidiInputPortOpenOptionsVm, MidiInputRecord, MidiInputRecordVm,
    MidiOutputPortOpenOptions, MidiOutputPortOpenOptionsVm, MidiOutputRecord, MidiOutputRecordVm,
    MidiPortDescriptor, MidiPortDescriptorVm, MidiPortDirection, MidiPortDirectionFlags,
    MidiPortListFlags, MidiPortListOptions, MidiPortListOptionsVm, MidiProtocol, MidiProtocolFlags,
    MidiRecordFraming, MidiVirtualInputCreateOptions, MidiVirtualInputCreateOptionsVm,
    MidiVirtualOutputCreateOptions, MidiVirtualOutputCreateOptionsVm,
};
use crate::platform::{NativeArray, PlatformError, VmArray, VmSlice};
use crate::runtime::BindingCallContext;
pub(crate) use crate::tests::platform::{assert_ok_or_expected_error, assert_platform_error_codes};
use crate::tests::runtime::TestRuntime;
pub(crate) use harness::HarnessValue;

/// Test harness context used by tests.
pub(crate) struct MidiHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(super) call_context: &'call BindingCallContext,
    /// VM context when running VM bindings.
    pub(super) vm_context: Option<*mut ()>,
}

/// Native midi harness.
pub(crate) struct NativeMidiHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeMidiHarness {
    /// Create a new native midi harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM midi harness.
pub(crate) struct VmMidiHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmMidiHarness {
    /// Create a new VM midi harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum MidiHarnessHandle {
    /// Native midi harness.
    Native(NativeMidiHarness),
    /// VM midi harness.
    Vm(VmMidiHarness),
}

impl MidiHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&mut self, callback: F) -> R
    where
        F: for<'call> FnOnce(MidiHarnessContext<'call>) -> R,
    {
        match self {
            MidiHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(MidiHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            MidiHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::BindingContext<'_> as *mut ();
                        callback(MidiHarnessContext {
                            call_context,
                            vm_context: Some(vm_context),
                        })
                    })
            }
        }
    }

    /// Run one callback that returns a runtime result.
    pub(crate) fn run<F>(&mut self, callback: F)
    where
        F: for<'call> FnOnce(MidiHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("midi harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&mut MidiHarnessHandle),
{
    let mut native = MidiHarnessHandle::Native(NativeMidiHarness::new());
    callback(&mut native);
    let mut vm = MidiHarnessHandle::Vm(VmMidiHarness::new());
    callback(&mut vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(MidiHarnessContext<'call>) -> RuntimeResult<()>,
{
    #[cfg(any(target_os = "macos", target_os = "linux", windows))]
    let _guard = {
        let global_lock = midi_test_lock();
        global_lock
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    };

    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}

/// Return whether one backend support state allows live host execution.
pub(crate) fn support_allows_host_execution(support: BackendSupport) -> bool {
    matches!(support, BackendSupport::Available)
}

/// One decoded backend descriptor row used by tests.
pub(crate) type BackendDescriptorRow = (
    MidiBackend,
    String,
    BackendSupport,
    u16,
    MidiBackendCapabilityFlags,
    MidiDataFormatFlags,
    MidiProtocolFlags,
);

/// One decoded MIDI port identity row used by tests.
pub(crate) type PortDescriptorIdentityRow = (
    String,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
    bool,
    bool,
);

#[cfg(any(target_os = "macos", target_os = "linux", windows))]
/// Return one process-global serialization lock for backend-global MIDI tests.
fn midi_test_lock() -> &'static Mutex<()> {
    static MIDI_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    MIDI_TEST_LOCK.get_or_init(|| Mutex::new(()))
}

/// Return the mutable VM context when the harness is running in VM mode.
pub(crate) fn vm_context_mut<'a>(
    context: &'a MidiHarnessContext<'a>,
) -> Option<&'a mut vm::BindingContext<'a>> {
    context
        .vm_context
        .map(|context| unsafe { &mut *(context as *mut vm::BindingContext<'_>) })
}

/// Build one harness string payload for native and VM binding calls.
pub(crate) fn harness_string(
    context: &mut MidiHarnessContext<'_>,
    value: &str,
) -> RuntimeResult<HarnessValue<NativeStringRef, vm::StringHandle>> {
    match vm_context_mut(context) {
        Some(vm_context) => Ok(HarnessValue::Vm(vm::StringHandle::new(
            vm_context
                .intern_string(value)
                .expect("vm test string should intern"),
        ))),
        None => Ok(HarnessValue::Native(
            context.call_context.store_string(value),
        )),
    }
}

/// Build default MIDI port-list options for the current engine mode.
pub(crate) fn harness_port_list_options(
    context: &mut MidiHarnessContext<'_>,
) -> HarnessValue<MidiPortListOptions, MidiPortListOptionsVm> {
    harness_port_list_options_with_flags(context, MidiPortListFlags(0))
}

/// Build MIDI port-list options with one explicit flag set.
pub(crate) fn harness_port_list_options_with_flags(
    context: &mut MidiHarnessContext<'_>,
    flags: MidiPortListFlags,
) -> HarnessValue<MidiPortListOptions, MidiPortListOptionsVm> {
    harness_port_list_options_for_backend_with_policy(
        context,
        MidiBackend::Auto,
        MidiBackendSelectionPolicy::AllowFallback,
        flags,
    )
}

/// Build MIDI port-list options for one explicit backend selector.
pub(crate) fn harness_port_list_options_for_backend(
    context: &mut MidiHarnessContext<'_>,
    backend: MidiBackend,
    flags: MidiPortListFlags,
) -> HarnessValue<MidiPortListOptions, MidiPortListOptionsVm> {
    harness_port_list_options_for_backend_with_policy(
        context,
        backend,
        MidiBackendSelectionPolicy::Strict,
        flags,
    )
}

/// Build MIDI port-list options for one backend selector and selection policy.
pub(crate) fn harness_port_list_options_for_backend_with_policy(
    context: &mut MidiHarnessContext<'_>,
    backend: MidiBackend,
    backend_policy: MidiBackendSelectionPolicy,
    flags: MidiPortListFlags,
) -> HarnessValue<MidiPortListOptions, MidiPortListOptionsVm> {
    let options = MidiPortListOptions {
        backend,
        backend_policy,
        flags,
    };

    context
        .harness_value_from(options)
        .expect("midi port-list options should encode")
}

/// Build default MIDI input-open options for the current engine mode.
pub(crate) fn harness_input_open_options(
    context: &mut MidiHarnessContext<'_>,
) -> HarnessValue<MidiInputPortOpenOptions, MidiInputPortOpenOptionsVm> {
    harness_input_open_options_for_transport(context, None, None)
}

/// Build MIDI input-open options for one requested transport shape.
pub(crate) fn harness_input_open_options_for_transport(
    context: &mut MidiHarnessContext<'_>,
    data_format: Option<MidiDataFormat>,
    protocol: Option<MidiProtocol>,
) -> HarnessValue<MidiInputPortOpenOptions, MidiInputPortOpenOptionsVm> {
    harness_input_open_options_for_backend_transport(
        context,
        MidiBackend::Auto,
        data_format,
        protocol,
    )
}

/// Build MIDI input-open options for one backend and requested transport shape.
pub(crate) fn harness_input_open_options_for_backend_transport(
    context: &mut MidiHarnessContext<'_>,
    backend: MidiBackend,
    data_format: Option<MidiDataFormat>,
    protocol: Option<MidiProtocol>,
) -> HarnessValue<MidiInputPortOpenOptions, MidiInputPortOpenOptionsVm> {
    harness_input_open_options_for_backend_transport_with_policy(
        context,
        backend,
        MidiBackendSelectionPolicy::Strict,
        data_format,
        protocol,
    )
}

/// Build MIDI input-open options for one backend, policy, and requested transport shape.
pub(crate) fn harness_input_open_options_for_backend_transport_with_policy(
    context: &mut MidiHarnessContext<'_>,
    backend: MidiBackend,
    backend_policy: MidiBackendSelectionPolicy,
    data_format: Option<MidiDataFormat>,
    protocol: Option<MidiProtocol>,
) -> HarnessValue<MidiInputPortOpenOptions, MidiInputPortOpenOptionsVm> {
    let options = MidiInputPortOpenOptions {
        backend,
        backend_policy,
        data_format,
        protocol,
        queue_capacity: 0,
    };

    context
        .harness_value_from(options)
        .expect("midi input-open options should encode")
}

/// Build default MIDI output-open options for the current engine mode.
pub(crate) fn harness_output_open_options(
    context: &mut MidiHarnessContext<'_>,
) -> HarnessValue<MidiOutputPortOpenOptions, MidiOutputPortOpenOptionsVm> {
    harness_output_open_options_for_transport(context, None, None)
}

/// Build MIDI output-open options for one requested transport shape.
pub(crate) fn harness_output_open_options_for_transport(
    context: &mut MidiHarnessContext<'_>,
    data_format: Option<MidiDataFormat>,
    protocol: Option<MidiProtocol>,
) -> HarnessValue<MidiOutputPortOpenOptions, MidiOutputPortOpenOptionsVm> {
    harness_output_open_options_for_backend_transport(
        context,
        MidiBackend::Auto,
        data_format,
        protocol,
    )
}

/// Build MIDI output-open options for one backend and requested transport shape.
pub(crate) fn harness_output_open_options_for_backend_transport(
    context: &mut MidiHarnessContext<'_>,
    backend: MidiBackend,
    data_format: Option<MidiDataFormat>,
    protocol: Option<MidiProtocol>,
) -> HarnessValue<MidiOutputPortOpenOptions, MidiOutputPortOpenOptionsVm> {
    harness_output_open_options_for_backend_transport_with_policy(
        context,
        backend,
        MidiBackendSelectionPolicy::Strict,
        data_format,
        protocol,
    )
}

/// Build MIDI output-open options for one backend, policy, and requested transport shape.
pub(crate) fn harness_output_open_options_for_backend_transport_with_policy(
    context: &mut MidiHarnessContext<'_>,
    backend: MidiBackend,
    backend_policy: MidiBackendSelectionPolicy,
    data_format: Option<MidiDataFormat>,
    protocol: Option<MidiProtocol>,
) -> HarnessValue<MidiOutputPortOpenOptions, MidiOutputPortOpenOptionsVm> {
    let options = MidiOutputPortOpenOptions {
        backend,
        backend_policy,
        data_format,
        protocol,
    };

    context
        .harness_value_from(options)
        .expect("midi output-open options should encode")
}

/// Build one outbound MIDI record batch for native and VM binding calls.
pub(crate) fn harness_output_records(
    context: &mut MidiHarnessContext<'_>,
    values: &[MidiOutputRecord],
) -> RuntimeResult<HarnessValue<NativeArray<MidiOutputRecord>, VmArray<MidiOutputRecordVm>>> {
    match vm_context_mut(context) {
        Some(vm_context) => {
            let mut vm_values = Vec::with_capacity(values.len());

            for value in values {
                let data = unsafe { value.data.as_slice()? };
                vm_values.push(MidiOutputRecordVm {
                    send_at_ns: value.send_at_ns,
                    data_format: value.data_format,
                    protocol: value.protocol,
                    framing: value.framing,
                    data: VmSlice::from_bytes(&mut vm_context.write(), data)
                        .expect("vm test byte slice should allocate"),
                });
            }

            Ok(HarnessValue::Vm(VmArray::from_values(
                &mut vm_context.write(),
                &vm_values,
            )?))
        }
        None => Ok(HarnessValue::Native(
            context.call_context.store_array(values.to_vec()),
        )),
    }
}

/// Build one minimal valid output record for one transport pair.
pub(crate) fn output_record_for_transport(
    context: &mut MidiHarnessContext<'_>,
    data_format: MidiDataFormat,
    protocol: MidiProtocol,
) -> MidiOutputRecord {
    output_record_for_transport_with_timestamp(context, data_format, protocol, None)
}

/// Build one minimal valid output record for one transport pair and optional deadline.
pub(crate) fn output_record_for_transport_with_timestamp(
    context: &mut MidiHarnessContext<'_>,
    data_format: MidiDataFormat,
    protocol: MidiProtocol,
    send_at_ns: Option<u64>,
) -> MidiOutputRecord {
    let data = match data_format {
        MidiDataFormat::Midi1Bytes => vec![0x90, 0x3C, 0x40],
        MidiDataFormat::Ump => vec![0x40, 0x90, 0x3C, 0x40],
    };

    MidiOutputRecord {
        send_at_ns,
        data_format,
        protocol: Some(protocol),
        framing: MidiRecordFraming::Complete,
        data: context.call_context.store_slice(data),
    }
}

/// Build virtual-input creation options for one backend and requested transport shape.
pub(crate) fn harness_virtual_input_create_options_for_backend_transport(
    context: &mut MidiHarnessContext<'_>,
    backend: MidiBackend,
    name: &str,
    data_format: MidiDataFormat,
    protocol: MidiProtocol,
) -> RuntimeResult<HarnessValue<MidiVirtualInputCreateOptions, MidiVirtualInputCreateOptionsVm>> {
    harness_virtual_input_create_options_for_backend_transport_with_policy(
        context,
        backend,
        MidiBackendSelectionPolicy::Strict,
        name,
        data_format,
        protocol,
    )
}

/// Build virtual-input creation options for one backend, policy, and requested transport shape.
pub(crate) fn harness_virtual_input_create_options_for_backend_transport_with_policy(
    context: &mut MidiHarnessContext<'_>,
    backend: MidiBackend,
    backend_policy: MidiBackendSelectionPolicy,
    name: &str,
    data_format: MidiDataFormat,
    protocol: MidiProtocol,
) -> RuntimeResult<HarnessValue<MidiVirtualInputCreateOptions, MidiVirtualInputCreateOptionsVm>> {
    match vm_context_mut(context) {
        Some(vm_context) => Ok(HarnessValue::Vm(MidiVirtualInputCreateOptionsVm {
            backend,
            backend_policy,
            name: vm::StringHandle::new(
                vm_context
                    .intern_string(name)
                    .expect("vm test string should intern"),
            ),
            manufacturer: None,
            model: None,
            version: None,
            data_format,
            protocol,
            queue_capacity: 0,
        })),
        None => Ok(HarnessValue::Native(MidiVirtualInputCreateOptions {
            backend,
            backend_policy,
            name: context.call_context.store_string(name),
            manufacturer: None,
            model: None,
            version: None,
            data_format,
            protocol,
            queue_capacity: 0,
        })),
    }
}

/// Build virtual-output creation options for one backend and requested transport shape.
pub(crate) fn harness_virtual_output_create_options_for_backend_transport(
    context: &mut MidiHarnessContext<'_>,
    backend: MidiBackend,
    name: &str,
    data_format: MidiDataFormat,
    protocol: MidiProtocol,
) -> RuntimeResult<HarnessValue<MidiVirtualOutputCreateOptions, MidiVirtualOutputCreateOptionsVm>> {
    harness_virtual_output_create_options_for_backend_transport_with_policy(
        context,
        backend,
        MidiBackendSelectionPolicy::Strict,
        name,
        data_format,
        protocol,
    )
}

/// Build virtual-output creation options for one backend, policy, and requested transport shape.
pub(crate) fn harness_virtual_output_create_options_for_backend_transport_with_policy(
    context: &mut MidiHarnessContext<'_>,
    backend: MidiBackend,
    backend_policy: MidiBackendSelectionPolicy,
    name: &str,
    data_format: MidiDataFormat,
    protocol: MidiProtocol,
) -> RuntimeResult<HarnessValue<MidiVirtualOutputCreateOptions, MidiVirtualOutputCreateOptionsVm>> {
    match vm_context_mut(context) {
        Some(vm_context) => Ok(HarnessValue::Vm(MidiVirtualOutputCreateOptionsVm {
            backend,
            backend_policy,
            name: vm::StringHandle::new(
                vm_context
                    .intern_string(name)
                    .expect("vm test string should intern"),
            ),
            manufacturer: None,
            model: None,
            version: None,
            data_format,
            protocol,
        })),
        None => Ok(HarnessValue::Native(MidiVirtualOutputCreateOptions {
            backend,
            backend_policy,
            name: context.call_context.store_string(name),
            manufacturer: None,
            model: None,
            version: None,
            data_format,
            protocol,
        })),
    }
}

/// Build MIDI event-subscription options for the current engine mode.
pub(crate) fn harness_event_open_options(
    context: &mut MidiHarnessContext<'_>,
    flags: MidiEventSubscriptionFlags,
    direction_mask: MidiPortDirectionFlags,
    delivery_mode: MidiEventDeliveryMode,
) -> HarnessValue<MidiEventSubscriptionOptions, MidiEventSubscriptionOptionsVm> {
    harness_event_open_options_for_backend(
        context,
        MidiBackend::Auto,
        flags,
        direction_mask,
        delivery_mode,
    )
}

/// Build MIDI event-subscription options for one backend selector.
pub(crate) fn harness_event_open_options_for_backend(
    context: &mut MidiHarnessContext<'_>,
    backend: MidiBackend,
    flags: MidiEventSubscriptionFlags,
    direction_mask: MidiPortDirectionFlags,
    delivery_mode: MidiEventDeliveryMode,
) -> HarnessValue<MidiEventSubscriptionOptions, MidiEventSubscriptionOptionsVm> {
    harness_event_open_options_for_backend_with_policy(
        context,
        backend,
        MidiBackendSelectionPolicy::Strict,
        flags,
        direction_mask,
        delivery_mode,
    )
}

/// Build MIDI event-subscription options for one backend selector and selection policy.
pub(crate) fn harness_event_open_options_for_backend_with_policy(
    context: &mut MidiHarnessContext<'_>,
    backend: MidiBackend,
    backend_policy: MidiBackendSelectionPolicy,
    flags: MidiEventSubscriptionFlags,
    direction_mask: MidiPortDirectionFlags,
    delivery_mode: MidiEventDeliveryMode,
) -> HarnessValue<MidiEventSubscriptionOptions, MidiEventSubscriptionOptionsVm> {
    let options = MidiEventSubscriptionOptions {
        backend,
        backend_policy,
        flags,
        direction_mask,
        delivery_mode,
        overflow_policy: MidiEventOverflowPolicy::DropOldest,
        queue_capacity: 0,
        poll_interval_ns: 0,
    };

    context
        .harness_value_from(options)
        .expect("midi event-subscription options should encode")
}

/// Decode one backend descriptor list into full plain Rust rows.
pub(crate) fn decode_backend_descriptors_full(
    context: &mut MidiHarnessContext<'_>,
    value: HarnessValue<NativeSlice<MidiBackendDescriptor>, VmSlice<MidiBackendDescriptorVm>>,
) -> RuntimeResult<Vec<BackendDescriptorRow>> {
    match value {
        HarnessValue::Native(values) => {
            let values = unsafe { values.as_slice()? };
            let mut decoded = Vec::with_capacity(values.len());

            for value in values {
                let name = unsafe { value.name.as_str()? }.to_string();
                decoded.push((
                    value.backend,
                    name,
                    value.support,
                    value.priority,
                    value.capability_flags,
                    value.supported_data_formats,
                    value.supported_protocols,
                ));
            }

            Ok(decoded)
        }
        HarnessValue::Vm(values) => {
            let Some(vm_context) = vm_context_mut(context) else {
                return Err(RuntimeError::from(PlatformError::invalid_argument(
                    "missing vm context",
                ))
                .boxed());
            };

            let values = values.read_values(&vm_context.read())?;
            let mut decoded = Vec::with_capacity(values.len());

            for value in values {
                let name = vm_context
                    .string_ref(value.name)
                    .map_err(|error| RuntimeError::from(error).boxed())?
                    .as_str()
                    .to_string();
                decoded.push((
                    value.backend,
                    name,
                    value.support,
                    value.priority,
                    value.capability_flags,
                    value.supported_data_formats,
                    value.supported_protocols,
                ));
            }

            Ok(decoded)
        }
    }
}

/// Decode and return one backend descriptor row by selector.
#[cfg(any(target_os = "android", windows))]
pub(crate) fn backend_descriptor_row(
    context: &mut MidiHarnessContext<'_>,
    backend: MidiBackend,
) -> RuntimeResult<BackendDescriptorRow> {
    let descriptors = context.destack_device_midi_backend_list()?;
    let descriptors = decode_backend_descriptors_full(context, descriptors)?;

    descriptors
        .into_iter()
        .find(|row| row.0 == backend)
        .ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument(format!(
                "missing midi backend descriptor row for {backend:?}"
            )))
            .boxed()
        })
}

/// Decode listed input-port rows for one explicit backend.
#[cfg(any(target_os = "android", windows))]
pub(crate) fn listed_backend_input_rows(
    context: &mut MidiHarnessContext<'_>,
    backend: MidiBackend,
    flags: MidiPortListFlags,
) -> RuntimeResult<
    Vec<(
        String,
        String,
        bool,
        MidiDataFormatFlags,
        Option<MidiDataFormat>,
        MidiProtocolFlags,
        Option<MidiProtocol>,
    )>,
> {
    let options = harness_port_list_options_for_backend(context, backend, flags);
    let rows = context.destack_device_midi_input_port_list(options)?;

    decode_port_descriptors(context, rows)
}

/// Decode listed output-port rows for one explicit backend.
#[cfg(any(target_os = "android", windows))]
pub(crate) fn listed_backend_output_rows(
    context: &mut MidiHarnessContext<'_>,
    backend: MidiBackend,
    flags: MidiPortListFlags,
) -> RuntimeResult<
    Vec<(
        String,
        String,
        bool,
        MidiDataFormatFlags,
        Option<MidiDataFormat>,
        MidiProtocolFlags,
        Option<MidiProtocol>,
    )>,
> {
    let options = harness_port_list_options_for_backend(context, backend, flags);
    let rows = context.destack_device_midi_output_port_list(options)?;

    decode_port_descriptors(context, rows)
}

/// Return whether one advertised format mask contains one data format.
pub(crate) fn supports_data_format(
    supported_data_formats: MidiDataFormatFlags,
    data_format: MidiDataFormat,
) -> bool {
    let data_format_flag = 1u32 << (data_format as u32 - 1);

    supported_data_formats.0 & data_format_flag != 0
}

/// Return whether one advertised protocol mask contains one protocol.
pub(crate) fn supports_protocol(
    supported_protocols: MidiProtocolFlags,
    protocol: MidiProtocol,
) -> bool {
    let protocol_flag = 1u32 << (protocol as u32 - 1);

    supported_protocols.0 & protocol_flag != 0
}

/// Choose one exact transport pair from one advertised endpoint or backend shape.
pub(crate) fn preferred_transport_pair(
    supported_data_formats: MidiDataFormatFlags,
    default_data_format: Option<MidiDataFormat>,
    supported_protocols: MidiProtocolFlags,
    default_protocol: Option<MidiProtocol>,
) -> Option<(MidiDataFormat, MidiProtocol)> {
    let data_format = match default_data_format {
        Some(data_format) => data_format,
        None if supports_data_format(supported_data_formats, MidiDataFormat::Midi1Bytes) => {
            MidiDataFormat::Midi1Bytes
        }
        None if supports_data_format(supported_data_formats, MidiDataFormat::Ump) => {
            MidiDataFormat::Ump
        }
        None => return None,
    };

    let protocol = match default_protocol {
        Some(protocol) => protocol,
        None if supports_protocol(supported_protocols, MidiProtocol::Midi1) => MidiProtocol::Midi1,
        None if supports_protocol(supported_protocols, MidiProtocol::Midi2) => MidiProtocol::Midi2,
        None => return None,
    };

    if protocol == MidiProtocol::Midi2 && data_format != MidiDataFormat::Ump {
        return None;
    }

    Some((data_format, protocol))
}

/// Return whether one advertised transport shape can back one Web MIDI port.
pub(crate) fn supports_web_midi_transport(
    supported_data_formats: MidiDataFormatFlags,
    supported_protocols: MidiProtocolFlags,
) -> bool {
    supports_data_format(supported_data_formats, MidiDataFormat::Midi1Bytes)
        || (supports_data_format(supported_data_formats, MidiDataFormat::Ump)
            && supports_protocol(supported_protocols, MidiProtocol::Midi1))
}

/// Choose one exact transport pair that a Web MIDI wrapper can expose.
pub(crate) fn preferred_web_midi_transport_pair(
    supported_data_formats: MidiDataFormatFlags,
    supported_protocols: MidiProtocolFlags,
) -> Option<(MidiDataFormat, MidiProtocol)> {
    if supports_data_format(supported_data_formats, MidiDataFormat::Midi1Bytes) {
        return Some((MidiDataFormat::Midi1Bytes, MidiProtocol::Midi1));
    }

    if supports_data_format(supported_data_formats, MidiDataFormat::Ump)
        && supports_protocol(supported_protocols, MidiProtocol::Midi1)
    {
        return Some((MidiDataFormat::Ump, MidiProtocol::Midi1));
    }

    None
}

/// Decode one MIDI port descriptor into one plain Rust row.
pub(crate) fn decode_port_descriptor(
    context: &mut MidiHarnessContext<'_>,
    value: HarnessValue<MidiPortDescriptor, MidiPortDescriptorVm>,
) -> RuntimeResult<(
    String,
    Option<String>,
    String,
    bool,
    bool,
    Option<MidiDataFormat>,
    Option<MidiProtocol>,
)> {
    match value {
        HarnessValue::Native(value) => Ok((
            unsafe { value.id.as_str()? }.to_string(),
            value
                .backend_id
                .map(|value| unsafe { value.as_str() })
                .transpose()?
                .map(str::to_string),
            unsafe { value.name.as_str()? }.to_string(),
            value.is_virtual,
            value.is_connected,
            value.default_data_format,
            value.default_protocol,
        )),
        HarnessValue::Vm(value) => {
            let Some(vm_context) = vm_context_mut(context) else {
                return Err(RuntimeError::from(PlatformError::invalid_argument(
                    "missing vm context",
                ))
                .boxed());
            };

            let id = vm_context
                .string_ref(value.id)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string();
            let backend_id = value
                .backend_id
                .map(|value| {
                    vm_context
                        .string_ref(value)
                        .map_err(|error| RuntimeError::from(error).boxed())
                        .map(|value| value.as_str().to_string())
                })
                .transpose()?;
            let name = vm_context
                .string_ref(value.name)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string();

            Ok((
                id,
                backend_id,
                name,
                value.is_virtual,
                value.is_connected,
                value.default_data_format,
                value.default_protocol,
            ))
        }
    }
}

/// Decode one MIDI port descriptor into one Web MIDI identity shape.
pub(crate) fn decode_port_descriptor_identity(
    context: &mut MidiHarnessContext<'_>,
    value: HarnessValue<MidiPortDescriptor, MidiPortDescriptorVm>,
) -> RuntimeResult<PortDescriptorIdentityRow> {
    match value {
        HarnessValue::Native(value) => Ok((
            unsafe { value.id.as_str()? }.to_string(),
            value
                .manufacturer
                .map(|value| unsafe { value.as_str() })
                .transpose()?
                .map(str::to_string),
            unsafe { value.name.as_str()? }.to_string(),
            value
                .version
                .map(|value| unsafe { value.as_str() })
                .transpose()?
                .map(str::to_string),
            value
                .backend_id
                .map(|value| unsafe { value.as_str() })
                .transpose()?
                .map(str::to_string),
            value.is_virtual,
            value.is_connected,
        )),
        HarnessValue::Vm(value) => {
            let Some(vm_context) = vm_context_mut(context) else {
                return Err(RuntimeError::from(PlatformError::invalid_argument(
                    "missing vm context",
                ))
                .boxed());
            };

            let id = vm_context
                .string_ref(value.id)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string();
            let manufacturer = value
                .manufacturer
                .map(|value| {
                    vm_context
                        .string_ref(value)
                        .map_err(|error| RuntimeError::from(error).boxed())
                        .map(|value| value.as_str().to_string())
                })
                .transpose()?;
            let name = vm_context
                .string_ref(value.name)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string();
            let version = value
                .version
                .map(|value| {
                    vm_context
                        .string_ref(value)
                        .map_err(|error| RuntimeError::from(error).boxed())
                        .map(|value| value.as_str().to_string())
                })
                .transpose()?;
            let backend_id = value
                .backend_id
                .map(|value| {
                    vm_context
                        .string_ref(value)
                        .map_err(|error| RuntimeError::from(error).boxed())
                        .map(|value| value.as_str().to_string())
                })
                .transpose()?;

            Ok((
                id,
                manufacturer,
                name,
                version,
                backend_id,
                value.is_virtual,
                value.is_connected,
            ))
        }
    }
}

/// Decode one MIDI port descriptor list into one Web MIDI identity shape.
pub(crate) fn decode_port_descriptor_identities(
    context: &mut MidiHarnessContext<'_>,
    value: HarnessValue<NativeSlice<MidiPortDescriptor>, VmSlice<MidiPortDescriptorVm>>,
) -> RuntimeResult<Vec<PortDescriptorIdentityRow>> {
    match value {
        HarnessValue::Native(values) => {
            let values = unsafe { values.as_slice()? };
            let mut decoded = Vec::with_capacity(values.len());

            for value in values {
                decoded.push(decode_port_descriptor_identity(
                    context,
                    context.harness_value(*value),
                )?);
            }

            Ok(decoded)
        }
        HarnessValue::Vm(values) => {
            let Some(vm_context) = vm_context_mut(context) else {
                return Err(RuntimeError::from(PlatformError::invalid_argument(
                    "missing vm context",
                ))
                .boxed());
            };

            let values = values.read_values(&vm_context.read())?;
            let mut decoded = Vec::with_capacity(values.len());

            for value in values {
                decoded.push(decode_port_descriptor_identity(
                    context,
                    context.harness_value_vm(value),
                )?);
            }

            Ok(decoded)
        }
    }
}

/// Decode one MIDI port descriptor list into plain Rust rows.
pub(crate) fn decode_port_descriptors(
    context: &mut MidiHarnessContext<'_>,
    value: HarnessValue<NativeSlice<MidiPortDescriptor>, VmSlice<MidiPortDescriptorVm>>,
) -> RuntimeResult<
    Vec<(
        String,
        String,
        bool,
        MidiDataFormatFlags,
        Option<MidiDataFormat>,
        MidiProtocolFlags,
        Option<MidiProtocol>,
    )>,
> {
    match value {
        HarnessValue::Native(values) => {
            let values = unsafe { values.as_slice()? };
            let mut decoded = Vec::with_capacity(values.len());

            for value in values {
                let id = unsafe { value.id.as_str()? }.to_string();
                let name = unsafe { value.name.as_str()? }.to_string();
                decoded.push((
                    id,
                    name,
                    value.is_connected,
                    value.supported_data_formats,
                    value.default_data_format,
                    value.supported_protocols,
                    value.default_protocol,
                ));
            }

            Ok(decoded)
        }
        HarnessValue::Vm(values) => {
            let Some(vm_context) = vm_context_mut(context) else {
                return Err(RuntimeError::from(PlatformError::invalid_argument(
                    "missing vm context",
                ))
                .boxed());
            };

            let values = values.read_values(&vm_context.read())?;
            let mut decoded = Vec::with_capacity(values.len());

            for value in values {
                let id = vm_context
                    .string_ref(value.id)
                    .map_err(|error| RuntimeError::from(error).boxed())?
                    .as_str()
                    .to_string();
                let name = vm_context
                    .string_ref(value.name)
                    .map_err(|error| RuntimeError::from(error).boxed())?
                    .as_str()
                    .to_string();
                decoded.push((
                    id,
                    name,
                    value.is_connected,
                    value.supported_data_formats,
                    value.default_data_format,
                    value.supported_protocols,
                    value.default_protocol,
                ));
            }

            Ok(decoded)
        }
    }
}

/// Decode one MIDI input record into one plain Rust row.
pub(crate) fn decode_input_record(
    context: &mut MidiHarnessContext<'_>,
    value: HarnessValue<MidiInputRecord, MidiInputRecordVm>,
) -> RuntimeResult<(
    u64,
    Option<String>,
    MidiDataFormat,
    Option<MidiProtocol>,
    Vec<u8>,
)> {
    match value {
        HarnessValue::Native(value) => Ok((
            value.received_at_ns,
            value
                .source_id
                .map(|value| unsafe { value.as_str() })
                .transpose()?
                .map(str::to_string),
            value.data_format,
            value.protocol,
            unsafe { value.data.as_slice()? }.to_vec(),
        )),
        HarnessValue::Vm(value) => {
            let Some(vm_context) = vm_context_mut(context) else {
                return Err(RuntimeError::from(PlatformError::invalid_argument(
                    "missing vm context",
                ))
                .boxed());
            };

            let source_id = value
                .source_id
                .map(|value| {
                    vm_context
                        .string_ref(value)
                        .map_err(|error| RuntimeError::from(error).boxed())
                        .map(|value| value.as_str().to_string())
                })
                .transpose()?;

            Ok((
                value.received_at_ns,
                source_id,
                value.data_format,
                value.protocol,
                value.data.read_bytes(&vm_context.read())?,
            ))
        }
    }
}

/// One decoded MIDI event summary used by tests.
pub(crate) struct DecodedMidiEvent {
    /// Variant kind string.
    pub kind: String,
    /// Event source selector.
    pub source: MidiEventSource,
    /// Port direction when present.
    pub direction: Option<MidiPortDirection>,
    /// Runtime id when present.
    pub id: Option<String>,
    /// Whether the descriptor is virtual when present.
    pub is_virtual: Option<bool>,
}

/// Decode one MIDI event into one plain Rust summary.
pub(crate) fn decode_event(
    context: &mut MidiHarnessContext<'_>,
    value: HarnessValue<MidiEvent, MidiEventVm>,
) -> RuntimeResult<DecodedMidiEvent> {
    match value {
        HarnessValue::Native(value) => match value {
            MidiEvent::MidiPortAddedEvent(event) => Ok(DecodedMidiEvent {
                kind: unsafe { event.kind.as_str()? }.to_string(),
                source: event.metadata.source,
                direction: Some(event.direction),
                id: Some(unsafe { event.descriptor.id.as_str()? }.to_string()),
                is_virtual: Some(event.descriptor.is_virtual),
            }),
            MidiEvent::MidiPortRemovedEvent(event) => Ok(DecodedMidiEvent {
                kind: unsafe { event.kind.as_str()? }.to_string(),
                source: event.metadata.source,
                direction: Some(event.direction),
                id: Some(unsafe { event.id.as_str()? }.to_string()),
                is_virtual: None,
            }),
            MidiEvent::MidiPortChangedEvent(event) => Ok(DecodedMidiEvent {
                kind: unsafe { event.kind.as_str()? }.to_string(),
                source: event.metadata.source,
                direction: Some(event.direction),
                id: Some(unsafe { event.descriptor.id.as_str()? }.to_string()),
                is_virtual: Some(event.descriptor.is_virtual),
            }),
            MidiEvent::MidiBackendDisconnectedEvent(event) => Ok(DecodedMidiEvent {
                kind: unsafe { event.kind.as_str()? }.to_string(),
                source: event.metadata.source,
                direction: None,
                id: None,
                is_virtual: None,
            }),
        },
        HarnessValue::Vm(value) => {
            let Some(vm_context) = vm_context_mut(context) else {
                return Err(RuntimeError::from(PlatformError::invalid_argument(
                    "missing vm context",
                ))
                .boxed());
            };

            match value {
                MidiEventVm::MidiPortAddedEvent(event) => Ok(DecodedMidiEvent {
                    kind: vm_context
                        .string_ref(event.kind)
                        .map_err(|error| RuntimeError::from(error).boxed())?
                        .as_str()
                        .to_string(),
                    source: event.metadata.source,
                    direction: Some(event.direction),
                    id: Some(
                        vm_context
                            .string_ref(event.descriptor.id)
                            .map_err(|error| RuntimeError::from(error).boxed())?
                            .as_str()
                            .to_string(),
                    ),
                    is_virtual: Some(event.descriptor.is_virtual),
                }),
                MidiEventVm::MidiPortRemovedEvent(event) => Ok(DecodedMidiEvent {
                    kind: vm_context
                        .string_ref(event.kind)
                        .map_err(|error| RuntimeError::from(error).boxed())?
                        .as_str()
                        .to_string(),
                    source: event.metadata.source,
                    direction: Some(event.direction),
                    id: Some(
                        vm_context
                            .string_ref(event.id)
                            .map_err(|error| RuntimeError::from(error).boxed())?
                            .as_str()
                            .to_string(),
                    ),
                    is_virtual: None,
                }),
                MidiEventVm::MidiPortChangedEvent(event) => Ok(DecodedMidiEvent {
                    kind: vm_context
                        .string_ref(event.kind)
                        .map_err(|error| RuntimeError::from(error).boxed())?
                        .as_str()
                        .to_string(),
                    source: event.metadata.source,
                    direction: Some(event.direction),
                    id: Some(
                        vm_context
                            .string_ref(event.descriptor.id)
                            .map_err(|error| RuntimeError::from(error).boxed())?
                            .as_str()
                            .to_string(),
                    ),
                    is_virtual: Some(event.descriptor.is_virtual),
                }),
                MidiEventVm::MidiBackendDisconnectedEvent(event) => Ok(DecodedMidiEvent {
                    kind: vm_context
                        .string_ref(event.kind)
                        .map_err(|error| RuntimeError::from(error).boxed())?
                        .as_str()
                        .to_string(),
                    source: event.metadata.source,
                    direction: None,
                    id: None,
                    is_virtual: None,
                }),
            }
        }
    }
}

/// Decode one MIDI event list into plain Rust summaries.
pub(crate) fn decode_events(
    context: &mut MidiHarnessContext<'_>,
    value: HarnessValue<NativeSlice<MidiEvent>, VmSlice<MidiEventVm>>,
) -> RuntimeResult<Vec<DecodedMidiEvent>> {
    match value {
        HarnessValue::Native(values) => {
            let values = unsafe { values.as_slice()? };
            let mut decoded = Vec::with_capacity(values.len());

            for value in values {
                decoded.push(decode_event(context, HarnessValue::Native(*value))?);
            }

            Ok(decoded)
        }
        HarnessValue::Vm(values) => {
            let Some(vm_context) = vm_context_mut(context) else {
                return Err(RuntimeError::from(PlatformError::invalid_argument(
                    "missing vm context",
                ))
                .boxed());
            };

            let values = values.read_values(&vm_context.read())?;
            let mut decoded = Vec::with_capacity(values.len());

            for value in values {
                decoded.push(decode_event(context, HarnessValue::Vm(value))?);
            }

            Ok(decoded)
        }
    }
}
