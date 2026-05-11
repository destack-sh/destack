use std::mem::MaybeUninit;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::{Condvar, Mutex};

use windows_sys::Win32::Foundation::{ERROR_DEVICE_NOT_CONNECTED, ERROR_SUCCESS};
use windows_sys::Win32::UI::Input::XboxController::{
    BATTERY_DEVTYPE_GAMEPAD, BATTERY_LEVEL_EMPTY, BATTERY_LEVEL_FULL, BATTERY_LEVEL_LOW,
    BATTERY_LEVEL_MEDIUM, BATTERY_TYPE_ALKALINE, BATTERY_TYPE_DISCONNECTED, BATTERY_TYPE_NIMH,
    BATTERY_TYPE_WIRED, XINPUT_CAPABILITIES, XINPUT_CAPS_FFB_SUPPORTED, XINPUT_CAPS_WIRELESS,
    XINPUT_FLAG_GAMEPAD, XINPUT_GAMEPAD_A, XINPUT_GAMEPAD_B, XINPUT_GAMEPAD_BACK,
    XINPUT_GAMEPAD_DPAD_DOWN, XINPUT_GAMEPAD_DPAD_LEFT, XINPUT_GAMEPAD_DPAD_RIGHT,
    XINPUT_GAMEPAD_DPAD_UP, XINPUT_GAMEPAD_LEFT_SHOULDER, XINPUT_GAMEPAD_LEFT_THUMB,
    XINPUT_GAMEPAD_RIGHT_SHOULDER, XINPUT_GAMEPAD_RIGHT_THUMB, XINPUT_GAMEPAD_START,
    XINPUT_GAMEPAD_TRIGGER_THRESHOLD, XINPUT_GAMEPAD_X, XINPUT_GAMEPAD_Y, XINPUT_STATE,
    XINPUT_VIBRATION, XInputGetBatteryInformation, XInputGetCapabilities, XInputGetState,
    XInputSetState,
};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::input::{
    InputAxisMetadata, InputButtonMetadata, InputCapabilityMetadataFidelity,
    InputCapabilityMetadataOrigin, InputDeviceCapabilities, InputDeviceCapabilityKind,
    InputDeviceDescriptor, InputDeviceKind, InputGamepadBatteryState, InputGamepadBatteryStatus,
    InputGamepadButtonState, InputGamepadConnectionType, InputGamepadMappingType,
    InputGamepadState, InputHapticEffectParameters, InputHapticEffectType, InputHapticsResult,
};
use crate::platform::{PlatformError, core as core_platform};
use crate::runtime::service::Service;
use crate::runtime::service::executor::periodic::{PeriodicTaskHandle, open_periodic_task};
use crate::runtime::{BindingCallContext, ExecutionMode, ExecutionPolicy};

/// Prefix for stable xinput device identifiers.
pub(super) const XINPUT_DEVICE_ID_PREFIX: &str = "xinput:";
/// Number of xinput user slots.
const XINPUT_USER_SLOT_COUNT: u8 = 4;
/// Standardized gamepad axis count in this runtime contract.
const XINPUT_STANDARD_AXIS_COUNT: u16 = 4;
/// Standardized gamepad button count in this runtime contract.
const XINPUT_STANDARD_BUTTON_COUNT: u16 = 17;
/// Poll interval for the shared XInput packet watcher.
const XINPUT_POLL_INTERVAL: Duration = Duration::from_millis(4);

/// One sampled XInput slot state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct XInputSlotState {
    /// Whether this slot is currently connected.
    connected: bool,
    /// Last observed packet number for this slot.
    packet_number: u32,
}

/// One synchronized XInput packet state snapshot.
#[derive(Debug)]
struct WindowsXInputSharedState {
    /// Last observed state for each fixed user slot.
    slots: Mutex<[XInputSlotState; XINPUT_USER_SLOT_COUNT as usize]>,
    /// Wake handle for blocking XInput event reads.
    wake: Condvar,
}

/// One process-global XInput packet polling service.
#[derive(Debug)]
pub(crate) struct WindowsXInputService {
    /// Shared packet state for blocking reads.
    state: Arc<WindowsXInputSharedState>,
    /// Registered periodic polling task.
    _task: PeriodicTaskHandle,
}

impl WindowsXInputSharedState {
    /// Build one shared XInput state snapshot.
    fn new() -> Self {
        Self {
            slots: Mutex::new(sample_xinput_slots()),
            wake: Condvar::new(),
        }
    }
}

impl WindowsXInputService {
    /// Build one process-global XInput packet polling service.
    fn new() -> RuntimeResult<Self> {
        let state = Arc::new(WindowsXInputSharedState::new());
        let poll_state = Arc::clone(&state);

        // keep one shared packet snapshot current for all XInput readers
        let task = open_periodic_task(
            "destack-input-windows-xinput",
            Self::POLICY,
            XINPUT_POLL_INTERVAL,
            move || {
                refresh_xinput_slot_state(&poll_state);

                Ok(())
            },
        )?;

        Ok(Self { state, _task: task })
    }

    /// Wait for one packet-number change on one user slot.
    pub(crate) fn wait_for_packet_change(
        &self,
        user_index: u8,
        current_packet_number: u32,
        nonblocking: bool,
        operation: &'static str,
    ) -> RuntimeResult<u32> {
        let slot_index = usize::from(user_index);
        let mut slots = self.state.slots.lock();

        loop {
            let slot = slots[slot_index];

            // surface host disconnects loudly instead of silently spinning
            if !slot.connected {
                return Err(xinput_error(
                    operation,
                    "XInputGetState",
                    ERROR_DEVICE_NOT_CONNECTED,
                    "controller disconnected",
                ));
            }

            // return the next observed packet edge
            if slot.packet_number != current_packet_number {
                return Ok(slot.packet_number);
            }

            // nonblocking reads stop when no packet edge is pending
            if nonblocking {
                return Err(core_platform::io_would_block(
                    operation,
                    "input queue is empty",
                ));
            }

            // otherwise wait for the shared poller to publish the next change
            self.state.wake.wait(&mut slots);
        }
    }
}

impl Service for WindowsXInputService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Polling);
}

/// Return one process-global XInput polling service.
pub(crate) fn windows_xinput_service(
    operation: &'static str,
) -> RuntimeResult<Arc<WindowsXInputService>> {
    WindowsXInputService::global(WindowsXInputService::new).map_err(|error| {
        core_platform::io_operation_error(
            operation,
            None,
            format!("failed to initialize windows xinput service: {error}"),
        )
    })
}

/// Read one monotonic timestamp from the shared runtime clock domain.
fn now_timestamp_ns() -> u64 {
    core_platform::monotonic_now_ns()
}

/// Sample one XInput slot without promoting disconnects into runtime errors.
fn sample_xinput_slot_state(user_index: u8) -> XInputSlotState {
    let mut state = MaybeUninit::<XINPUT_STATE>::zeroed();
    let status = unsafe { XInputGetState(u32::from(user_index), state.as_mut_ptr()) };

    // disconnected slots are expected in the fixed XInput namespace
    if status == ERROR_DEVICE_NOT_CONNECTED {
        return XInputSlotState {
            connected: false,
            packet_number: 0,
        };
    }

    // invalid slot indices are impossible here, so any other failure is treated as absent
    if status != ERROR_SUCCESS {
        return XInputSlotState {
            connected: false,
            packet_number: 0,
        };
    }

    let state = unsafe { state.assume_init() };

    XInputSlotState {
        connected: true,
        packet_number: state.dwPacketNumber,
    }
}

/// Sample all fixed XInput user slots.
fn sample_xinput_slots() -> [XInputSlotState; XINPUT_USER_SLOT_COUNT as usize] {
    let mut slots = [XInputSlotState::default(); XINPUT_USER_SLOT_COUNT as usize];

    for user_index in 0..XINPUT_USER_SLOT_COUNT {
        slots[usize::from(user_index)] = sample_xinput_slot_state(user_index);
    }

    slots
}

/// Refresh one shared XInput packet snapshot and wake blocked readers on change.
fn refresh_xinput_slot_state(state: &Arc<WindowsXInputSharedState>) {
    let next_slots = sample_xinput_slots();
    let mut slots = state.slots.lock();

    // avoid waking readers when the shared snapshot has not changed
    if *slots == next_slots {
        return;
    }

    *slots = next_slots;
    state.wake.notify_all();
}

/// Return one stable runtime xinput device identifier for one user index.
pub(super) fn xinput_device_id(user_index: u8) -> String {
    format!("{XINPUT_DEVICE_ID_PREFIX}{user_index}")
}

/// Parse one xinput user index from one normalized device identifier.
pub(super) fn parse_xinput_device_id(id: &str) -> Option<u8> {
    let suffix = id.strip_prefix(XINPUT_DEVICE_ID_PREFIX)?;
    if suffix.is_empty() {
        return None;
    }

    let user_index = suffix.parse::<u8>().ok()?;
    if user_index >= XINPUT_USER_SLOT_COUNT {
        return None;
    }

    Some(user_index)
}

/// Build one runtime io error from one xinput status code.
fn xinput_error(
    operation: &'static str,
    syscall: &'static str,
    status: u32,
    message: &str,
) -> Box<RuntimeError> {
    let platform_code = if status == ERROR_DEVICE_NOT_CONNECTED {
        Some(PlatformErrorCode::IoNotFound)
    } else {
        None
    };

    RuntimeError::from(PlatformError::io_with(
        platform_code,
        None,
        Some(status as i32),
        Some(operation.to_string()),
        None,
        format!("{syscall} failed: {message} ({status})"),
    ))
    .boxed()
}

/// Query one xinput state for one user index.
fn query_xinput_state(user_index: u8, operation: &'static str) -> RuntimeResult<XINPUT_STATE> {
    let mut state = MaybeUninit::<XINPUT_STATE>::zeroed();

    let status = unsafe { XInputGetState(u32::from(user_index), state.as_mut_ptr()) };
    if status != ERROR_SUCCESS {
        return Err(xinput_error(
            operation,
            "XInputGetState",
            status,
            "failed to query controller state",
        ));
    }

    Ok(unsafe { state.assume_init() })
}

/// Query one xinput capabilities payload when available.
fn query_xinput_capabilities(user_index: u8) -> Option<XINPUT_CAPABILITIES> {
    let mut capabilities = MaybeUninit::<XINPUT_CAPABILITIES>::zeroed();
    let status = unsafe {
        XInputGetCapabilities(
            u32::from(user_index),
            XINPUT_FLAG_GAMEPAD,
            capabilities.as_mut_ptr(),
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }

    Some(unsafe { capabilities.assume_init() })
}

/// Query one xinput battery payload when available.
fn query_xinput_battery(
    user_index: u8,
    operation: &'static str,
) -> RuntimeResult<InputGamepadBatteryStatus> {
    let mut battery = MaybeUninit::zeroed();
    let status = unsafe {
        XInputGetBatteryInformation(
            u32::from(user_index),
            BATTERY_DEVTYPE_GAMEPAD,
            battery.as_mut_ptr(),
        )
    };
    if status == ERROR_DEVICE_NOT_CONNECTED {
        return Ok(InputGamepadBatteryStatus {
            state: InputGamepadBatteryState::NotPresent,
            level: 0.0,
        });
    }
    if status != ERROR_SUCCESS {
        return Err(xinput_error(
            operation,
            "XInputGetBatteryInformation",
            status,
            "failed to query controller battery information",
        ));
    }

    let battery = unsafe { battery.assume_init() };
    if battery.BatteryType == BATTERY_TYPE_DISCONNECTED {
        return Ok(InputGamepadBatteryStatus {
            state: InputGamepadBatteryState::NotPresent,
            level: 0.0,
        });
    }

    if battery.BatteryType == BATTERY_TYPE_WIRED {
        return Ok(InputGamepadBatteryStatus {
            state: InputGamepadBatteryState::Full,
            level: 1.0,
        });
    }

    let level = if battery.BatteryLevel == BATTERY_LEVEL_EMPTY {
        0.0
    } else if battery.BatteryLevel == BATTERY_LEVEL_LOW {
        1.0 / 3.0
    } else if battery.BatteryLevel == BATTERY_LEVEL_MEDIUM {
        2.0 / 3.0
    } else if battery.BatteryLevel == BATTERY_LEVEL_FULL {
        1.0
    } else {
        0.0
    };

    let state = if battery.BatteryType == BATTERY_TYPE_ALKALINE
        || battery.BatteryType == BATTERY_TYPE_NIMH
    {
        InputGamepadBatteryState::Discharging
    } else {
        InputGamepadBatteryState::Unknown
    };

    Ok(InputGamepadBatteryStatus { state, level })
}

/// Normalize one signed stick sample into a [-1, 1] axis value.
fn normalize_stick_axis(value: i16) -> f64 {
    if value >= 0 {
        f64::from(value) / f64::from(i16::MAX)
    } else {
        f64::from(value) / f64::from(i16::MIN.unsigned_abs())
    }
}

/// Build one gamepad button snapshot.
fn gamepad_button_state(pressed: bool, value: f64) -> InputGamepadButtonState {
    InputGamepadButtonState {
        pressed,
        touched: pressed,
        value,
    }
}

/// Return one full gamepad-button snapshot array in standard order.
fn xinput_standard_button_states(state: &XINPUT_STATE) -> Vec<InputGamepadButtonState> {
    let buttons = state.Gamepad.wButtons;
    let left_trigger_value = f64::from(state.Gamepad.bLeftTrigger) / 255.0;
    let right_trigger_value = f64::from(state.Gamepad.bRightTrigger) / 255.0;

    vec![
        gamepad_button_state(
            (buttons & XINPUT_GAMEPAD_A) != 0,
            if (buttons & XINPUT_GAMEPAD_A) != 0 {
                1.0
            } else {
                0.0
            },
        ),
        gamepad_button_state(
            (buttons & XINPUT_GAMEPAD_B) != 0,
            if (buttons & XINPUT_GAMEPAD_B) != 0 {
                1.0
            } else {
                0.0
            },
        ),
        gamepad_button_state(
            (buttons & XINPUT_GAMEPAD_X) != 0,
            if (buttons & XINPUT_GAMEPAD_X) != 0 {
                1.0
            } else {
                0.0
            },
        ),
        gamepad_button_state(
            (buttons & XINPUT_GAMEPAD_Y) != 0,
            if (buttons & XINPUT_GAMEPAD_Y) != 0 {
                1.0
            } else {
                0.0
            },
        ),
        gamepad_button_state(
            (buttons & XINPUT_GAMEPAD_LEFT_SHOULDER) != 0,
            if (buttons & XINPUT_GAMEPAD_LEFT_SHOULDER) != 0 {
                1.0
            } else {
                0.0
            },
        ),
        gamepad_button_state(
            (buttons & XINPUT_GAMEPAD_RIGHT_SHOULDER) != 0,
            if (buttons & XINPUT_GAMEPAD_RIGHT_SHOULDER) != 0 {
                1.0
            } else {
                0.0
            },
        ),
        gamepad_button_state(
            left_trigger_value >= (f64::from(XINPUT_GAMEPAD_TRIGGER_THRESHOLD) / 255.0),
            left_trigger_value,
        ),
        gamepad_button_state(
            right_trigger_value >= (f64::from(XINPUT_GAMEPAD_TRIGGER_THRESHOLD) / 255.0),
            right_trigger_value,
        ),
        gamepad_button_state(
            (buttons & XINPUT_GAMEPAD_BACK) != 0,
            if (buttons & XINPUT_GAMEPAD_BACK) != 0 {
                1.0
            } else {
                0.0
            },
        ),
        gamepad_button_state(
            (buttons & XINPUT_GAMEPAD_START) != 0,
            if (buttons & XINPUT_GAMEPAD_START) != 0 {
                1.0
            } else {
                0.0
            },
        ),
        gamepad_button_state(
            (buttons & XINPUT_GAMEPAD_LEFT_THUMB) != 0,
            if (buttons & XINPUT_GAMEPAD_LEFT_THUMB) != 0 {
                1.0
            } else {
                0.0
            },
        ),
        gamepad_button_state(
            (buttons & XINPUT_GAMEPAD_RIGHT_THUMB) != 0,
            if (buttons & XINPUT_GAMEPAD_RIGHT_THUMB) != 0 {
                1.0
            } else {
                0.0
            },
        ),
        gamepad_button_state(
            (buttons & XINPUT_GAMEPAD_DPAD_UP) != 0,
            if (buttons & XINPUT_GAMEPAD_DPAD_UP) != 0 {
                1.0
            } else {
                0.0
            },
        ),
        gamepad_button_state(
            (buttons & XINPUT_GAMEPAD_DPAD_DOWN) != 0,
            if (buttons & XINPUT_GAMEPAD_DPAD_DOWN) != 0 {
                1.0
            } else {
                0.0
            },
        ),
        gamepad_button_state(
            (buttons & XINPUT_GAMEPAD_DPAD_LEFT) != 0,
            if (buttons & XINPUT_GAMEPAD_DPAD_LEFT) != 0 {
                1.0
            } else {
                0.0
            },
        ),
        gamepad_button_state(
            (buttons & XINPUT_GAMEPAD_DPAD_RIGHT) != 0,
            if (buttons & XINPUT_GAMEPAD_DPAD_RIGHT) != 0 {
                1.0
            } else {
                0.0
            },
        ),
        gamepad_button_state(false, 0.0),
    ]
}

/// Return one standard xinput axis array in runtime order.
fn standard_gamepad_axes(state: &XINPUT_STATE) -> Vec<f64> {
    vec![
        normalize_stick_axis(state.Gamepad.sThumbLX),
        normalize_stick_axis(state.Gamepad.sThumbLY),
        normalize_stick_axis(state.Gamepad.sThumbRX),
        normalize_stick_axis(state.Gamepad.sThumbRY),
    ]
}

/// Return one xinput connection-type classification from capability flags.
fn connection_type_from_capabilities(
    capabilities: Option<XINPUT_CAPABILITIES>,
) -> InputGamepadConnectionType {
    let Some(capabilities) = capabilities else {
        return InputGamepadConnectionType::Unknown;
    };

    if (capabilities.Flags & XINPUT_CAPS_WIRELESS) != 0 {
        return InputGamepadConnectionType::Wireless;
    }

    InputGamepadConnectionType::Wired
}

/// Return whether one xinput device supports force feedback.
fn supports_rumble(capabilities: Option<XINPUT_CAPABILITIES>) -> bool {
    let Some(capabilities) = capabilities else {
        return false;
    };

    (capabilities.Flags & XINPUT_CAPS_FFB_SUPPORTED) != 0
}

/// Enumerate connected xinput gamepads.
pub(super) fn list_xinput_devices(binding: &BindingCallContext) -> Vec<InputDeviceDescriptor> {
    let mut devices = Vec::new();

    for user_index in 0..XINPUT_USER_SLOT_COUNT {
        if query_xinput_state(user_index, "destack.input.device.list").is_err() {
            continue;
        }

        let capabilities = query_xinput_capabilities(user_index);
        let supports_rumble = supports_rumble(capabilities);
        let connection_type = connection_type_from_capabilities(capabilities);
        let name = if connection_type == InputGamepadConnectionType::Wireless {
            format!("xinput wireless controller {user_index}")
        } else {
            format!("xinput controller {user_index}")
        };
        let id = xinput_device_id(user_index);

        devices.push(InputDeviceDescriptor {
            id: binding.store_string(&id),
            instance_id: binding.store_string(&id),
            hardware_id: binding.store_string(&id),
            name: binding.store_string(&name),
            transport: binding.store_string("xinput"),
            kind: InputDeviceKind::Gamepad,
            vendor_id: 0,
            product_id: 0,
            key_count: 0,
            button_count: XINPUT_STANDARD_BUTTON_COUNT,
            axis_count: XINPUT_STANDARD_AXIS_COUNT,
            connected: true,
            supports_exclusive_grab: false,
            supports_raw: true,
            supports_text: false,
            supports_rumble,
            supports_battery: true,
            supports_light: false,
            supports_raw_hid: false,
            is_virtual: false,
            is_system: false,
        });
    }

    devices
}

/// Return one event packet number for one xinput user index.
pub(super) fn xinput_packet_number(user_index: u8, operation: &'static str) -> RuntimeResult<u32> {
    let state = query_xinput_state(user_index, operation)?;
    Ok(state.dwPacketNumber)
}

/// Build one capability payload for one xinput device.
pub(super) fn capabilities_for_xinput_device(
    binding: &BindingCallContext,
    user_index: u8,
    operation: &'static str,
) -> RuntimeResult<InputDeviceCapabilities> {
    let _ = query_xinput_state(user_index, operation)?;
    let supports_rumble = supports_rumble(query_xinput_capabilities(user_index));

    let mut kinds = vec![InputDeviceCapabilityKind::Gamepad];
    if supports_rumble {
        kinds.push(InputDeviceCapabilityKind::Haptics);
    }

    let axes = vec![
        InputAxisMetadata {
            code: 0,
            minimum: -1.0,
            maximum: 1.0,
            flat: 0.0,
            fuzz: 0.0,
            resolution: 0.0,
        },
        InputAxisMetadata {
            code: 1,
            minimum: -1.0,
            maximum: 1.0,
            flat: 0.0,
            fuzz: 0.0,
            resolution: 0.0,
        },
        InputAxisMetadata {
            code: 2,
            minimum: -1.0,
            maximum: 1.0,
            flat: 0.0,
            fuzz: 0.0,
            resolution: 0.0,
        },
        InputAxisMetadata {
            code: 3,
            minimum: -1.0,
            maximum: 1.0,
            flat: 0.0,
            fuzz: 0.0,
            resolution: 0.0,
        },
    ];

    let mut buttons = Vec::new();
    for code in 0..u32::from(XINPUT_STANDARD_BUTTON_COUNT) {
        let is_analog = code == 6 || code == 7;
        buttons.push(InputButtonMetadata {
            code,
            analog: is_analog,
        });
    }

    Ok(InputDeviceCapabilities {
        kinds: binding.store_array(kinds),
        axes: binding.store_array(axes),
        buttons: binding.store_array(buttons),
        metadata_origin: InputCapabilityMetadataOrigin::Mixed,
        axis_metadata_fidelity: InputCapabilityMetadataFidelity::Full,
        button_metadata_fidelity: InputCapabilityMetadataFidelity::Full,
        supports_relative_pointer: false,
        supports_pointer_grab: false,
        supports_pointer_capture: false,
        supports_pointer_warp: false,
        supports_text_input: false,
        supports_composition: false,
        supports_rumble,
        supports_trigger_rumble: false,
        supports_sensors: false,
        supports_battery_state: true,
        supports_light_control: false,
        supports_raw_hid: false,
        supports_player_index: true,
    })
}

/// Build one full gamepad-state snapshot from one xinput user index.
pub(super) fn gamepad_state_for_xinput(
    binding: &BindingCallContext,
    user_index: u8,
    player_index: u8,
    operation: &'static str,
) -> RuntimeResult<InputGamepadState> {
    let state = query_xinput_state(user_index, operation)?;
    let capabilities = query_xinput_capabilities(user_index);
    let connection_type = connection_type_from_capabilities(capabilities);
    let supports_rumble = supports_rumble(capabilities);
    let battery = query_xinput_battery(user_index, operation)?;
    Ok(InputGamepadState {
        timestamp_ns: now_timestamp_ns(),
        connected: true,
        mapping: InputGamepadMappingType::Standard,
        connection_type,
        player_index,
        battery,
        supports_rumble,
        supports_trigger_rumble: false,
        axes: binding.store_array(standard_gamepad_axes(&state)),
        buttons: binding.store_array(xinput_standard_button_states(&state)),
        touches: binding.store_array(Vec::new()),
    })
}

/// Return supported haptic effect kinds for one xinput user index.
pub(super) fn haptics_effects_for_xinput(
    user_index: u8,
    operation: &'static str,
) -> RuntimeResult<Vec<InputHapticEffectType>> {
    let _ = query_xinput_state(user_index, operation)?;
    let supports_rumble = supports_rumble(query_xinput_capabilities(user_index));
    if !supports_rumble {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(vec![InputHapticEffectType::DualRumble])
}

/// Play one haptic effect on one xinput user index.
pub(super) fn play_haptics_for_xinput(
    user_index: u8,
    effect: InputHapticEffectType,
    params: InputHapticEffectParameters,
    operation: &'static str,
) -> RuntimeResult<InputHapticsResult> {
    let _ = query_xinput_state(user_index, operation)?;
    if !supports_rumble(query_xinput_capabilities(user_index)) {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    if effect != InputHapticEffectType::DualRumble {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    let left = (params.strong_magnitude.clamp(0.0, 1.0) * f64::from(u16::MAX)) as u16;
    let right = (params.weak_magnitude.clamp(0.0, 1.0) * f64::from(u16::MAX)) as u16;
    let vibration = XINPUT_VIBRATION {
        wLeftMotorSpeed: left,
        wRightMotorSpeed: right,
    };

    let status = unsafe { XInputSetState(u32::from(user_index), &vibration) };
    if status != ERROR_SUCCESS {
        return Err(xinput_error(
            operation,
            "XInputSetState",
            status,
            "failed to submit controller vibration",
        ));
    }

    Ok(InputHapticsResult::Complete)
}

/// Stop all active haptic effects on one xinput user index.
pub(super) fn stop_haptics_for_xinput(
    user_index: u8,
    operation: &'static str,
) -> RuntimeResult<()> {
    let _ = query_xinput_state(user_index, operation)?;
    if !supports_rumble(query_xinput_capabilities(user_index)) {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    let vibration = XINPUT_VIBRATION {
        wLeftMotorSpeed: 0,
        wRightMotorSpeed: 0,
    };
    let status = unsafe { XInputSetState(u32::from(user_index), &vibration) };
    if status == ERROR_SUCCESS || status == ERROR_DEVICE_NOT_CONNECTED {
        return Ok(());
    }

    Err(xinput_error(
        operation,
        "XInputSetState",
        status,
        "failed to stop controller vibration",
    ))
}

#[cfg(test)]
mod tests {
    use crate::platform::input::host::windows::xinput::{
        normalize_stick_axis, parse_xinput_device_id, xinput_device_id,
    };

    /// Parse valid xinput identifiers into user indices.
    #[test]
    fn test_parse_xinput_device_id_accepts_valid_ids() {
        assert_eq!(parse_xinput_device_id("xinput:0"), Some(0));
        assert_eq!(parse_xinput_device_id("xinput:3"), Some(3));
    }

    /// Reject malformed xinput identifiers.
    #[test]
    fn test_parse_xinput_device_id_rejects_invalid_ids() {
        assert_eq!(parse_xinput_device_id("xinput"), None);
        assert_eq!(parse_xinput_device_id("xinput:"), None);
        assert_eq!(parse_xinput_device_id("xinput:4"), None);
        assert_eq!(parse_xinput_device_id("xinput:a"), None);
        assert_eq!(parse_xinput_device_id("raw:device:1"), None);
    }

    /// Normalize signed stick samples into expected axis range endpoints.
    #[test]
    fn test_normalize_stick_axis_maps_signed_range() {
        assert_eq!(normalize_stick_axis(i16::MAX), 1.0);
        assert_eq!(normalize_stick_axis(i16::MIN), -1.0);
        assert_eq!(normalize_stick_axis(0), 0.0);
    }

    /// Build stable xinput device identifiers.
    #[test]
    fn test_xinput_device_id_formats_prefix() {
        assert_eq!(xinput_device_id(0), "xinput:0");
        assert_eq!(xinput_device_id(2), "xinput:2");
    }
}
