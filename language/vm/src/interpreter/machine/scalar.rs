use super::prelude::*;

/// Reduction operators for vector or tensor reductions.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ReduceOperator {
    /// Add values.
    Add,
    /// Multiply values.
    Multiply,
    /// Select the minimum value.
    Min,
    /// Select the maximum value.
    Max,
    /// Apply bitwise or logical and.
    And,
    /// Apply bitwise or logical or.
    Or,
    /// Apply bitwise or logical xor.
    Xor,
}

impl From<mir::VectorReduceOperator> for ReduceOperator {
    fn from(value: mir::VectorReduceOperator) -> Self {
        match value {
            mir::VectorReduceOperator::Add => ReduceOperator::Add,
            mir::VectorReduceOperator::Multiply => ReduceOperator::Multiply,
            mir::VectorReduceOperator::Min => ReduceOperator::Min,
            mir::VectorReduceOperator::Max => ReduceOperator::Max,
            mir::VectorReduceOperator::And => ReduceOperator::And,
            mir::VectorReduceOperator::Or => ReduceOperator::Or,
            mir::VectorReduceOperator::Xor => ReduceOperator::Xor,
        }
    }
}

impl From<mir::TensorReduceOperator> for ReduceOperator {
    fn from(value: mir::TensorReduceOperator) -> Self {
        match value {
            mir::TensorReduceOperator::Add => ReduceOperator::Add,
            mir::TensorReduceOperator::Multiply => ReduceOperator::Multiply,
            mir::TensorReduceOperator::Min => ReduceOperator::Min,
            mir::TensorReduceOperator::Max => ReduceOperator::Max,
            mir::TensorReduceOperator::And => ReduceOperator::And,
            mir::TensorReduceOperator::Or => ReduceOperator::Or,
            mir::TensorReduceOperator::Xor => ReduceOperator::Xor,
        }
    }
}

/// Scalar type information for numeric conversions.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ScalarTypeInfo {
    /// Signed or unsigned integers with a bit width.
    Int {
        /// The bit width.
        width: u16,
        /// Whether the integer is signed.
        is_signed: bool,
    },
    /// Floating-point values with a bit width.
    Float {
        /// The bit width.
        width: u16,
    },
    /// Boolean values.
    Bool,
}

/// Conversion modes for vector or tensor element conversions.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ScalarConvertMode {
    /// Require an exact conversion without rounding or saturation.
    Exact,
    /// Round to nearest, ties to even.
    RoundTiesEven,
    /// Round toward zero.
    RoundTowardZero,
    /// Round toward negative infinity.
    RoundFloor,
    /// Round toward positive infinity.
    RoundCeil,
    /// Clamp overflow to the destination range.
    Saturate,
}

impl From<mir::VectorConvertMode> for ScalarConvertMode {
    fn from(value: mir::VectorConvertMode) -> Self {
        match value {
            mir::VectorConvertMode::Exact => ScalarConvertMode::Exact,
            mir::VectorConvertMode::RoundTiesEven => ScalarConvertMode::RoundTiesEven,
            mir::VectorConvertMode::RoundTowardZero => ScalarConvertMode::RoundTowardZero,
            mir::VectorConvertMode::RoundFloor => ScalarConvertMode::RoundFloor,
            mir::VectorConvertMode::RoundCeil => ScalarConvertMode::RoundCeil,
            mir::VectorConvertMode::Saturate => ScalarConvertMode::Saturate,
        }
    }
}

impl From<mir::TensorConvertMode> for ScalarConvertMode {
    fn from(value: mir::TensorConvertMode) -> Self {
        match value {
            mir::TensorConvertMode::Exact => ScalarConvertMode::Exact,
            mir::TensorConvertMode::RoundTiesEven => ScalarConvertMode::RoundTiesEven,
            mir::TensorConvertMode::RoundTowardZero => ScalarConvertMode::RoundTowardZero,
            mir::TensorConvertMode::RoundFloor => ScalarConvertMode::RoundFloor,
            mir::TensorConvertMode::RoundCeil => ScalarConvertMode::RoundCeil,
            mir::TensorConvertMode::Saturate => ScalarConvertMode::Saturate,
        }
    }
}

/// Resolve scalar type information from a MIR type.
pub(crate) fn scalar_type_info(
    tree: &mir::NodeTree,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<ScalarTypeInfo, Error> {
    // resolve scalar types
    match tree.get(ty) {
        mir::Type::Int { width, is_signed } => Ok(ScalarTypeInfo::Int {
            width: *width,
            is_signed: *is_signed,
        }),
        mir::Type::Float { width } => Ok(ScalarTypeInfo::Float { width: *width }),
        mir::Type::Boolean => Ok(ScalarTypeInfo::Bool),
        _ => Err(Error::TypeMismatch {
            expected: "scalar type".to_string(),
            actual: format!("{ty:?}"),
        }),
    }
}

/// Convert a scalar value between numeric types.
pub(crate) fn convert_scalar_value(
    value: Value,
    source: ScalarTypeInfo,
    dest: ScalarTypeInfo,
    mode: ScalarConvertMode,
) -> Result<Value, Error> {
    // short-circuit identical scalar kinds
    if matches!((source, dest), (ScalarTypeInfo::Bool, ScalarTypeInfo::Bool)) {
        return Ok(value);
    }

    // reject boolean conversions for now
    if matches!(source, ScalarTypeInfo::Bool) || matches!(dest, ScalarTypeInfo::Bool) {
        return Err(Error::TypeMismatch {
            expected: "numeric conversion".to_string(),
            actual: format!("{value:?}"),
        });
    }

    // convert between numeric kinds
    match (source, dest) {
        (
            ScalarTypeInfo::Int { width, is_signed },
            ScalarTypeInfo::Int {
                width: dest_width,
                is_signed: dest_signed,
            },
        ) => convert_int_to_int(value, width, is_signed, dest_width, dest_signed, mode),
        (ScalarTypeInfo::Int { width, is_signed }, ScalarTypeInfo::Float { width: dest_width }) => {
            convert_int_to_float(value, width, is_signed, dest_width, mode)
        }
        (
            ScalarTypeInfo::Float { width },
            ScalarTypeInfo::Int {
                width: dest_width,
                is_signed,
            },
        ) => convert_float_to_int(value, width, dest_width, is_signed, mode),
        (ScalarTypeInfo::Float { width }, ScalarTypeInfo::Float { width: dest_width }) => {
            convert_float_to_float(value, width, dest_width, mode)
        }
        _ => Err(Error::TypeMismatch {
            expected: "numeric conversion".to_string(),
            actual: format!("{value:?}"),
        }),
    }
}

/// Convert an integer value to another integer type.
pub(crate) fn convert_int_to_int(
    value: Value,
    source_width: u16,
    source_signed: bool,
    dest_width: u16,
    dest_signed: bool,
    mode: ScalarConvertMode,
) -> Result<Value, Error> {
    // resolve width information
    let source_width_u8 = width_u8(source_width)?;
    let dest_width_u8 = width_u8(dest_width)?;

    // resolve source value
    let source_value = if source_signed {
        let (value, width) = value
            .as_int_with_width()
            .ok_or_else(|| Error::TypeMismatch {
                expected: "signed integer".to_string(),
                actual: format!("{value:?}"),
            })?;
        if width != source_width_u8 {
            return Err(Error::TypeMismatch {
                expected: format!("int{source_width_u8}"),
                actual: format!("int{width}"),
            });
        }

        IntValue::Signed(value)
    } else {
        let (value, width) = value
            .as_uint_with_width()
            .ok_or_else(|| Error::TypeMismatch {
                expected: "unsigned integer".to_string(),
                actual: format!("{value:?}"),
            })?;
        if width != source_width_u8 {
            return Err(Error::TypeMismatch {
                expected: format!("uint{source_width_u8}"),
                actual: format!("uint{width}"),
            });
        }

        IntValue::Unsigned(value)
    };

    // convert to destination representation
    if dest_signed {
        // convert into the signed destination domain
        let (min, max) = signed_bounds(dest_width);
        let value = match source_value {
            IntValue::Signed(value) => value,
            IntValue::Unsigned(value) => {
                let value = i128::from(value);
                if value > i128::from(i64::MAX) {
                    let clamped = clamp_or_error_signed(value, min, max, mode)?;
                    return Ok(Value::int(clamped, dest_width_u8));
                }

                value as i64
            }
        };
        let clamped = clamp_or_error_signed(i128::from(value), min, max, mode)?;

        Ok(Value::int(clamped, dest_width_u8))
    } else {
        // convert into the unsigned destination domain
        let max = unsigned_max(dest_width);
        let value = match source_value {
            IntValue::Signed(value) => {
                if value < 0 {
                    let clamped = clamp_or_error_unsigned(-1, max, mode)?;
                    return Ok(Value::uint(clamped, dest_width_u8));
                }

                value as i128
            }
            IntValue::Unsigned(value) => i128::from(value),
        };
        let clamped = clamp_or_error_unsigned(value, max, mode)?;

        Ok(Value::uint(clamped, dest_width_u8))
    }
}

/// Convert an integer value to a float.
pub(crate) fn convert_int_to_float(
    value: Value,
    source_width: u16,
    source_signed: bool,
    dest_width: u16,
    mode: ScalarConvertMode,
) -> Result<Value, Error> {
    // resolve width information
    let source_width_u8 = width_u8(source_width)?;

    // resolve integer value
    let int_value = if source_signed {
        let (value, width) = value
            .as_int_with_width()
            .ok_or_else(|| Error::TypeMismatch {
                expected: "signed integer".to_string(),
                actual: format!("{value:?}"),
            })?;
        if width != source_width_u8 {
            return Err(Error::TypeMismatch {
                expected: format!("int{source_width_u8}"),
                actual: format!("int{width}"),
            });
        }

        IntValue::Signed(value)
    } else {
        let (value, width) = value
            .as_uint_with_width()
            .ok_or_else(|| Error::TypeMismatch {
                expected: "unsigned integer".to_string(),
                actual: format!("{value:?}"),
            })?;
        if width != source_width_u8 {
            return Err(Error::TypeMismatch {
                expected: format!("uint{source_width_u8}"),
                actual: format!("uint{width}"),
            });
        }

        IntValue::Unsigned(value)
    };

    // convert to float
    let float_value = match int_value {
        IntValue::Signed(value) => value as f64,
        IntValue::Unsigned(value) => value as f64,
    };

    // enforce exactness if requested
    if matches!(mode, ScalarConvertMode::Exact) {
        let round_trip = if dest_width == 32 {
            (float_value as f32) as f64
        } else {
            float_value
        };
        if round_trip != float_value {
            return Err(Error::TypeMismatch {
                expected: "exact integer to float conversion".to_string(),
                actual: float_value.to_string(),
            });
        }
    }

    // emit destination float
    if dest_width == 32 {
        Ok(Value::float32(float_value as f32))
    } else {
        Ok(Value::float64(float_value))
    }
}

/// Convert a float value to an integer.
pub(crate) fn convert_float_to_int(
    value: Value,
    source_width: u16,
    dest_width: u16,
    dest_signed: bool,
    mode: ScalarConvertMode,
) -> Result<Value, Error> {
    // resolve width information
    let dest_width_u8 = width_u8(dest_width)?;

    // resolve float value
    let float_value = match source_width {
        32 => value.as_float32().map(|v| v as f64),
        64 => value.as_float64(),
        _ => None,
    }
    .ok_or_else(|| Error::TypeMismatch {
        expected: "float".to_string(),
        actual: format!("{value:?}"),
    })?;

    // require finite values for exact conversions
    if !float_value.is_finite() && !matches!(mode, ScalarConvertMode::Saturate) {
        return Err(Error::TypeMismatch {
            expected: "finite float".to_string(),
            actual: float_value.to_string(),
        });
    }

    // round according to the requested mode
    let rounded = match mode {
        ScalarConvertMode::Exact => {
            if float_value.fract() != 0.0 {
                return Err(Error::TypeMismatch {
                    expected: "integral float".to_string(),
                    actual: float_value.to_string(),
                });
            }

            float_value
        }
        ScalarConvertMode::RoundTiesEven => float_value.round_ties_even(),
        ScalarConvertMode::RoundTowardZero => float_value.trunc(),
        ScalarConvertMode::RoundFloor => float_value.floor(),
        ScalarConvertMode::RoundCeil => float_value.ceil(),
        ScalarConvertMode::Saturate => float_value.trunc(),
    };

    // convert to destination integer
    if dest_signed {
        // clamp or reject in the signed destination domain
        let (min, max) = signed_bounds(dest_width);
        let value = if rounded.is_finite() {
            rounded as i128
        } else {
            0
        };
        let clamped = clamp_or_error_signed(value, min, max, mode)?;

        Ok(Value::int(clamped, dest_width_u8))
    } else {
        // clamp or reject in the unsigned destination domain
        let max = unsigned_max(dest_width);
        let value = if rounded.is_finite() {
            rounded as i128
        } else {
            0
        };
        let clamped = clamp_or_error_unsigned(value, max, mode)?;

        Ok(Value::uint(clamped, dest_width_u8))
    }
}

/// Convert a float value to another float type.
pub(crate) fn convert_float_to_float(
    value: Value,
    source_width: u16,
    dest_width: u16,
    mode: ScalarConvertMode,
) -> Result<Value, Error> {
    // resolve float value
    let float_value = match source_width {
        32 => value.as_float32().map(|v| v as f64),
        64 => value.as_float64(),
        _ => None,
    }
    .ok_or_else(|| Error::TypeMismatch {
        expected: "float".to_string(),
        actual: format!("{value:?}"),
    })?;

    // apply rounding mode for narrowing conversions
    let rounded = match mode {
        ScalarConvertMode::Exact => float_value,
        ScalarConvertMode::RoundTiesEven => float_value.round_ties_even(),
        ScalarConvertMode::RoundTowardZero => float_value.trunc(),
        ScalarConvertMode::RoundFloor => float_value.floor(),
        ScalarConvertMode::RoundCeil => float_value.ceil(),
        ScalarConvertMode::Saturate => float_value,
    };

    // enforce exactness if requested
    if matches!(mode, ScalarConvertMode::Exact) && dest_width == 32 {
        let round_trip = (rounded as f32) as f64;
        if round_trip != rounded {
            return Err(Error::TypeMismatch {
                expected: "exact float conversion".to_string(),
                actual: rounded.to_string(),
            });
        }
    }

    // emit destination float
    if dest_width == 32 {
        Ok(Value::float32(rounded as f32))
    } else {
        Ok(Value::float64(rounded))
    }
}

/// Resolved integer values for conversions.
enum IntValue {
    /// A signed integer value.
    Signed(i64),
    /// An unsigned integer value.
    Unsigned(u64),
}

/// Compute signed integer bounds for a width.
pub(crate) fn signed_bounds(width: u16) -> (i64, i64) {
    // handle full-width values
    if width >= 64 {
        return (i64::MIN, i64::MAX);
    }

    // compute min and max
    let shift = (width as u32).saturating_sub(1);
    let max = (1i64 << shift) - 1;
    let min = -(1i64 << shift);

    (min, max)
}

/// Convert a bit width to a u8, erroring on overflow.
pub(crate) fn width_u8(width: u16) -> Result<u8, Error> {
    // convert width and reject oversized values
    u8::try_from(width).map_err(|_| Error::TypeMismatch {
        expected: "width <= 255".to_string(),
        actual: width.to_string(),
    })
}

/// Compute the unsigned max for a width.
pub(crate) fn unsigned_max(width: u16) -> u64 {
    // handle full-width values
    if width >= 64 {
        return u64::MAX;
    }

    // compute max
    (1u64 << width) - 1
}

/// Apply saturating or exact behavior for signed conversions.
pub(crate) fn clamp_or_error_signed(
    value: i128,
    min: i64,
    max: i64,
    mode: ScalarConvertMode,
) -> Result<i64, Error> {
    // accept values in range
    if value >= i128::from(min) && value <= i128::from(max) {
        return Ok(value as i64);
    }

    // saturate or error
    if matches!(mode, ScalarConvertMode::Saturate) {
        return Ok(if value < i128::from(min) { min } else { max });
    }

    Err(Error::TypeMismatch {
        expected: format!("signed range {min}..={max}"),
        actual: value.to_string(),
    })
}

/// Apply saturating or exact behavior for unsigned conversions.
pub(crate) fn clamp_or_error_unsigned(
    value: i128,
    max: u64,
    mode: ScalarConvertMode,
) -> Result<u64, Error> {
    // accept values in range
    if value >= 0 && value <= i128::from(max) {
        return Ok(value as u64);
    }

    // saturate or error
    if matches!(mode, ScalarConvertMode::Saturate) {
        return Ok(if value < 0 { 0 } else { max });
    }

    Err(Error::TypeMismatch {
        expected: format!("unsigned range 0..={max}"),
        actual: value.to_string(),
    })
}

/// Apply a reduction operator to two values.
pub(crate) fn apply_reduce_operator(
    op: ReduceOperator,
    a: Value,
    b: Value,
) -> Result<Value, Error> {
    let (a_tag, b_tag) = (a.tag(), b.tag());
    match op {
        ReduceOperator::Add => match (a_tag, b_tag) {
            (ValueTag::Int, ValueTag::Int) => {
                let av = a.raw_data() as i64;
                let bv = b.raw_data() as i64;
                Ok(Value::int(av.wrapping_add(bv), a.width()))
            }
            (ValueTag::UInt, ValueTag::UInt) => {
                let av = a.raw_data();
                let bv = b.raw_data();
                Ok(Value::uint(av.wrapping_add(bv), a.width()))
            }
            (ValueTag::Float32, ValueTag::Float32) => {
                let av = f32::from_bits(a.raw_data() as u32);
                let bv = f32::from_bits(b.raw_data() as u32);
                Ok(Value::float32(av + bv))
            }
            (ValueTag::Float64, ValueTag::Float64) => {
                let av = f64::from_bits(a.raw_data());
                let bv = f64::from_bits(b.raw_data());
                Ok(Value::float64(av + bv))
            }
            _ => Err(Error::TypeMismatch {
                expected: "compatible add operands".to_string(),
                actual: format!("{a:?}, {b:?}"),
            }),
        },
        ReduceOperator::Multiply => match (a_tag, b_tag) {
            (ValueTag::Int, ValueTag::Int) => {
                let av = a.raw_data() as i64;
                let bv = b.raw_data() as i64;
                Ok(Value::int(av.wrapping_mul(bv), a.width()))
            }
            (ValueTag::UInt, ValueTag::UInt) => {
                let av = a.raw_data();
                let bv = b.raw_data();
                Ok(Value::uint(av.wrapping_mul(bv), a.width()))
            }
            (ValueTag::Float32, ValueTag::Float32) => {
                let av = f32::from_bits(a.raw_data() as u32);
                let bv = f32::from_bits(b.raw_data() as u32);
                Ok(Value::float32(av * bv))
            }
            (ValueTag::Float64, ValueTag::Float64) => {
                let av = f64::from_bits(a.raw_data());
                let bv = f64::from_bits(b.raw_data());
                Ok(Value::float64(av * bv))
            }
            _ => Err(Error::TypeMismatch {
                expected: "compatible mul operands".to_string(),
                actual: format!("{a:?}, {b:?}"),
            }),
        },
        ReduceOperator::Min => match (a_tag, b_tag) {
            (ValueTag::Int, ValueTag::Int) => {
                let av = a.raw_data() as i64;
                let bv = b.raw_data() as i64;
                Ok(Value::int(av.min(bv), a.width()))
            }
            (ValueTag::UInt, ValueTag::UInt) => {
                let av = a.raw_data();
                let bv = b.raw_data();
                Ok(Value::uint(av.min(bv), a.width()))
            }
            (ValueTag::Float32, ValueTag::Float32) => {
                let av = f32::from_bits(a.raw_data() as u32);
                let bv = f32::from_bits(b.raw_data() as u32);
                Ok(Value::float32(av.min(bv)))
            }
            (ValueTag::Float64, ValueTag::Float64) => {
                let av = f64::from_bits(a.raw_data());
                let bv = f64::from_bits(b.raw_data());
                Ok(Value::float64(av.min(bv)))
            }
            _ => Err(Error::TypeMismatch {
                expected: "compatible min operands".to_string(),
                actual: format!("{a:?}, {b:?}"),
            }),
        },
        ReduceOperator::Max => match (a_tag, b_tag) {
            (ValueTag::Int, ValueTag::Int) => {
                let av = a.raw_data() as i64;
                let bv = b.raw_data() as i64;
                Ok(Value::int(av.max(bv), a.width()))
            }
            (ValueTag::UInt, ValueTag::UInt) => {
                let av = a.raw_data();
                let bv = b.raw_data();
                Ok(Value::uint(av.max(bv), a.width()))
            }
            (ValueTag::Float32, ValueTag::Float32) => {
                let av = f32::from_bits(a.raw_data() as u32);
                let bv = f32::from_bits(b.raw_data() as u32);
                Ok(Value::float32(av.max(bv)))
            }
            (ValueTag::Float64, ValueTag::Float64) => {
                let av = f64::from_bits(a.raw_data());
                let bv = f64::from_bits(b.raw_data());
                Ok(Value::float64(av.max(bv)))
            }
            _ => Err(Error::TypeMismatch {
                expected: "compatible max operands".to_string(),
                actual: format!("{a:?}, {b:?}"),
            }),
        },
        ReduceOperator::And => match (a_tag, b_tag) {
            (ValueTag::Int, ValueTag::Int) => {
                let av = a.raw_data() as i64;
                let bv = b.raw_data() as i64;
                Ok(Value::int(av & bv, a.width()))
            }
            (ValueTag::UInt, ValueTag::UInt) => {
                let av = a.raw_data();
                let bv = b.raw_data();
                Ok(Value::uint(av & bv, a.width()))
            }
            (ValueTag::Bool, ValueTag::Bool) => {
                Ok(Value::bool(a.raw_data() != 0 && b.raw_data() != 0))
            }
            _ => Err(Error::TypeMismatch {
                expected: "compatible and operands".to_string(),
                actual: format!("{a:?}, {b:?}"),
            }),
        },
        ReduceOperator::Or => match (a_tag, b_tag) {
            (ValueTag::Int, ValueTag::Int) => {
                let av = a.raw_data() as i64;
                let bv = b.raw_data() as i64;
                Ok(Value::int(av | bv, a.width()))
            }
            (ValueTag::UInt, ValueTag::UInt) => {
                let av = a.raw_data();
                let bv = b.raw_data();
                Ok(Value::uint(av | bv, a.width()))
            }
            (ValueTag::Bool, ValueTag::Bool) => {
                Ok(Value::bool(a.raw_data() != 0 || b.raw_data() != 0))
            }
            _ => Err(Error::TypeMismatch {
                expected: "compatible or operands".to_string(),
                actual: format!("{a:?}, {b:?}"),
            }),
        },
        ReduceOperator::Xor => match (a_tag, b_tag) {
            (ValueTag::Int, ValueTag::Int) => {
                let av = a.raw_data() as i64;
                let bv = b.raw_data() as i64;
                Ok(Value::int(av ^ bv, a.width()))
            }
            (ValueTag::UInt, ValueTag::UInt) => {
                let av = a.raw_data();
                let bv = b.raw_data();
                Ok(Value::uint(av ^ bv, a.width()))
            }
            (ValueTag::Bool, ValueTag::Bool) => {
                let av = a.raw_data() != 0;
                let bv = b.raw_data() != 0;
                Ok(Value::bool(av ^ bv))
            }
            _ => Err(Error::TypeMismatch {
                expected: "compatible xor operands".to_string(),
                actual: format!("{a:?}, {b:?}"),
            }),
        },
    }
}
