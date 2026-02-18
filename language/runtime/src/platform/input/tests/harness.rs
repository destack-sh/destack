use destack_vm as vm;

use super::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::{
    InputDeviceInfo, InputDeviceInfoVm, InputDeviceKind, InputEvent, InputEventAction,
    InputEventKind, InputEventVm,
};
use crate::platform::{NativeSlice, NativeStringRef, PlatformError, VmSlice};

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
    pub supports_grab: bool,
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

impl<'call> InputHarnessContext<'call> {
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
                        supports_grab: value.supports_grab,
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
                        supports_grab: value.supports_grab,
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
                Ok(InputEventRecord {
                    device_id,
                    kind: value.kind,
                    action: value.action,
                    code: value.code,
                    value: value.value,
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
                Ok(InputEventRecord {
                    device_id,
                    kind: value.kind,
                    action: value.action,
                    code: value.code,
                    value: value.value,
                    sequence: value.sequence,
                })
            }
        }
    }
}
