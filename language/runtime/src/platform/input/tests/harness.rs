#![cfg_attr(not(windows), allow(dead_code, unused_imports))]

use destack_vm as vm;

use super::InputHarnessContext;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::{
    InputDeviceCapabilities, InputDeviceCapabilitiesVm, InputDeviceCapabilityKind,
    InputDeviceDescriptor, InputDeviceDescriptorVm, InputDeviceKind, InputEvent, InputEventAction,
    InputEventVm, InputGamepadState, InputGamepadStateVm, InputHapticEffectParameters,
    InputHapticEffectParametersVm, InputHapticEffectType, InputKeyboardState, InputKeyboardStateVm,
    InputMonitorEvent, InputMonitorEventVm, InputPointerState, InputPointerStateVm,
    InputRawHidReport, InputRawHidReportVm, InputSensorConfig, InputSensorConfigVm,
    InputSensorDescriptor, InputSensorDescriptorVm, InputSensorEffectiveConfig,
    InputSensorEffectiveConfigVm, InputSensorSample, InputSensorSampleVm, InputTextGeometry,
    InputTextGeometryVm, InputTextInputType, InputTextRange, InputTextSessionConfig,
    InputTextSessionConfigVm, InputTextSessionState, InputTextSessionStateVm, InputTouchState,
    InputTouchStateVm, InputWindowTarget, InputWindowTargetVm,
};
use crate::platform::{
    NativeAbiCodec, NativeArray, NativeSlice, NativeStringRef, PlatformError, VmAbiCodec, VmArray,
    VmSlice,
};
use crate::tests::platform::vm_test_string;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputEventRecordKind {
    /// Key press or release.
    Key,
    /// Pointer motion.
    PointerMotion,
    /// Pointer button press or release.
    PointerButton,
    /// Scroll-wheel or gesture scroll.
    Scroll,
    /// Touch contact event.
    Touch,
    /// Gamepad event.
    Gamepad,
    /// Text input event.
    Text,
    /// Device connect or state event.
    Device,
    /// Sensor sample event.
    Sensor,
    /// IME composition event.
    Composition,
}

/// Decoded input event metadata used by tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InputEventRecord {
    /// Stable runtime device identifier.
    pub device_id: String,
    /// Monotonic event timestamp in nanoseconds.
    pub timestamp_ns: u64,
    /// Event kind selector.
    pub kind: InputEventRecordKind,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputMonitorEventRecordKind {
    /// Device monitor change event.
    Change,
    /// Device monitor connect event.
    Connect,
    /// Device monitor disconnect event.
    Disconnect,
}

/// Decoded monitor event metadata used by tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InputMonitorEventRecord {
    /// Stable runtime device identifier.
    pub device_id: String,
    /// Monotonic monitor timestamp in nanoseconds.
    pub timestamp_ns: u64,
    /// Monitor event kind selector.
    pub kind: InputMonitorEventRecordKind,
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
    /// Monotonic snapshot timestamp in nanoseconds.
    pub timestamp_ns: u64,
    /// Monotonic snapshot sequence number.
    pub sequence: u64,
    /// Keyboard modifier bitset for this snapshot.
    pub modifiers: u32,
    /// Number of pressed key-code entries.
    pub pressed_code_count: usize,
    /// Number of pressed scan-code entries.
    pub pressed_scan_code_count: usize,
}

/// Decode one native input event into one normalized event record.
fn decode_native_event(value: InputEvent) -> RuntimeResult<InputEventRecord> {
    match value {
        InputEvent::InputKeyEvent(value) => Ok(InputEventRecord {
            device_id: unsafe { value.metadata.device_id.as_str()? }.to_string(),
            timestamp_ns: value.metadata.timestamp_ns,
            kind: InputEventRecordKind::Key,
            action: value.payload.action,
            code: value.payload.backend_code,
            value: value.payload.backend_value,
            sequence: value.metadata.sequence,
        }),
        InputEvent::InputPointerMotionEvent(value) => Ok(InputEventRecord {
            device_id: unsafe { value.metadata.device_id.as_str()? }.to_string(),
            timestamp_ns: value.metadata.timestamp_ns,
            kind: InputEventRecordKind::PointerMotion,
            action: InputEventAction::Move,
            code: 0,
            value: 0,
            sequence: value.metadata.sequence,
        }),
        InputEvent::InputPointerButtonEvent(value) => Ok(InputEventRecord {
            device_id: unsafe { value.metadata.device_id.as_str()? }.to_string(),
            timestamp_ns: value.metadata.timestamp_ns,
            kind: InputEventRecordKind::PointerButton,
            action: value.payload.action,
            code: value.payload.backend_code,
            value: value.payload.backend_value,
            sequence: value.metadata.sequence,
        }),
        InputEvent::InputScrollEvent(value) => Ok(InputEventRecord {
            device_id: unsafe { value.metadata.device_id.as_str()? }.to_string(),
            timestamp_ns: value.metadata.timestamp_ns,
            kind: InputEventRecordKind::Scroll,
            action: InputEventAction::Scroll,
            code: 0,
            value: value.payload.delta_y as i64,
            sequence: value.metadata.sequence,
        }),
        InputEvent::InputTouchEvent(value) => Ok(InputEventRecord {
            device_id: unsafe { value.metadata.device_id.as_str()? }.to_string(),
            timestamp_ns: value.metadata.timestamp_ns,
            kind: InputEventRecordKind::Touch,
            action: value.payload.action,
            code: value.payload.pointer_id as u32,
            value: value.payload.pressure as i64,
            sequence: value.metadata.sequence,
        }),
        InputEvent::InputGamepadEvent(value) => Ok(InputEventRecord {
            device_id: unsafe { value.metadata.device_id.as_str()? }.to_string(),
            timestamp_ns: value.metadata.timestamp_ns,
            kind: InputEventRecordKind::Gamepad,
            action: value.payload.action,
            code: value.payload.backend_code,
            value: value.payload.backend_value,
            sequence: value.metadata.sequence,
        }),
        InputEvent::InputTextEvent(value) => Ok(InputEventRecord {
            device_id: unsafe { value.metadata.device_id.as_str()? }.to_string(),
            timestamp_ns: value.metadata.timestamp_ns,
            kind: InputEventRecordKind::Text,
            action: InputEventAction::Text,
            code: 0,
            value: 0,
            sequence: value.metadata.sequence,
        }),
        InputEvent::InputDeviceEvent(value) => Ok(InputEventRecord {
            device_id: unsafe { value.metadata.device_id.as_str()? }.to_string(),
            timestamp_ns: value.metadata.timestamp_ns,
            kind: InputEventRecordKind::Device,
            action: value.payload.action,
            code: value.payload.backend_code,
            value: value.payload.backend_value,
            sequence: value.metadata.sequence,
        }),
        InputEvent::InputSensorEvent(value) => Ok(InputEventRecord {
            device_id: unsafe { value.metadata.device_id.as_str()? }.to_string(),
            timestamp_ns: value.metadata.timestamp_ns,
            kind: InputEventRecordKind::Sensor,
            action: value.payload.action,
            code: value.payload.backend_code,
            value: value.payload.backend_value,
            sequence: value.metadata.sequence,
        }),
        InputEvent::InputCompositionEvent(value) => Ok(InputEventRecord {
            device_id: unsafe { value.metadata.device_id.as_str()? }.to_string(),
            timestamp_ns: value.metadata.timestamp_ns,
            kind: InputEventRecordKind::Composition,
            action: value.payload.action,
            code: 0,
            value: 0,
            sequence: value.metadata.sequence,
        }),
    }
}

/// Decode one VM input event into one normalized event record.
fn decode_vm_event(
    context: &mut vm::BindingContext<'_>,
    value: InputEventVm,
) -> RuntimeResult<InputEventRecord> {
    match value {
        InputEventVm::InputKeyEvent(value) => Ok(InputEventRecord {
            device_id: context
                .string_ref(value.metadata.device_id)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string(),
            timestamp_ns: value.metadata.timestamp_ns,
            kind: InputEventRecordKind::Key,
            action: value.payload.action,
            code: value.payload.backend_code,
            value: value.payload.backend_value,
            sequence: value.metadata.sequence,
        }),
        InputEventVm::InputPointerMotionEvent(value) => Ok(InputEventRecord {
            device_id: context
                .string_ref(value.metadata.device_id)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string(),
            timestamp_ns: value.metadata.timestamp_ns,
            kind: InputEventRecordKind::PointerMotion,
            action: InputEventAction::Move,
            code: 0,
            value: 0,
            sequence: value.metadata.sequence,
        }),
        InputEventVm::InputPointerButtonEvent(value) => Ok(InputEventRecord {
            device_id: context
                .string_ref(value.metadata.device_id)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string(),
            timestamp_ns: value.metadata.timestamp_ns,
            kind: InputEventRecordKind::PointerButton,
            action: value.payload.action,
            code: value.payload.backend_code,
            value: value.payload.backend_value,
            sequence: value.metadata.sequence,
        }),
        InputEventVm::InputScrollEvent(value) => Ok(InputEventRecord {
            device_id: context
                .string_ref(value.metadata.device_id)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string(),
            timestamp_ns: value.metadata.timestamp_ns,
            kind: InputEventRecordKind::Scroll,
            action: InputEventAction::Scroll,
            code: 0,
            value: value.payload.delta_y as i64,
            sequence: value.metadata.sequence,
        }),
        InputEventVm::InputTouchEvent(value) => Ok(InputEventRecord {
            device_id: context
                .string_ref(value.metadata.device_id)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string(),
            timestamp_ns: value.metadata.timestamp_ns,
            kind: InputEventRecordKind::Touch,
            action: value.payload.action,
            code: value.payload.pointer_id as u32,
            value: value.payload.pressure as i64,
            sequence: value.metadata.sequence,
        }),
        InputEventVm::InputGamepadEvent(value) => Ok(InputEventRecord {
            device_id: context
                .string_ref(value.metadata.device_id)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string(),
            timestamp_ns: value.metadata.timestamp_ns,
            kind: InputEventRecordKind::Gamepad,
            action: value.payload.action,
            code: value.payload.backend_code,
            value: value.payload.backend_value,
            sequence: value.metadata.sequence,
        }),
        InputEventVm::InputTextEvent(value) => Ok(InputEventRecord {
            device_id: context
                .string_ref(value.metadata.device_id)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string(),
            timestamp_ns: value.metadata.timestamp_ns,
            kind: InputEventRecordKind::Text,
            action: InputEventAction::Text,
            code: 0,
            value: 0,
            sequence: value.metadata.sequence,
        }),
        InputEventVm::InputDeviceEvent(value) => Ok(InputEventRecord {
            device_id: context
                .string_ref(value.metadata.device_id)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string(),
            timestamp_ns: value.metadata.timestamp_ns,
            kind: InputEventRecordKind::Device,
            action: value.payload.action,
            code: value.payload.backend_code,
            value: value.payload.backend_value,
            sequence: value.metadata.sequence,
        }),
        InputEventVm::InputSensorEvent(value) => Ok(InputEventRecord {
            device_id: context
                .string_ref(value.metadata.device_id)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string(),
            timestamp_ns: value.metadata.timestamp_ns,
            kind: InputEventRecordKind::Sensor,
            action: value.payload.action,
            code: value.payload.backend_code,
            value: value.payload.backend_value,
            sequence: value.metadata.sequence,
        }),
        InputEventVm::InputCompositionEvent(value) => Ok(InputEventRecord {
            device_id: context
                .string_ref(value.metadata.device_id)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string(),
            timestamp_ns: value.metadata.timestamp_ns,
            kind: InputEventRecordKind::Composition,
            action: value.payload.action,
            code: 0,
            value: 0,
            sequence: value.metadata.sequence,
        }),
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
                let bytes = VmSlice::from_bytes(&mut context.write(), bytes)?;
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
                value.read_bytes(&context.read())
            }
        }
    }

    /// Return the vm context when this harness executes vm bindings.
    #[allow(clippy::mut_from_ref)]
    fn vm_context_mut(&self) -> Option<&mut vm::BindingContext<'_>> {
        // recover mutable vm context from stored raw pointer
        self.vm_context
            .map(|context| unsafe { &mut *(context as *mut vm::BindingContext<'_>) })
    }

    /// Build one backend-specific string value.
    pub(crate) fn string_value(
        &self,
        value: &str,
    ) -> HarnessValue<NativeStringRef, vm::StringHandle> {
        // build vm or native string payload based on active harness kind
        match self.vm_context_mut() {
            Some(context) => {
                let value = vm_test_string(context, value);
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
        value: InputTextGeometry,
    ) -> HarnessValue<InputTextGeometry, InputTextGeometryVm> {
        match self.vm_context_mut() {
            Some(_) => self.harness_value_vm(value),
            None => self.harness_value(value),
        }
    }

    /// Build one text-session configuration for the active harness engine.
    pub(crate) fn text_session_config(
        &self,
        target: InputWindowTarget,
        input_type: InputTextInputType,
        is_multiline: bool,
        is_secure: bool,
    ) -> HarnessValue<InputTextSessionConfig, InputTextSessionConfigVm> {
        let value = InputTextSessionConfig {
            target,
            input_type,
            is_multiline,
            is_secure,
        };

        match self.vm_context_mut() {
            Some(_) => self.harness_value_vm(value),
            None => self.harness_value(value),
        }
    }

    /// Build one text-session state for the active harness engine.
    pub(crate) fn text_session_state(
        &self,
        text: &str,
        selection: InputTextRange,
        composing: Option<InputTextRange>,
    ) -> HarnessValue<InputTextSessionState, InputTextSessionStateVm> {
        match self.vm_context_mut() {
            Some(context) => {
                let value = InputTextSessionStateVm {
                    text: vm_test_string(context, text),
                    selection,
                    composing,
                };

                self.harness_value_vm(value)
            }
            None => {
                let value = InputTextSessionState {
                    text: self.call_context.store_string(text),
                    selection,
                    composing,
                };

                self.harness_value(value)
            }
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
        value: HarnessValue<InputTextGeometry, InputTextGeometryVm>,
    ) -> InputTextGeometry {
        match value {
            HarnessValue::Native(value) => value,
            HarnessValue::Vm(value) => value,
        }
    }

    /// Decode one list result into normalized device records.
    pub(crate) fn device_records_from_value(
        &self,
        value: HarnessValue<NativeSlice<InputDeviceDescriptor>, VmSlice<InputDeviceDescriptorVm>>,
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
                let read_context = context.read();
                let values = value.read_values(&read_context)?;
                let mut records = Vec::with_capacity(values.len());
                for value in values {
                    let id = read_context
                        .string_ref(value.id)
                        .map_err(|error| RuntimeError::from(error).boxed())?
                        .as_str()
                        .to_string();
                    let name = read_context
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
            HarnessValue::Native(value) => decode_native_event(value),
            HarnessValue::Vm(value) => {
                let context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "vm context is required for vm input-event values",
                    ))
                    .boxed()
                })?;
                decode_vm_event(context, value)
            }
        }
    }

    /// Decode one monitor event wrapper into one unified monitor payload.
    pub(crate) fn monitor_event_from_value(
        &self,
        value: HarnessValue<InputMonitorEvent, InputMonitorEventVm>,
    ) -> RuntimeResult<InputMonitorEventRecord> {
        match value {
            HarnessValue::Native(value) => match value {
                InputMonitorEvent::InputMonitorChangeEvent(value) => Ok(InputMonitorEventRecord {
                    device_id: unsafe { value.metadata.device_id.as_str()? }.to_string(),
                    timestamp_ns: value.metadata.timestamp_ns,
                    kind: InputMonitorEventRecordKind::Change,
                    connected: value.metadata.connected,
                    sequence: value.metadata.sequence,
                }),
                InputMonitorEvent::InputMonitorConnectEvent(value) => Ok(InputMonitorEventRecord {
                    device_id: unsafe { value.metadata.device_id.as_str()? }.to_string(),
                    timestamp_ns: value.metadata.timestamp_ns,
                    kind: InputMonitorEventRecordKind::Connect,
                    connected: value.metadata.connected,
                    sequence: value.metadata.sequence,
                }),
                InputMonitorEvent::InputMonitorDisconnectEvent(value) => {
                    Ok(InputMonitorEventRecord {
                        device_id: unsafe { value.metadata.device_id.as_str()? }.to_string(),
                        timestamp_ns: value.metadata.timestamp_ns,
                        kind: InputMonitorEventRecordKind::Disconnect,
                        connected: value.metadata.connected,
                        sequence: value.metadata.sequence,
                    })
                }
            },
            HarnessValue::Vm(value) => {
                let context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "vm context is required for vm monitor-event values",
                    ))
                    .boxed()
                })?;
                match value {
                    InputMonitorEventVm::InputMonitorChangeEvent(value) => {
                        let device_id = context
                            .string_ref(value.metadata.device_id)
                            .map_err(|error| RuntimeError::from(error).boxed())?
                            .as_str()
                            .to_string();
                        Ok(InputMonitorEventRecord {
                            device_id,
                            timestamp_ns: value.metadata.timestamp_ns,
                            kind: InputMonitorEventRecordKind::Change,
                            connected: value.metadata.connected,
                            sequence: value.metadata.sequence,
                        })
                    }
                    InputMonitorEventVm::InputMonitorConnectEvent(value) => {
                        let device_id = context
                            .string_ref(value.metadata.device_id)
                            .map_err(|error| RuntimeError::from(error).boxed())?
                            .as_str()
                            .to_string();
                        Ok(InputMonitorEventRecord {
                            device_id,
                            timestamp_ns: value.metadata.timestamp_ns,
                            kind: InputMonitorEventRecordKind::Connect,
                            connected: value.metadata.connected,
                            sequence: value.metadata.sequence,
                        })
                    }
                    InputMonitorEventVm::InputMonitorDisconnectEvent(value) => {
                        let device_id = context
                            .string_ref(value.metadata.device_id)
                            .map_err(|error| RuntimeError::from(error).boxed())?
                            .as_str()
                            .to_string();
                        Ok(InputMonitorEventRecord {
                            device_id,
                            timestamp_ns: value.metadata.timestamp_ns,
                            kind: InputMonitorEventRecordKind::Disconnect,
                            connected: value.metadata.connected,
                            sequence: value.metadata.sequence,
                        })
                    }
                }
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
                let read_context = context.read();
                let kinds = value.kinds.read_values(&read_context)?;
                let axes = value.axes.read_values(&read_context)?;
                let mut axis_codes = Vec::with_capacity(axes.len());
                for axis in axes {
                    axis_codes.push(axis.code);
                }
                let buttons = value.buttons.read_values(&read_context)?;
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
                let data = value.data.read_bytes(&context.read())?;
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
                let read_context = context.read();
                let device_id = read_context
                    .string_ref(value.device_id)
                    .map_err(|error| RuntimeError::from(error).boxed())?
                    .as_str()
                    .to_string();
                let contacts = value.contacts.read_values(&read_context)?;
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
                let read_context = context.read();
                let axes = value.axes.read_values(&read_context)?;
                let buttons = value.buttons.read_values(&read_context)?;
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
                    timestamp_ns: value.timestamp_ns,
                    sequence: value.sequence,
                    modifiers: value.backend_modifiers,
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
                let read_context = context.read();
                let device_id = read_context
                    .string_ref(value.device_id)
                    .map_err(|error| RuntimeError::from(error).boxed())?
                    .as_str()
                    .to_string();
                let pressed_codes = value.pressed_codes.read_values(&read_context)?;
                let pressed_scan_codes = value.pressed_scan_codes.read_values(&read_context)?;

                Ok(InputKeyboardStateRecord {
                    device_id,
                    timestamp_ns: value.timestamp_ns,
                    sequence: value.sequence,
                    modifiers: value.backend_modifiers,
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
            HarnessValue::Vm(value) => {
                let context = self
                    .vm_context_mut()
                    .expect("vm context should exist for vm pointer-state values");
                let value = <InputPointerStateVm as VmAbiCodec>::into_value(value, &context.read())
                    .expect("vm pointer state should decode");

                InputPointerState::from_value(self.call_context, value)
            }
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
                value.read_values(&context.read())
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
        value: HarnessValue<NativeArray<InputSensorDescriptor>, VmArray<InputSensorDescriptorVm>>,
    ) -> RuntimeResult<Vec<InputSensorDescriptor>> {
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
                value.read_values(&context.read())
            }
        }
    }
}
