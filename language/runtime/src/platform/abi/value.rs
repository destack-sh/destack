use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;

/// Codec for VM values inside replay helpers.
pub trait VmValueCodec: Copy {
    /// Decode a value from a VM slot.
    fn decode(value: vm::Value) -> RuntimeResult<Self>;

    /// Encode a value into a VM slot.
    fn encode(self) -> vm::Value;
}

/// Context-aware codec for VM values that can require heap access.
pub trait VmAggregateCodec: Copy {
    /// Decode a value from a VM slot with context access.
    fn decode_with_context(
        context: &vm::ExternalCallContext<'_>,
        value: vm::Value,
    ) -> RuntimeResult<Self>;

    /// Encode a value into a VM slot with context access.
    fn encode_with_context(
        self,
        context: &mut vm::ExternalCallContext<'_>,
    ) -> RuntimeResult<vm::Value>;
}

impl<T: VmValueCodec> VmAggregateCodec for T {
    fn decode_with_context(
        _context: &vm::ExternalCallContext<'_>,
        value: vm::Value,
    ) -> RuntimeResult<Self> {
        T::decode(value)
    }

    fn encode_with_context(
        self,
        _context: &mut vm::ExternalCallContext<'_>,
    ) -> RuntimeResult<vm::Value> {
        Ok(T::encode(self))
    }
}

/// Decode an integer value with an expected width.
fn decode_int(value: vm::Value, bits: u8) -> RuntimeResult<i64> {
    let (raw, width) = value.as_int_with_width().ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_type("value", "int")).boxed()
    })?;
    if width != bits {
        return Err(
            RuntimeError::from(PlatformError::invalid_argument_type("value", "int")).boxed(),
        );
    }

    Ok(raw)
}

/// Decode an unsigned integer value with an expected width.
fn decode_uint(value: vm::Value, bits: u8) -> RuntimeResult<u64> {
    let (raw, width) = value.as_uint_with_width().ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_type("value", "uint")).boxed()
    })?;
    if width != bits {
        return Err(
            RuntimeError::from(PlatformError::invalid_argument_type("value", "uint")).boxed(),
        );
    }

    Ok(raw)
}

impl VmValueCodec for bool {
    fn decode(value: vm::Value) -> RuntimeResult<Self> {
        value.as_bool().ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_type("value", "bool")).boxed()
        })
    }

    fn encode(self) -> vm::Value {
        vm::Value::bool(self)
    }
}

impl VmValueCodec for i8 {
    fn decode(value: vm::Value) -> RuntimeResult<Self> {
        Ok(decode_int(value, 8)? as i8)
    }

    fn encode(self) -> vm::Value {
        vm::Value::int(self as i64, 8)
    }
}

impl VmValueCodec for i16 {
    fn decode(value: vm::Value) -> RuntimeResult<Self> {
        Ok(decode_int(value, 16)? as i16)
    }

    fn encode(self) -> vm::Value {
        vm::Value::int(self as i64, 16)
    }
}

impl VmValueCodec for i32 {
    fn decode(value: vm::Value) -> RuntimeResult<Self> {
        Ok(decode_int(value, 32)? as i32)
    }

    fn encode(self) -> vm::Value {
        vm::Value::int(self as i64, 32)
    }
}

impl VmValueCodec for i64 {
    fn decode(value: vm::Value) -> RuntimeResult<Self> {
        decode_int(value, 64)
    }

    fn encode(self) -> vm::Value {
        vm::Value::int(self, 64)
    }
}

impl VmValueCodec for u8 {
    fn decode(value: vm::Value) -> RuntimeResult<Self> {
        Ok(decode_uint(value, 8)? as u8)
    }

    fn encode(self) -> vm::Value {
        vm::Value::uint(self as u64, 8)
    }
}

impl VmValueCodec for u16 {
    fn decode(value: vm::Value) -> RuntimeResult<Self> {
        Ok(decode_uint(value, 16)? as u16)
    }

    fn encode(self) -> vm::Value {
        vm::Value::uint(self as u64, 16)
    }
}

impl VmValueCodec for u32 {
    fn decode(value: vm::Value) -> RuntimeResult<Self> {
        Ok(decode_uint(value, 32)? as u32)
    }

    fn encode(self) -> vm::Value {
        vm::Value::uint(self as u64, 32)
    }
}

impl VmValueCodec for u64 {
    fn decode(value: vm::Value) -> RuntimeResult<Self> {
        decode_uint(value, 64)
    }

    fn encode(self) -> vm::Value {
        vm::Value::uint(self, 64)
    }
}

impl VmValueCodec for f32 {
    fn decode(value: vm::Value) -> RuntimeResult<Self> {
        value.as_float32().ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_type("value", "float32")).boxed()
        })
    }

    fn encode(self) -> vm::Value {
        vm::Value::float32(self)
    }
}

impl VmValueCodec for f64 {
    fn decode(value: vm::Value) -> RuntimeResult<Self> {
        value.as_float64().ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_type("value", "float64")).boxed()
        })
    }

    fn encode(self) -> vm::Value {
        vm::Value::float64(self)
    }
}

impl VmValueCodec for vm::StringHandle {
    fn decode(value: vm::Value) -> RuntimeResult<Self> {
        if value.tag() != vm::ValueTag::String {
            return Err(RuntimeError::from(PlatformError::invalid_argument_type(
                "value", "string",
            ))
            .boxed());
        }

        Ok(vm::StringHandle::new(value))
    }

    fn encode(self) -> vm::Value {
        self.value()
    }
}
