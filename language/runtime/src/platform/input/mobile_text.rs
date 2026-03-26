#![cfg_attr(test, allow(dead_code))]

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, LazyLock, Mutex};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::abi::text::{
    HostTextGeometryRequest, HostTextOpenRequestPayload, HostTextStateRequestPayload,
    host_text_rectangle, host_text_transform,
};
#[cfg(target_os = "android")]
use crate::host::android::abi::text::ffi::{
    destack_host_android_text_close, destack_host_android_text_open,
    destack_host_android_text_set_geometry, destack_host_android_text_set_state,
};
use crate::host::core::{HOST_STATUS_FAILED, HostStatus};
#[cfg(target_os = "ios")]
use crate::host::ios::abi::text::ffi::{
    destack_host_ios_text_close, destack_host_ios_text_open, destack_host_ios_text_set_geometry,
    destack_host_ios_text_set_state,
};
use crate::platform::input::{
    InputTextGeometry, InputTextRange, InputTextSessionConfig, InputTextSessionEvent,
    InputTextSessionEventValue, InputTextSessionState, InputTextSessionStateEventValue,
    InputTextSessionStateValue, validation as input_validation,
};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{NativeAbiCodec, PlatformError, core as core_platform, resource};
use crate::runtime::BindingCallContext;
use parking_lot::{Condvar, Mutex as ParkingLotMutex};

/// Resource-table label for active mobile text-session entries.
const TEXT_SESSION_RESOURCE_LABEL: &str = "input.text.session";

/// Stable device identifier used for mobile text-session events.
const MOBILE_TEXT_DEVICE_ID: &str = "host.text.mobile";

/// Shared registry for active mobile text sessions.
static MOBILE_TEXT_SESSIONS: LazyLock<Mutex<HashMap<MobileTextSessionKey, MobileTextSession>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Waitable queued-event signal for one mobile text session.
#[derive(Debug, Default)]
struct TextSessionEventSignal {
    /// Monotonic wake generation for this session queue.
    generation: ParkingLotMutex<u64>,
    /// Wake signal for queued session events.
    wake: Condvar,
}

/// One runtime-scoped mobile text-session key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct MobileTextSessionKey {
    /// The owning runtime session handle.
    runtime_id: u64,
    /// The stable text-session resource identifier.
    session_id: u64,
}

/// One active mobile text-session payload.
#[derive(Debug)]
struct MobileTextSession {
    /// The current text-geometry hint.
    geometry: Option<InputTextGeometry>,
    /// The current renderer or host text state.
    state: InputTextSessionStateValue,
    /// The pending text-session events.
    events: VecDeque<InputTextSessionEventValue>,
    /// The next event sequence number.
    next_sequence: u64,
    /// Waitable signal for queued session events.
    event_signal: Arc<TextSessionEventSignal>,
    /// The target window hint for session metadata.
    target_window: Option<resource::WindowHandle>,
}

/// Finalizer for one mobile text session.
struct MobileTextSessionFinalizer {
    /// The owning runtime identifier.
    runtime_id: u64,
    /// The text session identifier.
    session_id: u64,
}

impl crate::platform::resource::ResourceFinalizer for MobileTextSessionFinalizer {
    /// Close one leaked mobile text session.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        remove_session(self.runtime_id, self.session_id);

        let _ = unsafe { host_text_close(self.runtime_id, self.session_id) };
    }
}

/// Build one io-not-found error for one missing text session.
fn text_session_not_found(
    operation: &'static str,
    session: resource::InputTextSessionHandle,
) -> Box<RuntimeError> {
    core_platform::io_not_found(operation, format!("text session {} not found", session.0.0))
}

/// Build one key for one runtime and one text handle.
fn session_key(runtime_id: u64, session: resource::InputTextSessionHandle) -> MobileTextSessionKey {
    MobileTextSessionKey {
        runtime_id,
        session_id: session.0.0,
    }
}

/// Remove one mobile text session from the shared registry.
fn remove_session(runtime_id: u64, session_id: u64) {
    let mut sessions = MOBILE_TEXT_SESSIONS.lock().unwrap();
    sessions.remove(&MobileTextSessionKey {
        runtime_id,
        session_id,
    });
}

/// Wake queued-event readers after one session event is appended.
fn notify_text_session_event(signal: &TextSessionEventSignal) {
    let mut generation = signal.generation.lock();
    *generation = generation.wrapping_add(1);
    signal.wake.notify_all();
}

/// Resolve one mobile text session mutably.
fn with_mobile_session<R>(
    runtime_id: u64,
    session: resource::InputTextSessionHandle,
    operation: &'static str,
    body: impl FnOnce(&mut MobileTextSession) -> RuntimeResult<R>,
) -> RuntimeResult<R> {
    let mut sessions = MOBILE_TEXT_SESSIONS.lock().unwrap();
    let Some(session) = sessions.get_mut(&session_key(runtime_id, session)) else {
        return Err(text_session_not_found(operation, session));
    };

    body(session)
}

/// Pop one queued mobile text-session event when one is available.
fn pop_mobile_session_event(
    binding: &BindingCallContext,
    runtime_id: u64,
    session: resource::InputTextSessionHandle,
    operation: &'static str,
) -> RuntimeResult<Option<InputTextSessionEvent>> {
    let event = with_mobile_session(runtime_id, session, operation, |session| {
        Ok(session.events.pop_front())
    })?;

    Ok(event.map(|event| InputTextSessionEvent::from_value(binding, event)))
}

/// Wait for one queued mobile text-session event.
fn wait_for_mobile_session_event(
    binding: &BindingCallContext,
    runtime_id: u64,
    session: resource::InputTextSessionHandle,
    operation: &'static str,
) -> RuntimeResult<InputTextSessionEvent> {
    loop {
        // fast path
        if let Some(event) = pop_mobile_session_event(binding, runtime_id, session, operation)? {
            return Ok(event);
        }

        let signal = with_mobile_session(runtime_id, session, operation, |session| {
            Ok(Arc::clone(&session.event_signal))
        })?;
        let mut generation = signal.generation.lock();
        let observed_generation = *generation;

        // avoid sleeping when one event raced in after the fast path
        if let Some(event) = pop_mobile_session_event(binding, runtime_id, session, operation)? {
            return Ok(event);
        }

        while *generation == observed_generation {
            signal.wake.wait(&mut generation);
        }
    }
}

/// Validate one text range against one UTF-16 text length.
fn validate_text_range(
    field: &'static str,
    range: InputTextRange,
    text_length: u32,
) -> RuntimeResult<()> {
    // reject inverted ranges
    if range.start_offset > range.end_offset {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "text range start must not be greater than the end",
        ))
        .boxed());
    }

    // reject out-of-bounds ranges
    if range.end_offset > text_length {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "text range exceeds the current UTF-16 text length",
        ))
        .boxed());
    }

    Ok(())
}

/// Return the UTF-16 code-unit length for one string.
fn utf16_length(text: &str) -> u32 {
    text.encode_utf16().count() as u32
}

/// Validate one full text-session state payload.
fn validate_text_session_state(state: &InputTextSessionStateValue) -> RuntimeResult<()> {
    let text_length = utf16_length(&state.text);

    // selection
    validate_text_range("state.selection", state.selection, text_length)?;

    // composing
    if let Some(composing) = state.composing {
        validate_text_range("state.composing", composing, text_length)?;
    }

    Ok(())
}

/// Validate one mobile text-open request.
fn validate_text_open(
    config: InputTextSessionConfig,
    state: &InputTextSessionStateValue,
) -> RuntimeResult<()> {
    // mobile text follows one focused host target, not explicit window routing
    if input_validation::has_explicit_window_target(config.target) {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.input.text.open")).boxed(),
        );
    }

    validate_text_session_state(state)
}

/// Map one mobile host status into one runtime result.
fn host_status_result(
    status: u32,
    operation: &'static str,
    action: &'static str,
) -> RuntimeResult<()> {
    let Some(status) = HostStatus::from_code(status) else {
        return Err(RuntimeError::from(PlatformError::invalid_data(format!(
            "{operation}: mobile text host {action} failed with unknown status code {status}"
        )))
        .boxed());
    };

    match status {
        HostStatus::Ok => Ok(()),
        HostStatus::NotSupported => {
            Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
        }
        HostStatus::InvalidArgument => {
            Err(RuntimeError::from(PlatformError::invalid_argument(operation)).boxed())
        }
        HostStatus::NotFound => Err(core_platform::io_not_found(
            operation,
            format!("mobile text host {action} could not resolve one object"),
        )),
        HostStatus::PermissionDenied => Err(core_platform::io_operation_error(
            operation,
            Some(crate::platform::diagnostic::PlatformErrorCode::IoPermissionDenied),
            format!("mobile text host {action} denied permission"),
        )),
        HostStatus::BufferTooSmall => {
            Err(RuntimeError::from(PlatformError::invalid_data(format!(
            "{operation}: mobile text host {action} reported one unexpectedly small output buffer"
        )))
            .boxed())
        }
        HostStatus::Failed => Err(RuntimeError::from(PlatformError::invalid_data(format!(
            "{operation}: mobile text host {action} failed"
        )))
        .boxed()),
        HostStatus::WouldBlock => Err(core_platform::io_would_block(
            operation,
            format!("mobile text host {action} would block"),
        )),
    }
}

/// Call the target mobile host text-open callback.
unsafe fn host_text_open(
    _runtime_id: u64,
    _request: crate::host::abi::text::HostTextOpenRequest,
) -> u32 {
    #[cfg(target_os = "android")]
    {
        return unsafe { destack_host_android_text_open(_runtime_id, _request) };
    }

    #[cfg(target_os = "ios")]
    {
        return unsafe { destack_host_ios_text_open(_runtime_id, _request) };
    }

    #[allow(unreachable_code)]
    HOST_STATUS_FAILED
}

/// Call the target mobile host text-close callback.
unsafe fn host_text_close(_runtime_id: u64, _session_id: u64) -> u32 {
    #[cfg(target_os = "android")]
    {
        return unsafe { destack_host_android_text_close(_runtime_id, _session_id) };
    }

    #[cfg(target_os = "ios")]
    {
        return unsafe { destack_host_ios_text_close(_runtime_id, _session_id) };
    }

    #[allow(unreachable_code)]
    HOST_STATUS_FAILED
}

/// Call the target mobile host text-geometry callback.
unsafe fn host_text_set_geometry(_runtime_id: u64, _request: HostTextGeometryRequest) -> u32 {
    #[cfg(target_os = "android")]
    {
        return unsafe { destack_host_android_text_set_geometry(_runtime_id, _request) };
    }

    #[cfg(target_os = "ios")]
    {
        return unsafe { destack_host_ios_text_set_geometry(_runtime_id, _request) };
    }

    #[allow(unreachable_code)]
    HOST_STATUS_FAILED
}

/// Call the target mobile host text-state callback.
unsafe fn host_text_set_state(
    _runtime_id: u64,
    _request: crate::host::abi::text::HostTextStateRequest,
) -> u32 {
    #[cfg(target_os = "android")]
    {
        return unsafe { destack_host_android_text_set_state(_runtime_id, _request) };
    }

    #[cfg(target_os = "ios")]
    {
        return unsafe { destack_host_ios_text_set_state(_runtime_id, _request) };
    }

    #[allow(unreachable_code)]
    HOST_STATUS_FAILED
}

/// Open one mobile text session.
pub(crate) unsafe fn destack_input_text_open(
    binding: &BindingCallContext,
    out: *mut resource::InputTextSessionHandle,
    config: InputTextSessionConfig,
    state: InputTextSessionState,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // state
    let state = unsafe { state.into_value()? };
    validate_text_open(config, &state)?;

    let runtime_id = binding.agent().runtime_id.0;
    let entry = ResourceEntry::new(ResourceKind::InputTextSession)
        .with_label(TEXT_SESSION_RESOURCE_LABEL)
        .with_backing(crate::platform::resource::ResourceBacking::Host);
    let resource_id =
        binding
            .agent()
            .resources
            .insert(binding.world(), entry, Some(binding.engine()));
    let session = resource::InputTextSessionHandle(resource_id);
    let session_id = resource_id.0;

    // register the session before the host opens it
    {
        let mut sessions = MOBILE_TEXT_SESSIONS.lock().unwrap();
        sessions.insert(
            session_key(runtime_id, session),
            MobileTextSession {
                geometry: None,
                state: state.clone(),
                events: VecDeque::new(),
                next_sequence: 1,
                event_signal: Arc::new(TextSessionEventSignal::default()),
                target_window: config.target.window,
            },
        );
    }

    // attach the leak finalizer after the session id exists
    binding
        .agent()
        .resources
        .with_entry_mut(resource_id, |entry| {
            entry.finalizer = Some(Box::new(MobileTextSessionFinalizer {
                runtime_id,
                session_id,
            }));
        });

    let request = HostTextOpenRequestPayload::new(session_id, config, &state);
    let status = unsafe { host_text_open(runtime_id, request.abi()) };
    if let Err(error) = host_status_result(status, "destack.input.text.open", "open") {
        remove_session(runtime_id, session_id);
        let _ =
            binding
                .agent()
                .resources
                .remove(binding.world(), resource_id, Some(binding.engine()));
        return Err(error);
    }

    unsafe {
        out.write(session);
    }

    Ok(())
}

/// Close one mobile text session.
pub(crate) unsafe fn destack_input_text_close(
    binding: &BindingCallContext,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    let exists = binding.agent().resources.with_entry(session.0, |entry| {
        entry.kind == ResourceKind::InputTextSession
            && entry.label.as_deref() == Some(TEXT_SESSION_RESOURCE_LABEL)
    });
    if exists != Some(true) {
        return Err(text_session_not_found("destack.input.text.close", session));
    }

    let runtime_id = binding.agent().runtime_id.0;
    let status = unsafe { host_text_close(runtime_id, session.0.0) };
    host_status_result(status, "destack.input.text.close", "close")?;

    remove_session(runtime_id, session.0.0);
    let _ = binding
        .agent()
        .resources
        .remove(binding.world(), session.0, Some(binding.engine()));

    Ok(())
}

/// Get one mobile text-session geometry.
pub(crate) unsafe fn destack_input_text_get_geometry(
    binding: &BindingCallContext,
    out: *mut InputTextGeometry,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let runtime_id = binding.agent().runtime_id.0;
    let geometry = with_mobile_session(
        runtime_id,
        session,
        "destack.input.text.getGeometry",
        |session| {
            session.geometry.ok_or_else(|| {
                RuntimeError::from(PlatformError::io_with(
                    Some(crate::platform::diagnostic::PlatformErrorCode::IoWouldBlock),
                    None,
                    None,
                    Some("destack.input.text.getGeometry".to_string()),
                    None,
                    "text geometry is not available yet".to_string(),
                ))
                .boxed()
            })
        },
    )?;

    unsafe {
        out.write(geometry);
    }

    Ok(())
}

/// Set one mobile text-session geometry.
pub(crate) unsafe fn destack_input_text_set_geometry(
    binding: &BindingCallContext,
    session: resource::InputTextSessionHandle,
    geometry: InputTextGeometry,
) -> RuntimeResult<()> {
    let runtime_id = binding.agent().runtime_id.0;

    with_mobile_session(
        runtime_id,
        session,
        "destack.input.text.setGeometry",
        |_session| Ok(()),
    )?;

    let status = unsafe {
        host_text_set_geometry(
            runtime_id,
            HostTextGeometryRequest {
                session_id: session.0.0,
                local_to_target_transform: host_text_transform(geometry.local_to_target_transform),
                editor_rectangle: host_text_rectangle(geometry.editor_rectangle),
                has_caret_rectangle: geometry.caret_rectangle.is_some(),
                caret_rectangle: geometry
                    .caret_rectangle
                    .map(host_text_rectangle)
                    .unwrap_or_default(),
                has_composing_rectangle: geometry.composing_rectangle.is_some(),
                composing_rectangle: geometry
                    .composing_rectangle
                    .map(host_text_rectangle)
                    .unwrap_or_default(),
            },
        )
    };
    host_status_result(status, "destack.input.text.setGeometry", "set geometry")?;

    with_mobile_session(
        runtime_id,
        session,
        "destack.input.text.setGeometry",
        |session| {
            session.geometry = Some(geometry);
            Ok(())
        },
    )?;

    Ok(())
}

/// Set one mobile text-session state.
pub(crate) unsafe fn destack_input_text_set_state(
    binding: &BindingCallContext,
    session: resource::InputTextSessionHandle,
    state: InputTextSessionState,
) -> RuntimeResult<()> {
    let runtime_id = binding.agent().runtime_id.0;
    let state = unsafe { state.into_value()? };
    validate_text_session_state(&state)?;

    with_mobile_session(
        runtime_id,
        session,
        "destack.input.text.setState",
        |_session| Ok(()),
    )?;

    let request = HostTextStateRequestPayload::new(session.0.0, &state);
    let status = unsafe { host_text_set_state(runtime_id, request.abi()) };

    host_status_result(status, "destack.input.text.setState", "set state")?;

    with_mobile_session(
        runtime_id,
        session,
        "destack.input.text.setState",
        |session| {
            session.state = state;
            Ok(())
        },
    )?;

    Ok(())
}

/// Read one mobile text-session event.
pub(crate) unsafe fn destack_input_text_read_event(
    binding: &BindingCallContext,
    out: *mut InputTextSessionEvent,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let runtime_id = binding.agent().runtime_id.0;
    let event = wait_for_mobile_session_event(
        binding,
        runtime_id,
        session,
        "destack.input.text.readEvent",
    )?;

    unsafe {
        out.write(event);
    }

    Ok(())
}

/// Poll one mobile text-session event.
pub(crate) unsafe fn destack_input_text_try_read_event(
    binding: &BindingCallContext,
    out: *mut InputTextSessionEvent,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let runtime_id = binding.agent().runtime_id.0;
    let Some(event) = pop_mobile_session_event(
        binding,
        runtime_id,
        session,
        "destack.input.text.tryReadEvent",
    )?
    else {
        return Err(core_platform::io_would_block(
            "destack.input.text.tryReadEvent",
            "no pending text-session event is available",
        ));
    };

    unsafe {
        out.write(event);
    }

    Ok(())
}

/// Queue one mobile text-state event from one native host callback.
pub(crate) fn notify_text_input_state(
    runtime_id: u64,
    session_id: u64,
    state: InputTextSessionStateValue,
) -> RuntimeResult<()> {
    validate_text_session_state(&state)?;

    let mut sessions = MOBILE_TEXT_SESSIONS.lock().unwrap();
    let Some(session) = sessions.get_mut(&MobileTextSessionKey {
        runtime_id,
        session_id,
    }) else {
        return Err(core_platform::io_not_found(
            "destack.input.text.notifyState",
            format!("text session {session_id} not found"),
        ));
    };

    // host state is authoritative for host-originated edits
    session.state = state.clone();

    let event =
        InputTextSessionEventValue::InputTextSessionStateEvent(InputTextSessionStateEventValue {
            kind: "stateChanged".to_string(),
            metadata: crate::platform::input::InputEventMetadataValue {
                timestamp_ns: core_platform::monotonic_now_ns(),
                sequence: session.next_sequence,
                device_id: MOBILE_TEXT_DEVICE_ID.to_string(),
                target_window: session.target_window,
            },
            state,
        });

    session.next_sequence += 1;
    session.events.push_back(event);
    notify_text_session_event(&session.event_signal);

    Ok(())
}
