use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformError;

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
        context: &vm::ExternalReadContext<'_, '_>,
        value: vm::Value,
    ) -> RuntimeResult<Self>;

    /// Decode one value from one VM value view with context access.
    fn decode_value_ref_with_context(
        _context: &vm::ExternalReadContext<'_, '_>,
        _value_ref: &vm::VmValueRef<'_, '_>,
    ) -> RuntimeResult<Self> {
        Err(RuntimeError::from(PlatformError::invalid_argument_type(
            "value",
            "aggregate value",
        ))
        .boxed())
    }

    /// Decode one semantic field from one VM value view.
    fn decode_field_with_context(
        context: &vm::ExternalReadContext<'_, '_>,
        value_ref: &vm::VmValueRef<'_, '_>,
        index: u32,
    ) -> RuntimeResult<Self> {
        // prefer nested storage views when the field has one
        if let Ok(field_ref) = value_ref.field_ref(index) {
            return Self::decode_value_ref_with_context(context, &field_ref);
        }

        let value = value_ref
            .field_value(index)
            .map_err(Box::<RuntimeError>::from)?;

        Self::decode_with_context(context, value)
    }

    /// Encode a value into a VM slot with context access.
    fn encode_with_context(
        self,
        context: &mut vm::ExternalWriteContext<'_, '_>,
    ) -> RuntimeResult<vm::Value>;
}

/// Storage strategy for VM collection elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmCollectionStorage {
    /// Store elements as packed VM values.
    Values,
    /// Store elements as raw bytes.
    Bytes,
}

/// Collection element codec for VM slices and arrays.
pub trait VmCollectionElement: VmAggregateCodec {
    /// The storage strategy for this element type inside VM collections.
    const STORAGE: VmCollectionStorage = VmCollectionStorage::Values;
}

impl<T: VmValueCodec> VmAggregateCodec for T {
    fn decode_with_context(
        _context: &vm::ExternalReadContext<'_, '_>,
        value: vm::Value,
    ) -> RuntimeResult<Self> {
        T::decode(value)
    }

    fn encode_with_context(
        self,
        _context: &mut vm::ExternalWriteContext<'_, '_>,
    ) -> RuntimeResult<vm::Value> {
        Ok(T::encode(self))
    }
}

impl<T: VmAggregateCodec> VmAggregateCodec for Option<T> {
    fn decode_with_context(
        context: &vm::ExternalReadContext<'_, '_>,
        value: vm::Value,
    ) -> RuntimeResult<Self> {
        if value.tag() == vm::ValueTag::Void {
            return Ok(None);
        }

        let decoded = T::decode_with_context(context, value)?;
        Ok(Some(decoded))
    }

    fn decode_value_ref_with_context(
        context: &vm::ExternalReadContext<'_, '_>,
        value_ref: &vm::VmValueRef<'_, '_>,
    ) -> RuntimeResult<Self> {
        Ok(Some(T::decode_value_ref_with_context(context, value_ref)?))
    }

    fn encode_with_context(
        self,
        context: &mut vm::ExternalWriteContext<'_, '_>,
    ) -> RuntimeResult<vm::Value> {
        match self {
            Some(value) => T::encode_with_context(value, context),
            None => Ok(vm::Value::VOID),
        }
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

impl VmValueCodec for vm::RawPointer {
    fn decode(value: vm::Value) -> RuntimeResult<Self> {
        value.as_raw_pointer().ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_type("value", "raw pointer")).boxed()
        })
    }

    fn encode(self) -> vm::Value {
        vm::Value::raw_pointer(self)
    }
}

impl VmAggregateCodec for vm::StringHandle {
    fn decode_with_context(
        context: &vm::ExternalReadContext<'_, '_>,
        value: vm::Value,
    ) -> RuntimeResult<Self> {
        context
            .string_handle_from_value(value)
            .map_err(Box::<RuntimeError>::from)
    }

    fn encode_with_context(
        self,
        _context: &mut vm::ExternalWriteContext<'_, '_>,
    ) -> RuntimeResult<vm::Value> {
        Ok(self.value())
    }
}

macro_rules! value_collection_elements {
    ($($type:ty),+ $(,)?) => {
        $(
            impl VmCollectionElement for $type {}
        )+
    };
}

value_collection_elements!(bool, i8, i16, i32, i64, u16, u32, u64, f32, f64,);

impl VmCollectionElement for vm::StringHandle {}

impl VmCollectionElement for u8 {
    const STORAGE: VmCollectionStorage = VmCollectionStorage::Bytes;
}

impl<T: VmCollectionElement> VmCollectionElement for Option<T> {}
