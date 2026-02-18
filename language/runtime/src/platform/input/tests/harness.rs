use destack_vm as vm;

use super::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::{
    InputDeviceInfo, InputDeviceInfoVm, InputDeviceKind, InputEvent, InputEventVm,
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
    /// Connected state at enumeration time.
    pub connected: bool,
}

impl<'call> InputHarnessContext<'call> {
    /// Return the vm context when this harness executes vm bindings.
    #[allow(clippy::mut_from_ref)]
    fn vm_context_mut(&self) -> Option<&mut vm::ExternalCallContext<'_>> {
        self.vm_context
            .map(|context| unsafe { &mut *(context as *mut vm::ExternalCallContext<'_>) })
    }

    /// Build one backend-specific string value.
    pub(crate) fn string_value(
        &self,
        value: &str,
    ) -> HarnessValue<NativeStringRef, vm::StringHandle> {
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
                let values = unsafe { value.as_slice()? };
                let mut records = Vec::with_capacity(values.len());
                for value in values {
                    records.push(InputDeviceRecord {
                        id: unsafe { value.id.as_str()? }.to_string(),
                        name: unsafe { value.name.as_str()? }.to_string(),
                        kind: value.kind,
                        vendor_id: value.vendor_id,
                        product_id: value.product_id,
                        connected: value.connected,
                    });
                }

                Ok(records)
            }
            HarnessValue::Vm(value) => {
                let context = self.vm_context_mut().ok_or_else(|| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "context",
                        "vm context is required for vm input-device values",
                    ))
                    .boxed()
                })?;
                decode_vm_device_records(context, value)
            }
        }
    }

    /// Decode one event wrapper into one unified event payload.
    pub(crate) fn event_from_value(
        &self,
        value: HarnessValue<InputEvent, InputEventVm>,
    ) -> InputEvent {
        match value {
            HarnessValue::Native(value) => value,
            HarnessValue::Vm(value) => value,
        }
    }
}

/// Decode one VM input-device slice into normalized harness records.
fn decode_vm_device_records(
    context: &mut vm::ExternalCallContext<'_>,
    values: VmSlice<InputDeviceInfoVm>,
) -> RuntimeResult<Vec<InputDeviceRecord>> {
    // decode vm slice payload
    let values = values.raw_values(context)?;
    let mut records = Vec::with_capacity(values.len());

    for value in values {
        // validate aggregate shape
        if value.tag() != vm::ValueTag::Aggregate {
            return Err(RuntimeError::from(PlatformError::invalid_argument_type(
                "device",
                "InputDeviceInfo",
            ))
            .boxed());
        }

        // decode aggregate fields
        let slots = context
            .aggregate_slots(value)
            .map_err(|error| RuntimeError::from(error).boxed())?;
        if slots.len() != 6 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "device",
                "expected InputDeviceInfo aggregate with 6 fields",
            ))
            .boxed());
        }

        let id_handle = vm::StringHandle::new(slots[0]);
        let name_handle = vm::StringHandle::new(slots[1]);
        let id = context
            .string_ref(id_handle)
            .map_err(|error| RuntimeError::from(error).boxed())?
            .as_str()
            .to_string();
        let name = context
            .string_ref(name_handle)
            .map_err(|error| RuntimeError::from(error).boxed())?
            .as_str()
            .to_string();

        // decode device kind
        let (kind_raw, kind_width) = slots[2].as_uint_with_width().ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_type("device.kind", "uint8")).boxed()
        })?;
        if kind_width != 8 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_type(
                "device.kind",
                "uint8",
            ))
            .boxed());
        }
        let kind = match kind_raw as u8 {
            1 => InputDeviceKind::Keyboard,
            2 => InputDeviceKind::Mouse,
            3 => InputDeviceKind::Touch,
            4 => InputDeviceKind::Gamepad,
            5 => InputDeviceKind::Pen,
            255 => InputDeviceKind::Raw,
            _ => {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "device.kind",
                    "unknown InputDeviceKind value",
                ))
                .boxed());
            }
        };

        // decode vendor and product ids
        let (vendor_id_raw, vendor_id_width) = slots[3].as_uint_with_width().ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_type(
                "device.vendorId",
                "uint16",
            ))
            .boxed()
        })?;
        if vendor_id_width != 16 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_type(
                "device.vendorId",
                "uint16",
            ))
            .boxed());
        }

        let (product_id_raw, product_id_width) =
            slots[4].as_uint_with_width().ok_or_else(|| {
                RuntimeError::from(PlatformError::invalid_argument_type(
                    "device.productId",
                    "uint16",
                ))
                .boxed()
            })?;
        if product_id_width != 16 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_type(
                "device.productId",
                "uint16",
            ))
            .boxed());
        }

        // decode connected flag
        let connected = slots[5].as_bool().ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_type(
                "device.connected",
                "boolean",
            ))
            .boxed()
        })?;

        // normalize decoded record
        records.push(InputDeviceRecord {
            id,
            name,
            kind,
            vendor_id: vendor_id_raw as u16,
            product_id: product_id_raw as u16,
            connected,
        });
    }

    Ok(records)
}
