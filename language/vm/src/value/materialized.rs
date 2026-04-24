use destack_engine::MaterializedValue;

use super::{Value, ValueTag};

/// Error returned when one materialized value cannot become one VM value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DematerializeValueError {
    /// The materialized value is undefined.
    Undefined,
    /// The materialized value is one aggregate payload.
    Aggregate,
    /// The materialized value is one frame address.
    FrameAddress,
    /// The materialized value is one global address.
    GlobalAddress,
    /// The materialized value is one function handle.
    Function,
}

impl std::fmt::Display for DematerializeValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DematerializeValueError::Undefined => {
                write!(f, "undefined value cannot be dematerialized")
            }
            DematerializeValueError::Aggregate => {
                write!(f, "aggregate value cannot be dematerialized")
            }
            DematerializeValueError::FrameAddress => {
                write!(f, "frame address cannot be dematerialized")
            }
            DematerializeValueError::GlobalAddress => {
                write!(f, "global address cannot be dematerialized")
            }
            DematerializeValueError::Function => {
                write!(f, "function handle cannot be dematerialized")
            }
        }
    }
}

impl std::error::Error for DematerializeValueError {}

/// Error returned when one VM value cannot become one materialized value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MaterializeValueError {
    /// The VM value has an unknown packed tag byte.
    InvalidTag(u8),
    /// The VM value encoding did not match its tag.
    InvalidEncoding(ValueTag),
    /// The VM value is one stack pointer.
    StackPointer,
    /// The VM value is one frame pointer.
    FramePointer,
    /// The VM value is one global pointer.
    GlobalPointer,
    /// The VM value is one function pointer.
    FunctionPointer,
}

impl std::fmt::Display for MaterializeValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MaterializeValueError::InvalidTag(tag) => {
                write!(f, "value tag {tag} is invalid")
            }
            MaterializeValueError::InvalidEncoding(tag) => {
                write!(f, "value encoding is invalid for tag {tag:?}")
            }
            MaterializeValueError::StackPointer => {
                write!(f, "stack-pointer value is not materialized")
            }
            MaterializeValueError::FramePointer => {
                write!(f, "frame-pointer value is not materialized")
            }
            MaterializeValueError::GlobalPointer => {
                write!(f, "global-pointer value is not materialized")
            }
            MaterializeValueError::FunctionPointer => {
                write!(f, "function-pointer value is not materialized")
            }
        }
    }
}

impl std::error::Error for MaterializeValueError {}

impl TryFrom<&MaterializedValue> for Value {
    type Error = DematerializeValueError;

    fn try_from(value: &MaterializedValue) -> Result<Self, Self::Error> {
        match value {
            MaterializedValue::Void => Ok(Value::VOID),
            MaterializedValue::Bool(value) => Ok(Value::bool(*value)),
            MaterializedValue::Int { value, width } => Ok(Value::int(*value, *width)),
            MaterializedValue::UInt { value, width } => Ok(Value::uint(*value, *width)),
            MaterializedValue::Float32 { bits } => Ok(Value::float32(f32::from_bits(*bits))),
            MaterializedValue::Float64 { bits } => Ok(Value::float64(f64::from_bits(*bits))),
            MaterializedValue::Char(value) => Ok(Value::char(*value)),
            MaterializedValue::HeapReference(reference) => Ok(Value::heap_reference(*reference)),
            MaterializedValue::SharedHeapReference(reference) => {
                Ok(Value::shared_heap_reference(*reference))
            }
            MaterializedValue::RawPointer(pointer) => Ok(Value::raw_pointer(*pointer)),
            MaterializedValue::SharedRawPointer(pointer) => Ok(Value::shared_raw_pointer(*pointer)),
            MaterializedValue::Undefined => Err(DematerializeValueError::Undefined),
            MaterializedValue::Aggregate { .. } => Err(DematerializeValueError::Aggregate),
            MaterializedValue::FrameAddress(_) => Err(DematerializeValueError::FrameAddress),
            MaterializedValue::GlobalAddress(_) => Err(DematerializeValueError::GlobalAddress),
            MaterializedValue::Function(_) => Err(DematerializeValueError::Function),
        }
    }
}

impl TryFrom<Value> for MaterializedValue {
    type Error = MaterializeValueError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let Some(tag) = value.checked_tag() else {
            return Err(MaterializeValueError::InvalidTag(value.tag_byte()));
        };

        match tag {
            ValueTag::Void => Ok(MaterializedValue::Void),
            ValueTag::Bool => {
                let Some(value) = value.as_bool() else {
                    return Err(MaterializeValueError::InvalidEncoding(ValueTag::Bool));
                };

                Ok(MaterializedValue::Bool(value))
            }
            ValueTag::Int => {
                let Some((value, width)) = value.as_int_with_width() else {
                    return Err(MaterializeValueError::InvalidEncoding(ValueTag::Int));
                };

                Ok(MaterializedValue::Int { value, width })
            }
            ValueTag::UInt => {
                let Some((value, width)) = value.as_uint_with_width() else {
                    return Err(MaterializeValueError::InvalidEncoding(ValueTag::UInt));
                };

                Ok(MaterializedValue::UInt { value, width })
            }
            ValueTag::Float32 => {
                let Some(value) = value.as_float32() else {
                    return Err(MaterializeValueError::InvalidEncoding(ValueTag::Float32));
                };

                Ok(MaterializedValue::Float32 {
                    bits: value.to_bits(),
                })
            }
            ValueTag::Float64 => {
                let Some(value) = value.as_float64() else {
                    return Err(MaterializeValueError::InvalidEncoding(ValueTag::Float64));
                };

                Ok(MaterializedValue::Float64 {
                    bits: value.to_bits(),
                })
            }
            ValueTag::Char => {
                let Some(value) = value.as_char() else {
                    return Err(MaterializeValueError::InvalidEncoding(ValueTag::Char));
                };

                Ok(MaterializedValue::Char(value))
            }
            ValueTag::HeapReference => {
                let Some(value) = value.as_heap_reference() else {
                    return Err(MaterializeValueError::InvalidEncoding(
                        ValueTag::HeapReference,
                    ));
                };

                Ok(MaterializedValue::HeapReference(value))
            }
            ValueTag::SharedHeapReference => {
                let Some(value) = value.as_shared_heap_reference() else {
                    return Err(MaterializeValueError::InvalidEncoding(
                        ValueTag::SharedHeapReference,
                    ));
                };

                Ok(MaterializedValue::SharedHeapReference(value))
            }
            ValueTag::RawPointer => {
                let Some(value) = value.as_raw_pointer() else {
                    return Err(MaterializeValueError::InvalidEncoding(ValueTag::RawPointer));
                };

                Ok(MaterializedValue::RawPointer(value))
            }
            ValueTag::SharedRawPointer => {
                let Some(value) = value.as_shared_raw_pointer() else {
                    return Err(MaterializeValueError::InvalidEncoding(
                        ValueTag::SharedRawPointer,
                    ));
                };

                Ok(MaterializedValue::SharedRawPointer(value))
            }
            ValueTag::StackPointer => Err(MaterializeValueError::StackPointer),
            ValueTag::FramePointer => Err(MaterializeValueError::FramePointer),
            ValueTag::GlobalPointer => Err(MaterializeValueError::GlobalPointer),
            ValueTag::FunctionPointer => Err(MaterializeValueError::FunctionPointer),
        }
    }
}
