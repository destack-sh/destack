use crate::{
    BooleanOperation, CastOperation, FunctionId, IntegerOperation, Opcode, Result, Scalar,
    ValueType,
};

use super::TestParser;

/// Parse scalar operations and register reassignment into exact opcodes.
#[test]
fn test_parse_scalar_operations() {
    let (_, opcodes) = TestParser::new(
        r#"
function f0 {
    add.int32 r2, r0, r1
    add.overflowing.int32 r2, r3, r2, r1
    not.boolean r3, r3
    return r2:r3
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![
            Opcode::integer(IntegerOperation::Add, Scalar::Int32).expect("integer opcode"),
            Opcode::integer(IntegerOperation::AddOverflow, Scalar::Int32).expect("integer opcode"),
            Opcode::boolean(BooleanOperation::Not),
            Opcode::RETURN,
        ]
    );
}

/// Parse scalar and pointer conversions through one cast form.
#[test]
fn test_parse_casts() {
    let (_, opcodes) = TestParser::new(
        r#"
function f0 {
    truncate.int64.int32 r4, r0
    truncate.float64.uint64 r5, r1
    reinterpret.uint64.pointer r6, r3
    reinterpret.pointer.uint64 r7, r2
    demote.float64.float32 r8, r1
    return r4:r8
}
"#,
    )
    .parse_opcodes(FunctionId(0));

    assert_eq!(
        opcodes,
        vec![
            Opcode::cast(
                CastOperation::Truncate,
                ValueType::scalar(Scalar::Int64),
                ValueType::scalar(Scalar::Int32),
            )
            .expect("integer cast opcode"),
            Opcode::cast(
                CastOperation::FloatToInt,
                ValueType::scalar(Scalar::Float64),
                ValueType::scalar(Scalar::Uint64),
            )
            .expect("float to integer cast opcode"),
            Opcode::CAST_INT_TO_POINTER,
            Opcode::CAST_POINTER_TO_INT,
            Opcode::cast(
                CastOperation::FloatConvert,
                ValueType::scalar(Scalar::Float64),
                ValueType::scalar(Scalar::Float32),
            )
            .expect("floating point cast opcode"),
            Opcode::RETURN,
        ]
    );
}

/// Parse 128-bit values as contiguous two-register values.
#[test]
fn test_parse_wide_integer_registers() {
    let (object, opcodes) = TestParser::new(
        r#"
function f0 {
    add.int128 r4:r5, r0:r1, r2:r3
    return r4:r5
}
"#,
    )
    .parse_opcodes(FunctionId(0));
    assert_eq!(
        opcodes,
        vec![
            Opcode::integer128(IntegerOperation::Add, true),
            Opcode::RETURN
        ]
    );
    assert_eq!(object.functions()[0].register_count, 6);
}

/// Parse floating-point literals with decimal exponents.
#[test]
fn test_parse_float_literals() {
    let object = TestParser::new(
        r#"
function f0 {
    constant.float32 r0, 1.25e-3
    constant.float64 r1, -2E+2
    return r0:r1
}
"#,
    )
    .parse();
    let bits = object
        .instructions(FunctionId(0))
        .expect("defined function")
        .take(2)
        .map(|instruction| instruction.expect("valid instruction").read_u64(2))
        .collect::<Result<Vec<_>>>()
        .expect("valid literal operands");

    assert_eq!(
        bits,
        vec![u64::from((1.25e-3_f32).to_bits()), (-2e2_f64).to_bits()]
    );
}
