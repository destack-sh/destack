use std::collections::{HashMap, HashSet};
use std::ffi::CString;
use std::mem::MaybeUninit;
use std::os::unix::io::RawFd;

#[cfg(target_os = "linux")]
use super::linux as input_linux;
#[cfg(target_os = "macos")]
use super::macos as input_macos;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::input::{
    InputCompositionEventPayload, InputDeviceDescriptor, InputDeviceEventPayload, InputDeviceKind,
    InputEvent, InputEventAction, InputEventKind, InputEventPayload, InputGamepadEventPayload,
    InputKeyEventPayload, InputPointerButtonEventPayload, InputPointerMotionEventPayload,
    InputReadMode, InputScrollEventPayload, InputSensorEffectiveConfig, InputSensorEventPayload,
    InputSensorKind, InputTextEventPayload, InputTextInputArea, InputTextInputType,
    InputTouchEventPayload,
};
use crate::platform::resource::{ResourceFinalizer, ResourceId, ResourceKind};
use crate::platform::{PlatformError, core as core_platform, resource};
use crate::runtime::BindingCallContext;

/// Resource-table label for opened input-device entries.
pub(super) const INPUT_RESOURCE_LABEL: &str = "input.device";
/// Canonical tty path used for terminal-backed input streams.
pub(super) const UNIX_INPUT_TTY_PATH: &str = "/dev/tty";
/// Stable runtime identifier for tty-backed input streams.
const UNIX_INPUT_TTY_ID: &str = "tty:stdin";
/// Alias accepted for tty-backed input streams.
const UNIX_INPUT_TTY_ALIAS: &str = "tty";
/// Alias accepted for stdin-backed input streams.
const UNIX_INPUT_STDIN_ALIAS: &str = "stdin";
/// Display name for tty-backed input streams.
const UNIX_INPUT_TTY_NAME: &str = "unix terminal input";
/// Empty text payload for non-text events.
pub(super) const UNIX_INPUT_EMPTY_TEXT: &str = "";

/// Build one zeroed payload shell for event-kind projection.
pub(super) fn empty_unix_event_payload(context: &BindingCallContext) -> InputEventPayload {
    let empty_text = context.store_string(UNIX_INPUT_EMPTY_TEXT);
    InputEventPayload {
        key: InputKeyEventPayload {
            action: InputEventAction::Cancel,
            backend_code: 0,
            backend_scan_code: 0,
            backend_value: 0,
            modifiers: 0,
            repeat: false,
        },
        pointer_motion: InputPointerMotionEventPayload {
            x: 0.0,
            y: 0.0,
            buttons: 0,
            modifiers: 0,
        },
        pointer_button: InputPointerButtonEventPayload {
            action: InputEventAction::Cancel,
            backend_code: 0,
            backend_value: 0,
            x: 0.0,
            y: 0.0,
            modifiers: 0,
        },
        scroll: InputScrollEventPayload {
            wheel_x: 0.0,
            wheel_y: 0.0,
            x: 0.0,
            y: 0.0,
            modifiers: 0,
        },
        touch: InputTouchEventPayload {
            action: InputEventAction::Cancel,
            contact_id: 0,
            x: 0.0,
            y: 0.0,
            pressure: 0.0,
        },
        gamepad: InputGamepadEventPayload {
            action: InputEventAction::Cancel,
            backend_code: 0,
            backend_value: 0,
        },
        text: InputTextEventPayload { text: empty_text },
        device: InputDeviceEventPayload {
            action: InputEventAction::Cancel,
            backend_code: 0,
            backend_value: 0,
        },
        sensor: InputSensorEventPayload {
            action: InputEventAction::Cancel,
            backend_code: 0,
            backend_value: 0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        composition: InputCompositionEventPayload {
            action: InputEventAction::Cancel,
            text: empty_text,
            selection_start: 0,
            selection_end: 0,
        },
    }
}

/// Build one typed input event from one prepared payload.
pub(super) fn build_unix_input_event(
    context: &BindingCallContext,
    kind: InputEventKind,
    timestamp_ns: u64,
    sequence: u64,
    device_id: &str,
    payload: InputEventPayload,
) -> InputEvent {
    InputEvent {
        kind,
        timestamp_ns,
        sequence,
        device_id: context.store_string(device_id),
        payload,
    }
}

/// Backend kind for one Unix input binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum UnixInputBackend {
    /// Platform-specific host input backend.
    Platform,
    /// Unix tty byte-stream input endpoint.
    UnixTerminal,
}

/// Resource payload for one opened Unix input endpoint.
#[derive(Debug)]
pub(super) struct UnixInputBinding {
    /// Descriptor when backend is file-descriptor based.
    pub(super) descriptor: Option<RawFd>,
    /// Active backend kind.
    pub(super) backend: UnixInputBackend,
    /// Current read mode.
    pub(super) read_mode: InputReadMode,
    /// Whether text input is active for this handle.
    pub(super) text_active: bool,
    /// The active text input type for this handle.
    pub(super) text_input_type: InputTextInputType,
    /// The current text input area hint for this handle.
    pub(super) text_area: InputTextInputArea,
    /// Optional gamepad player-index override for this handle.
    pub(super) gamepad_player_index_override: Option<u8>,
    /// Whether relative pointer mode is enabled for this handle.
    pub(super) relative_mode_enabled: bool,
    /// Last pointer x position used for relative delta projection.
    pub(super) last_pointer_x: f64,
    /// Last pointer y position used for relative delta projection.
    pub(super) last_pointer_y: f64,
    /// Set of enabled sensor lanes for this handle.
    pub(super) sensor_enabled_kinds: HashSet<InputSensorKind>,
    /// Effective sensor configurations keyed by sensor lane.
    pub(super) sensor_effective_configs: HashMap<InputSensorKind, InputSensorEffectiveConfig>,
    /// Stable runtime device identifier used in emitted events.
    pub(super) device_id: String,
    /// Classified device kind for backend-specific event mapping.
    pub(super) device_kind: InputDeviceKind,
    /// Next per-handle event sequence number.
    pub(super) next_sequence: u64,
    /// Current Linux modifier-state bitset for this stream.
    #[cfg(target_os = "linux")]
    pub(super) linux_modifiers: u32,
    /// Current Linux pointer-button bitset for this stream.
    #[cfg(target_os = "linux")]
    pub(super) linux_pointer_buttons: u32,
    /// Linux uploaded rumble effect id for this handle, when one is active.
    #[cfg(target_os = "linux")]
    pub(super) linux_active_rumble_effect_id: Option<i16>,
    /// Original terminal mode snapshot for tty-backed streams.
    pub(super) terminal_original_mode: Option<libc::termios>,
    /// Cached macOS session polling state.
    #[cfg(target_os = "macos")]
    pub(super) macos_state: Option<input_macos::MacosInputState>,
}

/// Normalized open specification for Unix input identifiers.
#[derive(Debug, Clone)]
pub(super) struct UnixInputOpenSpec {
    /// Canonical input path or logical identifier.
    pub(super) path: String,
    /// Backend kind selected for the identifier.
    pub(super) backend: UnixInputBackend,
    /// Stable runtime device identifier.
    pub(super) device_id: String,
}

/// Finalizer payload for descriptor-backed Unix input resources.
#[derive(Debug)]
pub(super) struct InputDeviceFinalizer {
    /// Descriptor that must be closed when the resource is removed.
    pub(super) fd: RawFd,
    /// Terminal mode snapshot to restore before closing the descriptor.
    pub(super) restore_terminal_mode: Option<libc::termios>,
}

impl ResourceFinalizer for InputDeviceFinalizer {
    /// Close one input device descriptor during resource finalization.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        // restore terminal mode before close when a tty snapshot is available
        if let Some(restore_mode) = self.restore_terminal_mode {
            unsafe {
                libc::tcsetattr(self.fd, libc::TCSANOW, &restore_mode);
            }
        }

        // close the descriptor regardless of restore result
        unsafe {
            libc::close(self.fd);
        }
    }
}

/// Build io-not-found for one missing Unix input handle.
pub(super) fn input_not_found(
    operation: &'static str,
    handle: resource::InputDeviceHandle,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoNotFound),
        None,
        None,
        Some(operation.to_string()),
        None,
        format!("input device handle {} not found", handle.0.0),
    ))
    .boxed()
}

/// Resolve one Unix input binding from the resource table.
pub(super) fn resolve_unix_input_binding(
    context: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<UnixInputBinding> {
    // resolve resource entry and validate payload shape
    let binding = context.runtime().resources.with_entry(handle.0, |entry| {
        if entry.kind != ResourceKind::Input {
            return None;
        }

        if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
            return None;
        }

        let binding = entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<UnixInputBinding>())
            .map(|binding| UnixInputBinding {
                descriptor: binding.descriptor,
                backend: binding.backend,
                read_mode: binding.read_mode,
                text_active: binding.text_active,
                text_input_type: binding.text_input_type,
                text_area: binding.text_area,
                gamepad_player_index_override: binding.gamepad_player_index_override,
                relative_mode_enabled: binding.relative_mode_enabled,
                last_pointer_x: binding.last_pointer_x,
                last_pointer_y: binding.last_pointer_y,
                sensor_enabled_kinds: binding.sensor_enabled_kinds.clone(),
                sensor_effective_configs: binding.sensor_effective_configs.clone(),
                device_id: binding.device_id.clone(),
                device_kind: binding.device_kind,
                next_sequence: binding.next_sequence,
                #[cfg(target_os = "linux")]
                linux_modifiers: binding.linux_modifiers,
                #[cfg(target_os = "linux")]
                linux_pointer_buttons: binding.linux_pointer_buttons,
                #[cfg(target_os = "linux")]
                linux_active_rumble_effect_id: binding.linux_active_rumble_effect_id,
                terminal_original_mode: binding.terminal_original_mode,
                #[cfg(target_os = "macos")]
                macos_state: None,
            })?;

        Some(binding)
    });

    match binding.flatten() {
        Some(binding) => Ok(binding),
        None => Err(input_not_found(operation, handle)),
    }
}

/// Normalize one Unix input identifier into one backend open spec.
pub(super) fn normalize_unix_input_spec(id: &str) -> RuntimeResult<UnixInputOpenSpec> {
    // reject invalid identifiers up front
    if id.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "id cannot be empty",
        ))
        .boxed());
    }
    if id.contains('\0') {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "id contains nul byte",
        ))
        .boxed());
    }

    // map tty aliases to one canonical terminal endpoint
    let id_lower = id.to_ascii_lowercase();
    if id == UNIX_INPUT_TTY_PATH
        || id == UNIX_INPUT_TTY_ID
        || id_lower == UNIX_INPUT_TTY_ALIAS
        || id_lower == UNIX_INPUT_STDIN_ALIAS
    {
        return Ok(UnixInputOpenSpec {
            path: UNIX_INPUT_TTY_PATH.to_string(),
            backend: UnixInputBackend::UnixTerminal,
            device_id: UNIX_INPUT_TTY_ID.to_string(),
        });
    }

    normalize_platform_input_spec(id, &id_lower)
}

/// Enumerate Unix input devices for the active platform.
pub(super) fn list_unix_devices(
    context: &BindingCallContext,
) -> RuntimeResult<Vec<InputDeviceDescriptor>> {
    list_platform_devices(context)
}

/// Open one Unix input descriptor with nonblocking flags.
pub(super) fn open_input_descriptor(path: &str) -> RuntimeResult<RawFd> {
    // encode path for host open call
    let path_cstring = CString::new(path).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "id contains nul byte",
        ))
        .boxed()
    })?;

    // prefer read-write mode and fall back to read-only when write access is denied
    let descriptor = {
        let read_write_descriptor = unsafe {
            libc::open(
                path_cstring.as_ptr(),
                libc::O_RDWR | libc::O_NONBLOCK | libc::O_CLOEXEC,
            )
        };
        if read_write_descriptor >= 0 {
            read_write_descriptor
        } else {
            let errno = core_platform::get_errno();
            if errno != libc::EACCES && errno != libc::EPERM {
                return Err(core_platform::io_error("open", Some(path)));
            }

            unsafe {
                libc::open(
                    path_cstring.as_ptr(),
                    libc::O_RDONLY | libc::O_NONBLOCK | libc::O_CLOEXEC,
                )
            }
        }
    };
    if descriptor < 0 {
        return Err(core_platform::io_error("open", Some(path)));
    }

    Ok(descriptor)
}

/// Read one terminal mode snapshot from one tty descriptor.
pub(super) fn read_terminal_mode(descriptor: RawFd) -> RuntimeResult<libc::termios> {
    // query current terminal attributes from the host descriptor
    let mut termios = MaybeUninit::<libc::termios>::uninit();
    let status = unsafe { libc::tcgetattr(descriptor, termios.as_mut_ptr()) };
    if status < 0 {
        return Err(core_platform::io_error("tcgetattr", None));
    }

    let termios = unsafe { termios.assume_init() };
    Ok(termios)
}

/// Apply one terminal mode snapshot to one tty descriptor.
fn apply_terminal_mode(descriptor: RawFd, mode: &libc::termios) -> RuntimeResult<()> {
    let status = unsafe { libc::tcsetattr(descriptor, libc::TCSANOW, mode) };
    if status < 0 {
        return Err(core_platform::io_error("tcsetattr", None));
    }

    Ok(())
}

/// Build one raw tty mode from one baseline cooked mode snapshot.
fn raw_terminal_mode(mut mode: libc::termios) -> libc::termios {
    // apply standard raw terminal flags and one-byte read semantics
    unsafe {
        libc::cfmakeraw(&mut mode);
    }
    mode.c_cc[libc::VMIN] = 1;
    mode.c_cc[libc::VTIME] = 0;
    mode
}

/// Wait until one descriptor becomes readable.
pub(super) fn wait_for_readable_descriptor(descriptor: RawFd) -> RuntimeResult<()> {
    // block in poll until the descriptor reports readable data
    let mut pollfd = libc::pollfd {
        fd: descriptor,
        events: libc::POLLIN,
        revents: 0,
    };
    loop {
        let status = unsafe { libc::poll(&mut pollfd as *mut libc::pollfd, 1, -1) };
        if status > 0 {
            return Ok(());
        }
        if status == 0 {
            continue;
        }

        let errno = core_platform::get_errno();
        if errno == libc::EINTR {
            continue;
        }

        return Err(core_platform::io_error("poll", None));
    }
}

/// Read one Unix input event from the selected backend.
pub(super) fn read_unix_event(
    context: &BindingCallContext,
    binding: &UnixInputBinding,
    handle: resource::InputDeviceHandle,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<InputEvent> {
    let mut event = match binding.backend {
        UnixInputBackend::Platform => {
            read_platform_event(context, binding, handle, nonblocking, operation)
        }
        UnixInputBackend::UnixTerminal => {
            let Some(descriptor) = binding.descriptor else {
                return Err(input_not_found(operation, handle));
            };
            read_terminal_event(
                context,
                descriptor,
                &binding.device_id,
                nonblocking,
                binding.read_mode,
            )
        }
    }?;

    // stamp the event with one per-handle sequence number
    let sequence = next_unix_event_sequence(context, handle, operation)?;
    event.sequence = sequence;

    Ok(event)
}

/// Set exclusive-grab mode for one Unix input backend.
pub(super) fn set_unix_grab(
    descriptor: Option<RawFd>,
    backend: UnixInputBackend,
    enable: bool,
) -> RuntimeResult<()> {
    match backend {
        UnixInputBackend::Platform => set_platform_grab(descriptor, enable),
        UnixInputBackend::UnixTerminal => Err(RuntimeError::from(PlatformError::not_supported(
            "destack.input.event.setExclusiveGrab",
        ))
        .boxed()),
    }
}

/// Set read mode for one Unix input binding.
pub(super) fn set_unix_read_mode(
    context: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    mode: InputReadMode,
    operation: &'static str,
) -> RuntimeResult<()> {
    let result = context
        .runtime()
        .resources
        .with_entry_mut(handle.0, |entry| {
            if entry.kind != ResourceKind::Input {
                return None;
            }
            if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
                return None;
            }

            // resolve mutable binding payload
            let binding = entry
                .payload
                .as_mut()
                .and_then(|payload| payload.downcast_mut::<UnixInputBinding>())?;

            // apply backend-specific mode transitions
            let update = match binding.backend {
                UnixInputBackend::Platform => set_platform_read_mode(mode),
                UnixInputBackend::UnixTerminal => {
                    let Some(descriptor) = binding.descriptor else {
                        return Some(Err(input_not_found(operation, handle)));
                    };

                    // capture one baseline terminal mode for later restoration
                    let original_mode = match binding.terminal_original_mode {
                        Some(mode) => mode,
                        None => match read_terminal_mode(descriptor) {
                            Ok(mode) => {
                                binding.terminal_original_mode = Some(mode);
                                mode
                            }
                            Err(error) => return Some(Err(error)),
                        },
                    };

                    // select raw or cooked terminal mode
                    if mode == InputReadMode::Raw {
                        let raw_mode = raw_terminal_mode(original_mode);
                        apply_terminal_mode(descriptor, &raw_mode)
                    } else {
                        apply_terminal_mode(descriptor, &original_mode)
                    }
                }
            };

            if update.is_ok() {
                binding.read_mode = mode;
            }
            Some(update)
        });

    match result.flatten() {
        Some(result) => result,
        None => Err(input_not_found(operation, handle)),
    }
}

/// Persist one text active flag and type for one Unix input handle.
pub(super) fn set_text_state(
    context: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    active: bool,
    input_type: InputTextInputType,
    operation: &'static str,
) -> RuntimeResult<()> {
    let updated = context
        .runtime()
        .resources
        .with_entry_mut(handle.0, |entry| {
            if entry.kind != ResourceKind::Input {
                return None;
            }
            if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
                return None;
            }

            let binding = entry
                .payload
                .as_mut()
                .and_then(|payload| payload.downcast_mut::<UnixInputBinding>())?;
            binding.text_active = active;
            binding.text_input_type = input_type;
            Some(())
        });

    match updated.flatten() {
        Some(()) => Ok(()),
        None => Err(input_not_found(operation, handle)),
    }
}

/// Persist one text-area hint for one Unix input handle.
pub(super) fn set_text_area(
    context: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    area: InputTextInputArea,
    operation: &'static str,
) -> RuntimeResult<()> {
    let updated = context
        .runtime()
        .resources
        .with_entry_mut(handle.0, |entry| {
            if entry.kind != ResourceKind::Input {
                return None;
            }
            if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
                return None;
            }

            let binding = entry
                .payload
                .as_mut()
                .and_then(|payload| payload.downcast_mut::<UnixInputBinding>())?;
            binding.text_area = area;
            Some(())
        });

    match updated.flatten() {
        Some(()) => Ok(()),
        None => Err(input_not_found(operation, handle)),
    }
}

/// Return whether text input is active for one Unix input handle.
pub(super) fn is_text_active(
    context: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<bool> {
    let active = context.runtime().resources.with_entry(handle.0, |entry| {
        if entry.kind != ResourceKind::Input {
            return None;
        }
        if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
            return None;
        }

        let binding = entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<UnixInputBinding>())?;
        Some(binding.text_active)
    });

    match active.flatten() {
        Some(active) => Ok(active),
        None => Err(input_not_found(operation, handle)),
    }
}

/// Return the text-area hint for one Unix input handle.
pub(super) fn text_area(
    context: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<InputTextInputArea> {
    let area = context.runtime().resources.with_entry(handle.0, |entry| {
        if entry.kind != ResourceKind::Input {
            return None;
        }
        if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
            return None;
        }

        let binding = entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<UnixInputBinding>())?;
        Some(binding.text_area)
    });

    match area.flatten() {
        Some(area) => Ok(area),
        None => Err(input_not_found(operation, handle)),
    }
}

/// Persist one gamepad player-index override for one Unix input handle.
pub(super) fn set_gamepad_player_index(
    context: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    player_index: u8,
    operation: &'static str,
) -> RuntimeResult<()> {
    if player_index == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "playerindex",
            "playerindex must be greater than zero",
        ))
        .boxed());
    }

    let updated = context
        .runtime()
        .resources
        .with_entry_mut(handle.0, |entry| {
            if entry.kind != ResourceKind::Input {
                return None;
            }
            if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
                return None;
            }

            let binding = entry
                .payload
                .as_mut()
                .and_then(|payload| payload.downcast_mut::<UnixInputBinding>())?;
            binding.gamepad_player_index_override = Some(player_index);
            Some(())
        });

    match updated.flatten() {
        Some(()) => Ok(()),
        None => Err(input_not_found(operation, handle)),
    }
}

/// Return the effective gamepad player index for one Unix input handle.
pub(super) fn gamepad_player_index(
    context: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<u8> {
    let player_index = context.runtime().resources.with_entry(handle.0, |entry| {
        if entry.kind != ResourceKind::Input {
            return None;
        }
        if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
            return None;
        }

        let binding = entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<UnixInputBinding>())?;
        Some(binding.gamepad_player_index_override.unwrap_or(1))
    });

    match player_index.flatten() {
        Some(player_index) => Ok(player_index),
        None => Err(input_not_found(operation, handle)),
    }
}

/// Persist one relative-mode flag for one unix input handle.
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(super) fn set_relative_mode_flag(
    context: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    enabled: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    let updated = context
        .runtime()
        .resources
        .with_entry_mut(handle.0, |entry| {
            if entry.kind != ResourceKind::Input {
                return None;
            }
            if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
                return None;
            }

            let binding = entry
                .payload
                .as_mut()
                .and_then(|payload| payload.downcast_mut::<UnixInputBinding>())?;
            binding.relative_mode_enabled = enabled;
            Some(())
        });

    match updated.flatten() {
        Some(()) => Ok(()),
        None => Err(input_not_found(operation, handle)),
    }
}

/// Persist one pointer snapshot baseline for one unix input handle.
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(super) fn set_pointer_snapshot(
    context: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    x: f64,
    y: f64,
    operation: &'static str,
) -> RuntimeResult<()> {
    let updated = context
        .runtime()
        .resources
        .with_entry_mut(handle.0, |entry| {
            if entry.kind != ResourceKind::Input {
                return None;
            }
            if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
                return None;
            }

            let binding = entry
                .payload
                .as_mut()
                .and_then(|payload| payload.downcast_mut::<UnixInputBinding>())?;
            binding.last_pointer_x = x;
            binding.last_pointer_y = y;
            Some(())
        });

    match updated.flatten() {
        Some(()) => Ok(()),
        None => Err(input_not_found(operation, handle)),
    }
}

/// Persist one effective sensor-stream configuration for one handle and one sensor lane.
pub(super) fn set_sensor_stream_config(
    context: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    sensor_kind: InputSensorKind,
    config: InputSensorEffectiveConfig,
    operation: &'static str,
) -> RuntimeResult<()> {
    let updated = context
        .runtime()
        .resources
        .with_entry_mut(handle.0, |entry| {
            if entry.kind != ResourceKind::Input {
                return None;
            }
            if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
                return None;
            }

            let binding = entry
                .payload
                .as_mut()
                .and_then(|payload| payload.downcast_mut::<UnixInputBinding>())?;
            if config.enabled {
                binding.sensor_enabled_kinds.insert(sensor_kind);
                binding.sensor_effective_configs.insert(sensor_kind, config);
            } else {
                binding.sensor_enabled_kinds.remove(&sensor_kind);
                binding.sensor_effective_configs.remove(&sensor_kind);
            }

            Some(())
        });

    match updated.flatten() {
        Some(()) => Ok(()),
        None => Err(input_not_found(operation, handle)),
    }
}

/// Return whether one sensor stream is currently enabled for one handle and one sensor lane.
pub(super) fn is_sensor_stream_enabled(
    context: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    sensor_kind: InputSensorKind,
    operation: &'static str,
) -> RuntimeResult<bool> {
    let enabled = context.runtime().resources.with_entry(handle.0, |entry| {
        if entry.kind != ResourceKind::Input {
            return None;
        }
        if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
            return None;
        }

        let binding = entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<UnixInputBinding>())?;
        Some(binding.sensor_enabled_kinds.contains(&sensor_kind))
    });

    match enabled.flatten() {
        Some(enabled) => Ok(enabled),
        None => Err(input_not_found(operation, handle)),
    }
}

/// Persist the active uploaded rumble effect id for one Linux handle.
#[cfg(target_os = "linux")]
pub(super) fn set_linux_active_rumble_effect_id(
    context: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    effect_id: Option<i16>,
    operation: &'static str,
) -> RuntimeResult<()> {
    let updated = context
        .runtime()
        .resources
        .with_entry_mut(handle.0, |entry| {
            if entry.kind != ResourceKind::Input {
                return None;
            }
            if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
                return None;
            }

            let binding = entry
                .payload
                .as_mut()
                .and_then(|payload| payload.downcast_mut::<UnixInputBinding>())?;
            binding.linux_active_rumble_effect_id = effect_id;
            Some(())
        });

    match updated.flatten() {
        Some(()) => Ok(()),
        None => Err(input_not_found(operation, handle)),
    }
}

/// Return the active uploaded rumble effect id for one Linux handle, when present.
#[cfg(target_os = "linux")]
pub(super) fn linux_active_rumble_effect_id(
    context: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<Option<i16>> {
    let effect_id = context.runtime().resources.with_entry(handle.0, |entry| {
        if entry.kind != ResourceKind::Input {
            return None;
        }
        if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
            return None;
        }

        let binding = entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<UnixInputBinding>())?;
        Some(binding.linux_active_rumble_effect_id)
    });

    match effect_id.flatten() {
        Some(effect_id) => Ok(effect_id),
        None => Err(input_not_found(operation, handle)),
    }
}

/// Return one default read mode for platform-backed inputs.
pub(super) fn platform_default_read_mode() -> InputReadMode {
    InputReadMode::Raw
}

/// Return one initialized macOS platform state for one backend.
#[cfg(target_os = "macos")]
pub(super) fn initial_macos_state(
    backend: UnixInputBackend,
) -> Option<input_macos::MacosInputState> {
    if backend == UnixInputBackend::Platform {
        return Some(input_macos::MacosInputState::new());
    }

    None
}

/// Release one macOS platform subscription for one handle before close.
#[cfg(target_os = "macos")]
pub(super) fn release_macos_subscription(
    context: &BindingCallContext,
    handle: resource::InputDeviceHandle,
) {
    input_macos::release_macos_session_subscription(context, handle);
}

/// Normalize one platform-specific identifier into one input open spec on Linux.
#[cfg(target_os = "linux")]
fn normalize_platform_input_spec(id: &str, _id_lower: &str) -> RuntimeResult<UnixInputOpenSpec> {
    let path = input_linux::normalize_input_path(id)?;
    let device_id = input_linux::linux_runtime_device_id_for_path(&path);

    Ok(UnixInputOpenSpec {
        device_id,
        path,
        backend: UnixInputBackend::Platform,
    })
}

/// Normalize one platform-specific identifier into one input open spec on macOS.
#[cfg(target_os = "macos")]
fn normalize_platform_input_spec(id: &str, id_lower: &str) -> RuntimeResult<UnixInputOpenSpec> {
    if input_macos::is_macos_session_identifier(id, id_lower) {
        return Ok(UnixInputOpenSpec {
            path: String::new(),
            backend: UnixInputBackend::Platform,
            device_id: input_macos::MACOS_INPUT_SESSION_ID.to_string(),
        });
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "id",
        "id must be one supported input identifier for this host",
    ))
    .boxed())
}

/// Normalize one platform-specific identifier into one input open spec on other Unix hosts.
#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
fn normalize_platform_input_spec(_id: &str, _id_lower: &str) -> RuntimeResult<UnixInputOpenSpec> {
    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "id",
        "id must be one supported input identifier for this host",
    ))
    .boxed())
}

/// Enumerate platform-specific devices on Linux with terminal fallback.
#[cfg(target_os = "linux")]
fn list_platform_devices(
    context: &BindingCallContext,
) -> RuntimeResult<Vec<InputDeviceDescriptor>> {
    let devices = input_linux::list_linux_devices(context)?;
    if !devices.is_empty() {
        return Ok(devices);
    }

    let tty_device = list_terminal_device(context)?;
    Ok(tty_device.into_iter().collect())
}

/// Enumerate platform-specific devices on macOS and include terminal fallback.
#[cfg(target_os = "macos")]
fn list_platform_devices(
    context: &BindingCallContext,
) -> RuntimeResult<Vec<InputDeviceDescriptor>> {
    let mut devices = Vec::new();
    devices.push(InputDeviceDescriptor {
        id: context.store_string(input_macos::MACOS_INPUT_SESSION_ID),
        instance_id: context.store_string(input_macos::MACOS_INPUT_SESSION_ID),
        hardware_id: context.store_string(input_macos::MACOS_INPUT_SESSION_ID),
        name: context.store_string(input_macos::MACOS_INPUT_SESSION_NAME),
        transport: context.store_string("session"),
        kind: InputDeviceKind::Raw,
        vendor_id: 0,
        product_id: 0,
        key_count: input_macos::MACOS_SESSION_KEY_COUNT,
        button_count: input_macos::MACOS_SESSION_BUTTON_COUNT,
        axis_count: input_macos::MACOS_SESSION_AXIS_COUNT,
        connected: true,
        supports_exclusive_grab: false,
        supports_raw: true,
        supports_text: false,
        supports_rumble: false,
        supports_battery: false,
        supports_light: false,
        supports_raw_hid: false,
        is_virtual: false,
        is_system: true,
    });

    if let Some(tty_device) = list_terminal_device(context)? {
        devices.push(tty_device);
    }

    Ok(devices)
}

/// Enumerate platform-specific devices on other Unix hosts with terminal-only discovery.
#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
fn list_platform_devices(
    context: &BindingCallContext,
) -> RuntimeResult<Vec<InputDeviceDescriptor>> {
    let tty_device = list_terminal_device(context)?;
    Ok(tty_device.into_iter().collect())
}

/// Read one platform-specific event from one opened platform backend on Linux.
#[cfg(target_os = "linux")]
fn read_platform_event(
    context: &BindingCallContext,
    _binding: &UnixInputBinding,
    handle: resource::InputDeviceHandle,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<InputEvent> {
    let result = context
        .runtime()
        .resources
        .with_entry_mut(handle.0, |entry| {
            if entry.kind != ResourceKind::Input {
                return None;
            }

            if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
                return None;
            }

            let binding = entry
                .payload
                .as_mut()
                .and_then(|payload| payload.downcast_mut::<UnixInputBinding>())?;
            if binding.backend != UnixInputBackend::Platform {
                return None;
            }

            let Some(descriptor) = binding.descriptor else {
                return Some(Err(input_not_found(operation, handle)));
            };

            let event = input_linux::read_linux_event(
                context,
                descriptor,
                nonblocking,
                &binding.device_id,
                binding.device_kind,
                binding.linux_modifiers,
                binding.linux_pointer_buttons,
            );
            match event {
                Ok((event, modifiers, pointer_buttons)) => {
                    binding.linux_modifiers = modifiers;
                    binding.linux_pointer_buttons = pointer_buttons;
                    Some(Ok(event))
                }
                Err(error) => Some(Err(error)),
            }
        });

    match result {
        Some(Some(Ok(event))) => Ok(event),
        Some(Some(Err(error))) => Err(error),
        Some(None) | None => Err(input_not_found(operation, handle)),
    }
}

/// Read one platform-specific event from one opened platform backend on macOS.
#[cfg(target_os = "macos")]
fn read_platform_event(
    context: &BindingCallContext,
    binding: &UnixInputBinding,
    handle: resource::InputDeviceHandle,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<InputEvent> {
    let _ = (binding, operation);
    input_macos::read_macos_session_event(context, handle, nonblocking, binding.read_mode)
}

/// Read one platform-specific event from one opened platform backend on other Unix hosts.
#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
fn read_platform_event(
    _context: &BindingCallContext,
    _binding: &UnixInputBinding,
    _handle: resource::InputDeviceHandle,
    _nonblocking: bool,
    _operation: &'static str,
) -> RuntimeResult<InputEvent> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.input.event.read")).boxed())
}

/// Set exclusive-grab mode for one platform backend on Linux.
#[cfg(target_os = "linux")]
fn set_platform_grab(descriptor: Option<RawFd>, enable: bool) -> RuntimeResult<()> {
    let Some(descriptor) = descriptor else {
        return Err(RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoNotFound),
            None,
            None,
            Some("destack.input.event.setExclusiveGrab".to_string()),
            None,
            "input device handle is missing one descriptor".to_string(),
        ))
        .boxed());
    };

    input_linux::set_linux_grab(descriptor, enable)
}

/// Set exclusive-grab mode for one platform backend on non-Linux Unix hosts.
#[cfg(all(unix, not(target_os = "linux")))]
fn set_platform_grab(_descriptor: Option<RawFd>, _enable: bool) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.setExclusiveGrab",
    ))
    .boxed())
}

/// Set read mode for one platform backend on Linux and macOS.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn set_platform_read_mode(mode: InputReadMode) -> RuntimeResult<()> {
    if mode == InputReadMode::Cooked {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.input.event.setReadMode",
        ))
        .boxed());
    }

    Ok(())
}

/// Set read mode for one platform backend on other Unix hosts.
#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
fn set_platform_read_mode(_mode: InputReadMode) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.event.setReadMode",
    ))
    .boxed())
}

/// Build terminal input metadata when `/dev/tty` is available.
fn list_terminal_device(
    context: &BindingCallContext,
) -> RuntimeResult<Option<InputDeviceDescriptor>> {
    // encode terminal path for one open probe
    let path = CString::new(UNIX_INPUT_TTY_PATH).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "terminal path contains nul byte",
        ))
        .boxed()
    })?;

    // probe tty availability with one read-only descriptor open
    let descriptor = unsafe { libc::open(path.as_ptr(), libc::O_RDONLY | libc::O_CLOEXEC) };
    if descriptor < 0 {
        let errno = core_platform::get_errno();
        if errno == libc::ENOENT
            || errno == libc::ENOTTY
            || errno == libc::ENXIO
            || errno == libc::EACCES
            || errno == libc::EPERM
        {
            return Ok(None);
        }

        return Err(core_platform::io_error("open", Some(UNIX_INPUT_TTY_PATH)));
    }

    unsafe {
        libc::close(descriptor);
    }

    Ok(Some(InputDeviceDescriptor {
        id: context.store_string(UNIX_INPUT_TTY_ID),
        instance_id: context.store_string(UNIX_INPUT_TTY_ID),
        hardware_id: context.store_string(UNIX_INPUT_TTY_ID),
        name: context.store_string(UNIX_INPUT_TTY_NAME),
        transport: context.store_string("tty"),
        kind: InputDeviceKind::Keyboard,
        vendor_id: 0,
        product_id: 0,
        key_count: 0,
        button_count: 0,
        axis_count: 0,
        connected: true,
        supports_exclusive_grab: false,
        supports_raw: true,
        supports_text: true,
        supports_rumble: false,
        supports_battery: false,
        supports_light: false,
        supports_raw_hid: false,
        is_virtual: false,
        is_system: true,
    }))
}

/// Read one byte-oriented event from terminal input.
fn read_terminal_event(
    context: &BindingCallContext,
    descriptor: RawFd,
    device_id: &str,
    nonblocking: bool,
    read_mode: InputReadMode,
) -> RuntimeResult<InputEvent> {
    // probe readiness in nonblocking mode
    if nonblocking {
        let mut pollfd = libc::pollfd {
            fd: descriptor,
            events: libc::POLLIN,
            revents: 0,
        };
        let poll_status = unsafe { libc::poll(&mut pollfd as *mut libc::pollfd, 1, 0) };
        if poll_status < 0 {
            return Err(core_platform::io_error("poll", None));
        }
        if poll_status == 0 {
            return Err(RuntimeError::from(PlatformError::io_with(
                Some(PlatformErrorCode::IoWouldBlock),
                None,
                Some(libc::EWOULDBLOCK),
                Some("poll".to_string()),
                None,
                "input queue is empty",
            ))
            .boxed());
        }
    }

    // read one byte with interrupt and would-block handling
    let mut byte = 0u8;
    loop {
        let read_status = unsafe {
            libc::read(
                descriptor,
                (&mut byte as *mut u8).cast::<libc::c_void>(),
                std::mem::size_of::<u8>(),
            )
        };
        if read_status == 0 {
            return Err(RuntimeError::from(PlatformError::io_with(
                Some(PlatformErrorCode::IoNotFound),
                None,
                None,
                Some("read".to_string()),
                None,
                "input device reached end of stream",
            ))
            .boxed());
        }
        if read_status < 0 {
            let errno = core_platform::get_errno();
            if errno == libc::EINTR {
                continue;
            }
            if errno == libc::EAGAIN || errno == libc::EWOULDBLOCK {
                if nonblocking {
                    return Err(RuntimeError::from(PlatformError::io_with(
                        Some(PlatformErrorCode::IoWouldBlock),
                        None,
                        Some(errno),
                        Some("read".to_string()),
                        None,
                        "input queue is empty",
                    ))
                    .boxed());
                }

                wait_for_readable_descriptor(descriptor)?;
                continue;
            }

            return Err(core_platform::io_error("read", None));
        }
        break;
    }

    // map byte stream into text or key semantics
    let is_text_byte = byte.is_ascii_graphic() || matches!(byte, b' ' | b'\n' | b'\r' | b'\t');
    let kind = if read_mode == InputReadMode::Cooked && is_text_byte {
        InputEventKind::Text
    } else {
        InputEventKind::Key
    };
    let value = if kind == InputEventKind::Text {
        byte as i64
    } else {
        1
    };

    let text = if kind == InputEventKind::Text {
        let bytes = [byte];
        let value = String::from_utf8_lossy(&bytes);
        Some(value.to_string())
    } else {
        None
    };

    let mut payload = empty_unix_event_payload(context);
    if kind == InputEventKind::Text {
        payload.text = InputTextEventPayload {
            text: context.store_string(text.as_deref().unwrap_or(UNIX_INPUT_EMPTY_TEXT)),
        };
    } else {
        payload.key = InputKeyEventPayload {
            action: InputEventAction::Press,
            backend_code: byte as u32,
            backend_scan_code: byte as u32,
            backend_value: value,
            modifiers: 0,
            repeat: false,
        };
    }

    Ok(build_unix_input_event(
        context,
        kind,
        monotonic_timestamp_ns(),
        0,
        device_id,
        payload,
    ))
}

/// Return one monotonic host timestamp in nanoseconds.
pub(super) fn monotonic_timestamp_ns() -> u64 {
    // query one monotonic timespec from the host
    let mut timestamp = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let status = unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut timestamp) };
    if status < 0 || timestamp.tv_sec < 0 || timestamp.tv_nsec < 0 {
        return 0;
    }

    // convert one timespec payload to nanoseconds
    (timestamp.tv_sec as u64)
        .saturating_mul(1_000_000_000)
        .saturating_add(timestamp.tv_nsec as u64)
}

/// Allocate the next sequence number for one Unix input stream.
pub(super) fn next_unix_event_sequence(
    context: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<u64> {
    let sequence = context
        .runtime()
        .resources
        .with_entry_mut(handle.0, |entry| {
            if entry.kind != ResourceKind::Input {
                return None;
            }
            if entry.label.as_deref() != Some(INPUT_RESOURCE_LABEL) {
                return None;
            }

            let binding = entry
                .payload
                .as_mut()
                .and_then(|payload| payload.downcast_mut::<UnixInputBinding>())?;
            let next = binding.next_sequence;
            binding.next_sequence = binding.next_sequence.saturating_add(1);
            Some(next)
        });

    match sequence.flatten() {
        Some(sequence) => Ok(sequence),
        None => Err(input_not_found(operation, handle)),
    }
}
