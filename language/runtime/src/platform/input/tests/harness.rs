#![cfg_attr(not(windows), allow(dead_code, unused_imports))]

use destack_vm as vm;

use super::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::{
    InputDeviceCapabilities, InputDeviceCapabilitiesVm, InputDeviceCapabilityKind, InputDeviceInfo,
    InputDeviceInfoVm, InputDeviceKind, InputEvent, InputEventAction, InputEventKind, InputEventVm,
    InputGamepadState, InputGamepadStateVm, InputHapticEffectParameters,
    InputHapticEffectParametersVm, InputHapticEffectType, InputKeyboardState, InputKeyboardStateVm,
    InputMonitorEvent, InputMonitorEventKind, InputMonitorEventVm, InputPointerState,
    InputPointerStateVm, InputRawHidReport, InputRawHidReportVm, InputSensorConfig,
    InputSensorConfigVm, InputSensorEffectiveConfig, InputSensorEffectiveConfigVm, InputSensorInfo,
    InputSensorInfoVm, InputSensorSample, InputSensorSampleVm, InputTextInputArea,
    InputTextInputAreaVm, InputTouchState, InputTouchStateVm, InputWindowTarget,
    InputWindowTargetVm,
};
use crate::platform::{NativeArray, NativeSlice, NativeStringRef, PlatformError, VmArray, VmSlice};

#[path = "harness.generated.rs"]
mod generated;

#[allow(unused_imports)]
pub(crate) use generated::*;

/// Decoded input device metadata used by tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InputDeviceRecord {
    /// Stable runtime device identifier.
    pub id: String,
    /// Host-visible device name.
    pub name: String,
    /// Device kind classification.
    pub kind: InputDeviceKind,
    /// USB or bus vendor identifier when available.
    pub vendor_id: u16,
    /// USB or bus product identifier when available.
    pub product_id: u16,
    /// Number of logical keys when reported by backend metadata.
    pub key_count: u16,
    /// Number of logical buttons when reported by backend metadata.
    pub button_count: u16,
    /// Number of logical axes when reported by backend metadata.
    pub axis_count: u16,
    /// Connected state at enumeration time.
    pub connected: bool,
    /// Whether this endpoint supports exclusive-grab mode.
    pub supports_exclusive_grab: bool,
    /// Whether this endpoint supports raw event streams.
    pub supports_raw: bool,
    /// Whether this endpoint supports text events.
    pub supports_text: bool,
    /// Whether this endpoint supports haptic output.
    pub supports_rumble: bool,
}

/// Decoded input event metadata used by tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InputEventRecord {
    /// Stable runtime device identifier.
    pub device_id: String,
    /// Event kind selector.
    pub kind: InputEventKind,
    /// Event action selector.
    pub action: InputEventAction,
    /// Event code value.
    pub code: u32,
    /// Event scalar payload value.
    pub value: i64,
    /// Monotonic event sequence number.
    pub sequence: u64,
}

/// Decoded monitor event metadata used by tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InputMonitorEventRecord {
    /// Stable runtime device identifier.
    pub device_id: String,
    /// Monitor event kind selector.
    pub kind: InputMonitorEventKind,
    /// Connected state after this monitor event.
    pub connected: bool,
    /// Monotonic monitor sequence number.
    pub sequence: u64,
}

/// Decoded input capabilities metadata used by tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InputCapabilitiesRecord {
    /// Capability kind entries reported by backend metadata.
    pub kinds: Vec<InputDeviceCapabilityKind>,
    /// Axis codes reported by backend metadata.
    pub axis_codes: Vec<u32>,
    /// Button codes reported by backend metadata.
    pub button_codes: Vec<u32>,
    /// Whether this endpoint supports raw relative pointer semantics.
    pub supports_relative_pointer: bool,
    /// Whether this endpoint supports exclusive pointer grab.
    pub supports_pointer_grab: bool,
    /// Whether this endpoint supports pointer capture semantics.
    pub supports_pointer_capture: bool,
    /// Whether this endpoint supports pointer warping.
    pub supports_pointer_warp: bool,
    /// Whether this endpoint supports text input payloads.
    pub supports_text_input: bool,
    /// Whether this endpoint supports composition payloads.
    pub supports_composition: bool,
    /// Whether this endpoint supports rumble output.
    pub supports_rumble: bool,
    /// Whether this endpoint supports raw HID streams.
    pub supports_raw_hid: bool,
}

/// Decoded raw-hid report payload used by tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InputRawHidReportRecord {
    /// Monotonic report timestamp in nanoseconds.
    pub timestamp_ns: u64,
    /// Monotonic report sequence number.
    pub sequence: u64,
    /// Report identifier byte.
    pub report_id: u8,
    /// Raw report bytes.
    pub data: Vec<u8>,
}

/// Decoded touch-state payload used by tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InputTouchStateRecord {
    /// Monotonic snapshot timestamp in nanoseconds.
    pub timestamp_ns: u64,
    /// Monotonic snapshot sequence number.
    pub sequence: u64,
    /// Stable runtime device identifier.
    pub device_id: String,
    /// Number of contacts in this snapshot.
    pub contact_count: usize,
}

/// Decoded gamepad-state payload used by tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InputGamepadStateRecord {
    /// Whether the gamepad is currently connected.
    pub connected: bool,
    /// Number of standardized axis slots.
    pub axis_count: usize,
    /// Number of standardized button slots.
    pub button_count: usize,
}

/// Decoded keyboard-state payload used by tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InputKeyboardStateRecord {
    /// Stable runtime device identifier.
    pub device_id: String,
    /// Monotonic snapshot sequence number.
    pub sequence: u64,
    /// Keyboard modifier bitset for this snapshot.
    pub modifiers: u32,
    /// Number of pressed key-code entries.
    pub pressed_code_count: usize,
    /// Number of pressed scan-code entries.
    pub pressed_scan_code_count: usize,
}

/// Decode event action, code, and value from one native typed payload.
fn decode_native_event_payload(
    kind: InputEventKind,
    value: InputEvent,
) -> (InputEventAction, u32, i64) {
    match kind {
        InputEventKind::Key => (
            value.payload.key.action,
            value.payload.key.backend_code,
            value.payload.key.backend_value,
        ),
        InputEventKind::PointerMotion => (InputEventAction::Move, 0, 0),
        InputEventKind::PointerButton => (
            value.payload.pointer_button.action,
            value.payload.pointer_button.backend_code,
            value.payload.pointer_button.backend_value,
        ),
        InputEventKind::Scroll => (
            InputEventAction::Scroll,
            0,
            value.payload.scroll.wheel_y as i64,
        ),
        InputEventKind::Touch => (
            value.payload.touch.action,
            value.payload.touch.contact_id,
            value.payload.touch.pressure as i64,
        ),
        InputEventKind::Gamepad => (
            value.payload.gamepad.action,
            value.payload.gamepad.backend_code,
            value.payload.gamepad.backend_value,
        ),
        InputEventKind::Text => (InputEventAction::Text, 0, 0),
        InputEventKind::Device => (
            value.payload.device.action,
            value.payload.device.backend_code,
            value.payload.device.backend_value,
        ),
        InputEventKind::Sensor => (
            value.payload.sensor.action,
            value.payload.sensor.backend_code,
            value.payload.sensor.backend_value,
        ),
        InputEventKind::Composition => (value.payload.composition.action, 0, 0),
    }
}

/// Decode event action, code, and value from one VM typed payload.
fn decode_vm_event_payload(
    kind: InputEventKind,
    value: InputEventVm,
) -> (InputEventAction, u32, i64) {
    match kind {
        InputEventKind::Key => (
            value.payload.key.action,
            value.payload.key.backend_code,
            value.payload.key.backend_value,
        ),
        InputEventKind::PointerMotion => (InputEventAction::Move, 0, 0),
        InputEventKind::PointerButton => (
            value.payload.pointer_button.action,
            value.payload.pointer_button.backend_code,
            value.payload.pointer_button.backend_value,
        ),
        InputEventKind::Scroll => (
            InputEventAction::Scroll,
            0,
            value.payload.scroll.wheel_y as i64,
        ),
        InputEventKind::Touch => (
            value.payload.touch.action,
            value.payload.touch.contact_id,
            value.payload.touch.pressure as i64,
        ),
        InputEventKind::Gamepad => (
            value.payload.gamepad.action,
            value.payload.gamepad.backend_code,
            value.payload.gamepad.backend_value,
        ),
        InputEventKind::Text => (InputEventAction::Text, 0, 0),
        InputEventKind::Device => (
            value.payload.device.action,
            value.payload.device.backend_code,
            value.payload.device.backend_value,
        ),
        InputEventKind::Sensor => (
            value.payload.sensor.action,
            value.payload.sensor.backend_code,
            value.payload.sensor.backend_value,
        ),
        InputEventKind::Composition => (value.payload.composition.action, 0, 0),
    }
}

impl<'call> InputHarnessContext<'call> {
    /// Build one backend-specific byte-slice value.
    pub(crate) fn bytes_value(
        &mut self,
        bytes: &[u8],
    ) -> RuntimeResult<HarnessValue<NativeSlice<u8>, VmSlice<u8>>> {
        match self.vm_context_mut() {
            Some(context) => {
                let bytes = VmSlice::from_values(context, bytes)?;
                Ok(self.harness_value_vm(bytes))
            }
            None => {
                let bytes = self.call_context.store_slice(bytes.to_vec());
                Ok(self.harness_value(bytes))
            }
        }
    }

    /// Decode one backend-specific byte-slice value into bytes.
    pub(crate) fn bytes_from_value(
        &mut self,
        value: HarnessValue<NativeSlice<u8>, VmSlice<u8>>,
    ) -> RuntimeResult<Vec<u8>> {
        match value {
            HarnessValue::Native(value) => {
                let bytes = unsafe { value.as_slice()? };
                Ok(bytes.to_vec())
            }
            HarnessValue::Vm(value) => {
                let context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "vm context is required for vm byte-slice values",
                    ))
                    .boxed()
                })?;
                value.read_bytes(context)
            }
        }
    }

    /// Return the vm context when this harness executes vm bindings.
    #[allow(clippy::mut_from_ref)]
    fn vm_context_mut(&self) -> Option<&mut vm::ExternalCallContext<'_>> {
        // recover mutable vm context from stored raw pointer
        self.vm_context
            .map(|context| unsafe { &mut *(context as *mut vm::ExternalCallContext<'_>) })
    }

    /// Build one backend-specific string value.
    pub(crate) fn string_value(
        &self,
        value: &str,
    ) -> HarnessValue<NativeStringRef, vm::StringHandle> {
        // build vm or native string payload based on active harness kind
        match self.vm_context_mut() {
            Some(context) => {
                let value = vm::StringHandle::new(context.intern_string(value));
                self.harness_value_vm(value)
            }
            None => self.harness_value(self.call_context.store_string(value)),
        }
    }

    /// Build one window-target value for the active harness engine.
    pub(crate) fn window_target(
        &self,
        value: InputWindowTarget,
    ) -> HarnessValue<InputWindowTarget, InputWindowTargetVm> {
        match self.vm_context_mut() {
            Some(_) => self.harness_value_vm(value),
            None => self.harness_value(value),
        }
    }

    /// Build one text-area value for the active harness engine.
    pub(crate) fn text_input_area(
        &self,
        value: InputTextInputArea,
    ) -> HarnessValue<InputTextInputArea, InputTextInputAreaVm> {
        match self.vm_context_mut() {
            Some(_) => self.harness_value_vm(value),
            None => self.harness_value(value),
        }
    }

    /// Build one sensor-config value for the active harness engine.
    pub(crate) fn sensor_config(
        &self,
        value: InputSensorConfig,
    ) -> HarnessValue<InputSensorConfig, InputSensorConfigVm> {
        match self.vm_context_mut() {
            Some(_) => self.harness_value_vm(value),
            None => self.harness_value(value),
        }
    }

    /// Build one haptics-parameter value for the active harness engine.
    pub(crate) fn haptics_parameters(
        &self,
        value: InputHapticEffectParameters,
    ) -> HarnessValue<InputHapticEffectParameters, InputHapticEffectParametersVm> {
        match self.vm_context_mut() {
            Some(_) => self.harness_value_vm(value),
            None => self.harness_value(value),
        }
    }

    /// Decode one text-area harness value into one shared payload.
    pub(crate) fn text_input_area_from(
        &self,
        value: HarnessValue<InputTextInputArea, InputTextInputAreaVm>,
    ) -> InputTextInputArea {
        match value {
            HarnessValue::Native(value) => value,
            HarnessValue::Vm(value) => value,
        }
    }

    /// Decode one list result into normalized device records.
    pub(crate) fn device_records_from_value(
        &self,
        value: HarnessValue<NativeSlice<InputDeviceInfo>, VmSlice<InputDeviceInfoVm>>,
    ) -> RuntimeResult<Vec<InputDeviceRecord>> {
        match value {
            HarnessValue::Native(value) => {
                // decode native slice payload into normalized records
                let values = unsafe { value.as_slice()? };
                let mut records = Vec::with_capacity(values.len());
                for value in values {
                    records.push(InputDeviceRecord {
                        id: unsafe { value.id.as_str()? }.to_string(),
                        name: unsafe { value.name.as_str()? }.to_string(),
                        kind: value.kind,
                        vendor_id: value.vendor_id,
                        product_id: value.product_id,
                        key_count: value.key_count,
                        button_count: value.button_count,
                        axis_count: value.axis_count,
                        connected: value.connected,
                        supports_exclusive_grab: value.supports_exclusive_grab,
                        supports_raw: value.supports_raw,
                        supports_text: value.supports_text,
                        supports_rumble: value.supports_rumble,
                    });
                }

                Ok(records)
            }
            HarnessValue::Vm(value) => {
                // decode vm slice payload into normalized records
                let context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "vm context is required for vm input-device values",
                    ))
                    .boxed()
                })?;
                let values = value.read_values(context)?;
                let mut records = Vec::with_capacity(values.len());
                for value in values {
                    let id = context
                        .string_ref(value.id)
                        .map_err(|error| RuntimeError::from(error).boxed())?
                        .as_str()
                        .to_string();
                    let name = context
                        .string_ref(value.name)
                        .map_err(|error| RuntimeError::from(error).boxed())?
                        .as_str()
                        .to_string();
                    records.push(InputDeviceRecord {
                        id,
                        name,
                        kind: value.kind,
                        vendor_id: value.vendor_id,
                        product_id: value.product_id,
                        key_count: value.key_count,
                        button_count: value.button_count,
                        axis_count: value.axis_count,
                        connected: value.connected,
                        supports_exclusive_grab: value.supports_exclusive_grab,
                        supports_raw: value.supports_raw,
                        supports_text: value.supports_text,
                        supports_rumble: value.supports_rumble,
                    });
                }
                Ok(records)
            }
        }
    }

    /// Decode one event wrapper into one unified event payload.
    pub(crate) fn event_from_value(
        &self,
        value: HarnessValue<InputEvent, InputEventVm>,
    ) -> RuntimeResult<InputEventRecord> {
        match value {
            HarnessValue::Native(value) => {
                // decode native event strings
                let device_id = unsafe { value.device_id.as_str()? }.to_string();
                let (action, code, event_value) = decode_native_event_payload(value.kind, value);
                Ok(InputEventRecord {
                    device_id,
                    kind: value.kind,
                    action,
                    code,
                    value: event_value,
                    sequence: value.sequence,
                })
            }
            HarnessValue::Vm(value) => {
                // decode vm event strings
                let context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "vm context is required for vm input-event values",
                    ))
                    .boxed()
                })?;
                let device_id = context
                    .string_ref(value.device_id)
                    .map_err(|error| RuntimeError::from(error).boxed())?
                    .as_str()
                    .to_string();
                let (action, code, event_value) = decode_vm_event_payload(value.kind, value);
                Ok(InputEventRecord {
                    device_id,
                    kind: value.kind,
                    action,
                    code,
                    value: event_value,
                    sequence: value.sequence,
                })
            }
        }
    }

    /// Decode one monitor event wrapper into one unified monitor payload.
    pub(crate) fn monitor_event_from_value(
        &self,
        value: HarnessValue<InputMonitorEvent, InputMonitorEventVm>,
    ) -> RuntimeResult<InputMonitorEventRecord> {
        match value {
            HarnessValue::Native(value) => {
                let device_id = unsafe { value.device_id.as_str()? }.to_string();
                Ok(InputMonitorEventRecord {
                    device_id,
                    kind: value.kind,
                    connected: value.connected,
                    sequence: value.sequence,
                })
            }
            HarnessValue::Vm(value) => {
                let context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "vm context is required for vm monitor-event values",
                    ))
                    .boxed()
                })?;
                let device_id = context
                    .string_ref(value.device_id)
                    .map_err(|error| RuntimeError::from(error).boxed())?
                    .as_str()
                    .to_string();
                Ok(InputMonitorEventRecord {
                    device_id,
                    kind: value.kind,
                    connected: value.connected,
                    sequence: value.sequence,
                })
            }
        }
    }

    /// Decode one capabilities wrapper into one unified capabilities payload.
    pub(crate) fn capabilities_from_value(
        &self,
        value: HarnessValue<InputDeviceCapabilities, InputDeviceCapabilitiesVm>,
    ) -> RuntimeResult<InputCapabilitiesRecord> {
        match value {
            HarnessValue::Native(value) => {
                let kinds = unsafe { value.kinds.as_slice()? }.to_vec();
                let axes = unsafe { value.axes.as_slice()? };
                let mut axis_codes = Vec::with_capacity(axes.len());
                for axis in axes {
                    axis_codes.push(axis.code);
                }
                let buttons = unsafe { value.buttons.as_slice()? };
                let mut button_codes = Vec::with_capacity(buttons.len());
                for button in buttons {
                    button_codes.push(button.code);
                }

                Ok(InputCapabilitiesRecord {
                    kinds,
                    axis_codes,
                    button_codes,
                    supports_relative_pointer: value.supports_relative_pointer,
                    supports_pointer_grab: value.supports_pointer_grab,
                    supports_pointer_capture: value.supports_pointer_capture,
                    supports_pointer_warp: value.supports_pointer_warp,
                    supports_text_input: value.supports_text_input,
                    supports_composition: value.supports_composition,
                    supports_rumble: value.supports_rumble,
                    supports_raw_hid: value.supports_raw_hid,
                })
            }
            HarnessValue::Vm(value) => {
                let context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "vm context is required for vm input-capabilities values",
                    ))
                    .boxed()
                })?;

                let kinds = value.kinds.read_values(context)?;
                let axes = value.axes.read_values(context)?;
                let mut axis_codes = Vec::with_capacity(axes.len());
                for axis in axes {
                    axis_codes.push(axis.code);
                }
                let buttons = value.buttons.read_values(context)?;
                let mut button_codes = Vec::with_capacity(buttons.len());
                for button in buttons {
                    button_codes.push(button.code);
                }

                Ok(InputCapabilitiesRecord {
                    kinds,
                    axis_codes,
                    button_codes,
                    supports_relative_pointer: value.supports_relative_pointer,
                    supports_pointer_grab: value.supports_pointer_grab,
                    supports_pointer_capture: value.supports_pointer_capture,
                    supports_pointer_warp: value.supports_pointer_warp,
                    supports_text_input: value.supports_text_input,
                    supports_composition: value.supports_composition,
                    supports_rumble: value.supports_rumble,
                    supports_raw_hid: value.supports_raw_hid,
                })
            }
        }
    }

    /// Decode one raw-hid report wrapper into one unified raw-hid payload.
    pub(crate) fn raw_hid_report_from_value(
        &mut self,
        value: HarnessValue<InputRawHidReport, InputRawHidReportVm>,
    ) -> RuntimeResult<InputRawHidReportRecord> {
        match value {
            HarnessValue::Native(value) => {
                let data = unsafe { value.data.as_slice()? }.to_vec();
                Ok(InputRawHidReportRecord {
                    timestamp_ns: value.timestamp_ns,
                    sequence: value.sequence,
                    report_id: value.report_id,
                    data,
                })
            }
            HarnessValue::Vm(value) => {
                let context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "vm context is required for vm raw-hid-report values",
                    ))
                    .boxed()
                })?;
                let data = value.data.read_bytes(context)?;
                Ok(InputRawHidReportRecord {
                    timestamp_ns: value.timestamp_ns,
                    sequence: value.sequence,
                    report_id: value.report_id,
                    data,
                })
            }
        }
    }

    /// Decode one touch-state wrapper into one unified touch-state payload.
    pub(crate) fn touch_state_from_value(
        &self,
        value: HarnessValue<InputTouchState, InputTouchStateVm>,
    ) -> RuntimeResult<InputTouchStateRecord> {
        match value {
            HarnessValue::Native(value) => {
                let device_id = unsafe { value.device_id.as_str()? }.to_string();
                let contacts = unsafe { value.contacts.as_slice()? };
                Ok(InputTouchStateRecord {
                    timestamp_ns: value.timestamp_ns,
                    sequence: value.sequence,
                    device_id,
                    contact_count: contacts.len(),
                })
            }
            HarnessValue::Vm(value) => {
                let context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "vm context is required for vm touch-state values",
                    ))
                    .boxed()
                })?;
                let device_id = context
                    .string_ref(value.device_id)
                    .map_err(|error| RuntimeError::from(error).boxed())?
                    .as_str()
                    .to_string();
                let contacts = value.contacts.read_values(context)?;
                Ok(InputTouchStateRecord {
                    timestamp_ns: value.timestamp_ns,
                    sequence: value.sequence,
                    device_id,
                    contact_count: contacts.len(),
                })
            }
        }
    }

    /// Decode one gamepad-state wrapper into one unified gamepad-state payload.
    pub(crate) fn gamepad_state_from_value(
        &self,
        value: HarnessValue<InputGamepadState, InputGamepadStateVm>,
    ) -> RuntimeResult<InputGamepadStateRecord> {
        match value {
            HarnessValue::Native(value) => {
                let axes = unsafe { value.axes.as_slice()? };
                let buttons = unsafe { value.buttons.as_slice()? };
                Ok(InputGamepadStateRecord {
                    connected: value.connected,
                    axis_count: axes.len(),
                    button_count: buttons.len(),
                })
            }
            HarnessValue::Vm(value) => {
                let context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "vm context is required for vm gamepad-state values",
                    ))
                    .boxed()
                })?;
                let axes = value.axes.read_values(context)?;
                let buttons = value.buttons.read_values(context)?;
                Ok(InputGamepadStateRecord {
                    connected: value.connected,
                    axis_count: axes.len(),
                    button_count: buttons.len(),
                })
            }
        }
    }

    /// Decode one keyboard-state wrapper into one unified keyboard-state payload.
    pub(crate) fn keyboard_state_from_value(
        &self,
        value: HarnessValue<InputKeyboardState, InputKeyboardStateVm>,
    ) -> RuntimeResult<InputKeyboardStateRecord> {
        match value {
            HarnessValue::Native(value) => {
                let device_id = unsafe { value.device_id.as_str()? }.to_string();
                let pressed_codes = unsafe { value.pressed_codes.as_slice()? };
                let pressed_scan_codes = unsafe { value.pressed_scan_codes.as_slice()? };

                Ok(InputKeyboardStateRecord {
                    device_id,
                    sequence: value.sequence,
                    modifiers: value.modifiers,
                    pressed_code_count: pressed_codes.len(),
                    pressed_scan_code_count: pressed_scan_codes.len(),
                })
            }
            HarnessValue::Vm(value) => {
                let context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "vm context is required for vm keyboard-state values",
                    ))
                    .boxed()
                })?;
                let device_id = context
                    .string_ref(value.device_id)
                    .map_err(|error| RuntimeError::from(error).boxed())?
                    .as_str()
                    .to_string();
                let pressed_codes = value.pressed_codes.read_values(context)?;
                let pressed_scan_codes = value.pressed_scan_codes.read_values(context)?;

                Ok(InputKeyboardStateRecord {
                    device_id,
                    sequence: value.sequence,
                    modifiers: value.modifiers,
                    pressed_code_count: pressed_codes.len(),
                    pressed_scan_code_count: pressed_scan_codes.len(),
                })
            }
        }
    }

    /// Decode one pointer-state wrapper into one unified pointer-state payload.
    pub(crate) fn pointer_state_from_value(
        &self,
        value: HarnessValue<InputPointerState, InputPointerStateVm>,
    ) -> InputPointerState {
        match value {
            HarnessValue::Native(value) => value,
            HarnessValue::Vm(value) => value,
        }
    }

    /// Decode one haptics-effects wrapper into one unified effect list.
    pub(crate) fn haptic_effects_from_value(
        &self,
        value: HarnessValue<NativeArray<InputHapticEffectType>, VmArray<InputHapticEffectType>>,
    ) -> RuntimeResult<Vec<InputHapticEffectType>> {
        match value {
            HarnessValue::Native(value) => {
                let values = unsafe { value.as_slice()? };
                Ok(values.to_vec())
            }
            HarnessValue::Vm(value) => {
                let context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "vm context is required for vm haptics-effect values",
                    ))
                    .boxed()
                })?;
                value.read_values(context)
            }
        }
    }

    /// Decode one sensor-config wrapper into one unified payload.
    pub(crate) fn sensor_effective_config_from_value(
        &self,
        value: HarnessValue<InputSensorEffectiveConfig, InputSensorEffectiveConfigVm>,
    ) -> InputSensorEffectiveConfig {
        match value {
            HarnessValue::Native(value) => value,
            HarnessValue::Vm(value) => value,
        }
    }

    /// Decode one sensor-sample wrapper into one unified payload.
    pub(crate) fn sensor_sample_from_value(
        &self,
        value: HarnessValue<InputSensorSample, InputSensorSampleVm>,
    ) -> InputSensorSample {
        match value {
            HarnessValue::Native(value) => value,
            HarnessValue::Vm(value) => value,
        }
    }

    /// Decode one sensor-info list wrapper into one unified payload list.
    pub(crate) fn sensor_infos_from_value(
        &self,
        value: HarnessValue<NativeArray<InputSensorInfo>, VmArray<InputSensorInfoVm>>,
    ) -> RuntimeResult<Vec<InputSensorInfo>> {
        match value {
            HarnessValue::Native(value) => {
                let values = unsafe { value.as_slice()? };
                Ok(values.to_vec())
            }
            HarnessValue::Vm(value) => {
                let context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "vm context is required for vm sensor-info values",
                    ))
                    .boxed()
                })?;
                value.read_values(context)
            }
        }
    }
}
