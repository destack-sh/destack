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
export function calculate(r0: int32, r1: int32): (int32, boolean) {
    r2: int32 = int.add r0, r1
    r2: int32, r3: boolean = int.add.overflowing r2, r1
    r3: boolean = int.not r3
    return r2, r3
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
export function convert(
    r0: int64,
    r1: float64,
    r2: pointer,
    r3: uint64,
): (int32, uint64, pointer, uint64, float32) {
    r4: int32 = cast.truncate r0 -> int32
    r5: uint64 = cast.floatToInt.u r1 -> uint64
    r6: pointer = cast.intToPointer r3 -> pointer
    r7: uint64 = cast.pointerToInt r2 -> uint64
    r8: float32 = cast.floatTruncate r1 -> float32
    return r4, r5, r6, r7, r8
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
export function addWide(r0: int128, r2: int128): int128 {
    r4: int128 = int.add r0, r2
    return r4
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
    assert_eq!(object.functions()[0].body.register_count, 6);
}

/// Parse decimal exponents and round narrow literals using ties-to-even.
#[test]
fn test_parse_narrow_float_literals() {
    let object = TestParser::new(
        r#"
export function literals(): (float16, float16, bfloat16, bfloat16, float64, float64) {
    r0: float16 = 1.00048828125
    r1: float16 = 1.0009765625
    r2: bfloat16 = 1.00390625
    r3: bfloat16 = 1.0078125
    r4: float64 = 1.25e-3
    r5: float64 = -2E+2
    return r0, r1, r2, r3, r4, r5
}
"#,
    )
    .parse();
    let bits = object
        .instructions(FunctionId(0))
        .expect("defined function")
        .take(6)
        .map(|instruction| instruction.expect("valid instruction").read_u64(2))
        .collect::<Result<Vec<_>>>()
        .expect("valid literal operands");

    assert_eq!(
        bits,
        vec![
            0x3c00,
            0x3c01,
            0x3f80,
            0x3f81,
            1.25e-3_f64.to_bits(),
            (-2e2_f64).to_bits(),
        ]
    );
}
