#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{NativeArray, NativeSlice, NativeStringRef, PlatformError};

use crate::runtime::BindingCallContext;

use crate::platform::midi::{
    MidiBackendDescriptor, MidiEvent, MidiEventSubscriptionOptions, MidiInputPortOpenOptions,
    MidiInputRecord, MidiOutputPortOpenOptions, MidiOutputRecord, MidiPortDescriptor,
    MidiPortListOptions, MidiVirtualInputCreateOptions, MidiVirtualOutputCreateOptions,
};
use crate::platform::{core, resource};

/// List host MIDI backends.
/// Enumerate backend selectors, support state, and backend-level feature flags.
/// # Platform
/// Unix and Windows.
/// # Errors
/// Returns ioNotFound, ioInvalidData, notSupported.
/// # Security
/// Requires `midi.port`.
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_midi_backend_list(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<MidiBackendDescriptor>,
) -> RuntimeResult<()> {
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.midi.backend.list")).boxed())
}

/// Close one MIDI topology event subscription.
/// Close one MIDI event subscription and release backend notification resources.
/// Pending events are discarded.
/// # Platform
/// Unix and Windows.
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
/// # Security
/// Requires `midi.observe`.
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_midi_event_close(
    _binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.midi.event.close")).boxed())
}

/// Open one MIDI topology event subscription.
/// Open one backend event subscription for MIDI topology changes.
/// Subscription routing and queue depth follow host backend behavior.
/// # Platform
/// Unix and Windows.
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
/// # Security
/// Requires `midi.observe`.
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_midi_event_open(
    _binding: &BindingCallContext,
    out: *mut resource::MidiEventHandle,
    options: MidiEventSubscriptionOptions,
) -> RuntimeResult<()> {
    let _ = (out, options);

    Err(RuntimeError::from(PlatformError::not_supported("destack.midi.event.open")).boxed())
}

/// Wait for one MIDI topology event.
/// Wait for one pending event from one subscription queue.
/// Timeout uses nanoseconds in the runtime monotonic domain.
/// # Platform
/// Unix and Windows.
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
/// # Security
/// Requires `midi.observe`.
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_midi_event_read(
    _binding: &BindingCallContext,
    out: *mut MidiEvent,
    handle: resource::MidiEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (out, handle, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported("destack.midi.event.read")).boxed())
}

/// Wait for one batch of MIDI topology events.
/// Wait for pending events from one subscription queue and return up to `maxEvents`.
/// Timeout uses nanoseconds in the runtime monotonic domain.
/// # Platform
/// Unix and Windows.
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
/// # Security
/// Requires `midi.observe`.
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_midi_event_read_batch(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<MidiEvent>,
    handle: resource::MidiEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (out, handle, maxevents, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported("destack.midi.event.readBatch")).boxed())
}

/// Poll one MIDI topology event without blocking.
/// Poll one pending event from one subscription queue.
/// Empty queue state is reported through ioWouldBlock.
/// # Platform
/// Unix and Windows.
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
/// # Security
/// Requires `midi.observe`.
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_midi_event_try_read(
    _binding: &BindingCallContext,
    out: *mut MidiEvent,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.midi.event.tryRead")).boxed())
}

/// Poll one batch of MIDI topology events without blocking.
/// Poll pending events from one subscription queue and return up to `maxEvents`.
/// Empty queue state is reported through ioWouldBlock.
/// # Platform
/// Unix and Windows.
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
/// # Security
/// Requires `midi.observe`.
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_midi_event_try_read_batch(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<MidiEvent>,
    handle: resource::MidiEventHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    let _ = (out, handle, maxevents);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.midi.event.tryReadBatch",
    ))
    .boxed())
}

/// Close one opened MIDI input endpoint.
/// Close one opened MIDI input session and release host resources.
/// # Platform
/// Unix and Windows.
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
/// # Security
/// Requires `midi.port`.
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_midi_input_port_close(
    _binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.midi.input.port.close",
    ))
    .boxed())
}

/// List available MIDI input endpoints.
/// Enumerate host MIDI input endpoints for one selected backend.
/// Endpoint visibility and ordering follow host MIDI subsystem behavior.
/// # Platform
/// Unix and Windows.
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
/// # Security
/// Requires `midi.port`.
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_midi_input_port_list(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<MidiPortDescriptor>,
    options: MidiPortListOptions,
) -> RuntimeResult<()> {
    let _ = (out, options);

    Err(RuntimeError::from(PlatformError::not_supported("destack.midi.input.port.list")).boxed())
}

/// Open one MIDI input endpoint.
/// Open one host MIDI input endpoint for queued transport-record reads.
/// Endpoint open behavior follows host MIDI session policy and sharing semantics.
/// # Platform
/// Unix and Windows.
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
/// # Security
/// Requires `midi.port`.
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_midi_input_port_open(
    _binding: &BindingCallContext,
    out: *mut resource::MidiInputPortHandle,
    id: NativeStringRef,
    options: MidiInputPortOpenOptions,
) -> RuntimeResult<()> {
    let _ = (out, id, options);

    Err(RuntimeError::from(PlatformError::not_supported("destack.midi.input.port.open")).boxed())
}

/// Describe one opened MIDI input endpoint.
/// Resolve the current descriptor for one opened MIDI input session.
/// # Platform
/// Unix and Windows.
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
/// # Security
/// Requires `midi.port`.
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_midi_input_port_descriptor(
    _binding: &BindingCallContext,
    out: *mut MidiPortDescriptor,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.midi.input.port.descriptor",
    ))
    .boxed())
}

/// Read one MIDI input record.
/// Wait for one queued inbound MIDI transport record from one opened input endpoint.
/// Timeout uses nanoseconds in the runtime monotonic domain.
/// # Platform
/// Unix and Windows.
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
/// # Security
/// Requires `midi.read`.
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_midi_input_read(
    _binding: &BindingCallContext,
    out: *mut MidiInputRecord,
    handle: resource::MidiInputPortHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (out, handle, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported("destack.midi.input.read")).boxed())
}

/// Read one batch of MIDI input records.
/// Wait for queued inbound MIDI transport records from one opened input endpoint and return up to `maxRecords`.
/// Timeout uses nanoseconds in the runtime monotonic domain.
/// # Platform
/// Unix and Windows.
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
/// # Security
/// Requires `midi.read`.
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_midi_input_read_batch(
    _binding: &BindingCallContext,
    out: *mut NativeArray<MidiInputRecord>,
    handle: resource::MidiInputPortHandle,
    maxrecords: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (out, handle, maxrecords, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported("destack.midi.input.readBatch")).boxed())
}

/// Poll one MIDI input record without blocking.
/// Poll one pending inbound MIDI transport record from one opened input endpoint.
/// Empty queue state is reported through ioWouldBlock.
/// # Platform
/// Unix and Windows.
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
/// # Security
/// Requires `midi.read`.
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_midi_input_try_read(
    _binding: &BindingCallContext,
    out: *mut MidiInputRecord,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.midi.input.tryRead")).boxed())
}

/// Poll one batch of MIDI input records without blocking.
/// Poll pending inbound MIDI transport records from one opened input endpoint and return up to `maxRecords`.
/// Empty queue state is reported through ioWouldBlock.
/// # Platform
/// Unix and Windows.
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
/// # Security
/// Requires `midi.read`.
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_midi_input_try_read_batch(
    _binding: &BindingCallContext,
    out: *mut NativeArray<MidiInputRecord>,
    handle: resource::MidiInputPortHandle,
    maxrecords: u32,
) -> RuntimeResult<()> {
    let _ = (out, handle, maxrecords);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.midi.input.tryReadBatch",
    ))
    .boxed())
}

/// Create one virtual MIDI input endpoint.
/// Create one host-visible virtual MIDI input endpoint and return one opened input handle for reads.
/// # Platform
/// Unix and Windows.
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
/// # Security
/// Requires `midi.virtual`.
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_midi_input_virtual_create(
    _binding: &BindingCallContext,
    out: *mut resource::MidiInputPortHandle,
    options: MidiVirtualInputCreateOptions,
) -> RuntimeResult<()> {
    let _ = (out, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.midi.input.virtual.create",
    ))
    .boxed())
}

/// Close one opened MIDI output endpoint.
/// Close one opened MIDI output session and release host resources.
/// # Platform
/// Unix and Windows.
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
/// # Security
/// Requires `midi.port`.
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_midi_output_port_close(
    _binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.midi.output.port.close",
    ))
    .boxed())
}

/// List available MIDI output endpoints.
/// Enumerate host MIDI output endpoints for one selected backend.
/// Endpoint visibility and ordering follow host MIDI subsystem behavior.
/// # Platform
/// Unix and Windows.
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
/// # Security
/// Requires `midi.port`.
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_midi_output_port_list(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<MidiPortDescriptor>,
    options: MidiPortListOptions,
) -> RuntimeResult<()> {
    let _ = (out, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.midi.output.port.list",
    ))
    .boxed())
}

/// Open one MIDI output endpoint.
/// Open one host MIDI output endpoint for outbound transport-record writes.
/// Endpoint open behavior follows host MIDI session policy and sharing semantics.
/// # Platform
/// Unix and Windows.
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
/// # Security
/// Requires `midi.port`.
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_midi_output_port_open(
    _binding: &BindingCallContext,
    out: *mut resource::MidiOutputPortHandle,
    id: NativeStringRef,
    options: MidiOutputPortOpenOptions,
) -> RuntimeResult<()> {
    let _ = (out, id, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.midi.output.port.open",
    ))
    .boxed())
}

/// Describe one opened MIDI output endpoint.
/// Resolve the current descriptor for one opened MIDI output session.
/// # Platform
/// Unix and Windows.
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
/// # Security
/// Requires `midi.port`.
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_midi_output_port_descriptor(
    _binding: &BindingCallContext,
    out: *mut MidiPortDescriptor,
    handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.midi.output.port.descriptor",
    ))
    .boxed())
}

/// Create one virtual MIDI output endpoint.
/// Create one host-visible virtual MIDI output endpoint and return one opened output handle for writes.
/// # Platform
/// Unix and Windows.
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
/// # Security
/// Requires `midi.virtual`.
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_midi_output_virtual_create(
    _binding: &BindingCallContext,
    out: *mut resource::MidiOutputPortHandle,
    options: MidiVirtualOutputCreateOptions,
) -> RuntimeResult<()> {
    let _ = (out, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.midi.output.virtual.create",
    ))
    .boxed())
}

/// Write one batch of outbound MIDI records.
/// Submit one batch of outbound MIDI transport records to one opened output endpoint.
/// Scheduled timestamps are advisory unless the backend advertises scheduled output support.
/// # Platform
/// Unix and Windows.
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
/// # Security
/// Requires `midi.write`.
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_midi_output_write(
    _binding: &BindingCallContext,
    out: *mut u32,
    handle: resource::MidiOutputPortHandle,
    records: NativeArray<MidiOutputRecord>,
) -> RuntimeResult<()> {
    let _ = (out, handle, records);

    Err(RuntimeError::from(PlatformError::not_supported("destack.midi.output.write")).boxed())
}
