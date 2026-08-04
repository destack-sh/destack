use crate::tests::TestProgram;

/// Emit scalar constants with their exact bytecode representations.
#[test]
fn test_emit_bytecode_constants() {
    let program = TestProgram::mir(
        r#"
export function constants(): int128 {
entry:
    v0: ref<int32, managed, mutable, nullable> = null
    v1: ref<int32, managed, mutable, undefined> = undefined
    v2: boolean = true
    v3: char = 'A'
    v4: int8 = -8
    v5: uint16 = 65535
    v6: int128 = -170141183460469231731687303715884105728
    v7: uint128 = 340282366920938463463374607431768211455
    v8: float32 = 1.5
    v9: float64 = -2.25
    return v6
}
"#,
    );

    program.assert_bytecode(
        r#"
function constants {
    constant r0, null
    constant r0, undefined
    constant r0, true: boolean
    constant r0, 65: uint32
    constant r0, -8: int8
    constant r0, 65535: uint16
    constant r1:r2, -170141183460469231731687303715884105728: int128
    constant r3:r4, 340282366920938463463374607431768211455: uint128
    constant r0, 1.5: float32
    constant r0, -2.25: float64
    return r1:r2
}
"#,
    );
}

/// Emit signed, unsigned, bitwise, unary, comparison, and selection operations.
#[test]
fn test_emit_bytecode_integer_operations() {
    let program = TestProgram::mir(
        r#"
export function integer(v0: int64, v1: int64, v2: uint64, v3: uint64): int64 {
entry(v0: int64, v1: int64, v2: uint64, v3: uint64):
    v4: int64 = int.add v0, v1
    v5: int64 = int.sub v4, v1
    v6: int64 = int.mul v5, v1
    v7: int64 = int.div.s v6, v1
    v8: int64 = int.rem.s v7, v1
    v9: uint64 = int.div.u v2, v3
    v10: uint64 = int.rem.u v9, v3
    v11: int64 = int.and v8, v1
    v12: int64 = int.or v11, v0
    v13: int64 = int.xor v12, v1
    v14: int64 = int.shl v13, v1
    v15: int64 = int.shr.s v14, v1
    v16: uint64 = int.shr.u v10, v3
    v17: int64 = int.negate v15
    v18: int64 = int.not v17
    v19: boolean = int.lt.s v18, v0
    v20: boolean = int.ge.u v16, v2
    v21: int64 = select v19, v18, v0
    return v21
}
"#,
    );

    program.assert_bytecode(
        r#"
function integer {
    int.add r4, r0, r1: int64
    int.sub r5, r4, r1: int64
    int.mul r4, r5, r1: int64
    int.div r5, r4, r1: int64
    int.rem r4, r5, r1: int64
    int.div r5, r2, r3: uint64
    int.rem r6, r5, r3: uint64
    int.and r5, r4, r1: int64
    int.or r4, r5, r0: int64
    int.xor r5, r4, r1: int64
    int.shl r4, r5, r1: int64
    int.shr r5, r4, r1: int64
    int.shr r1, r6, r3: uint64
    int.negate r3, r5: int64
    int.not r4, r3: int64
    int.lt r3, r4, r0: int64
    int.ge r5, r1, r2: uint64
    select r1, r3, r4, r0
    return r1
}
"#,
    );
}

/// Emit floating-point arithmetic, unary, comparison, and selection operations.
#[test]
fn test_emit_bytecode_float_operations() {
    let program = TestProgram::mir(
        r#"
export function float(v0: float64, v1: float64): float64 {
entry(v0: float64, v1: float64):
    v2: float64 = float.add v0, v1
    v3: float64 = float.sub v2, v1
    v4: float64 = float.mul v3, v1
    v5: float64 = float.div v4, v1
    v6: float64 = float.negate v5
    v7: boolean = float.lt v6, v0
    v8: float64 = select v7, v6, v0
    return v8
}
"#,
    );

    program.assert_bytecode(
        r#"
function float {
    float.add r2, r0, r1: float64
    float.sub r3, r2, r1: float64
    float.mul r2, r3, r1: float64
    float.div r3, r2, r1: float64
    float.negate r1, r3: float64
    float.lt r2, r1, r0: float64
    select r3, r2, r1, r0
    return r3
}
"#,
    );
}

/// Emit every scalar representation conversion family.
#[test]
fn test_emit_bytecode_casts() {
    let program = TestProgram::mir(
        r#"
export function cast(v0: int64, v1: uint64, v2: float64): uint64 {
entry(v0: int64, v1: uint64, v2: float64):
    v3: int32 = cast.truncate v0 -> int32
    v4: int8 = cast.saturate v0 -> int8
    v5: uint64 = cast.extend.u v3 -> uint64
    v6: int64 = cast.extend.s v3 -> int64
    v7: int32 = cast.floatToInt.s v2 -> int32
    v8: uint32 = cast.floatToInt.u v2 -> uint32
    v9: int32 = cast.floatToIntSaturating.s v2 -> int32
    v10: uint32 = cast.floatToIntSaturating.u v2 -> uint32
    v11: float64 = cast.intToFloat.s v0 -> float64
    v12: float64 = cast.intToFloat.u v1 -> float64
    v13: float32 = cast.floatTruncate v2 -> float32
    v14: float64 = cast.floatExtend v13 -> float64
    v15: uint64 = cast.bit v11 -> uint64
    return v15
}
"#,
    );

    program.assert_bytecode(
        r#"
function cast {
    cast.truncate r3, r0: int64 -> int32
    cast.saturate r4, r0: int64 -> int8
    cast.extend.u r4, r3: int32 -> uint64
    cast.extend.s r4, r3: int32 -> int64
    cast.floatToInt.s r3, r2: float64 -> int32
    cast.floatToInt.u r3, r2: float64 -> uint32
    cast.floatToIntSaturating.s r3, r2: float64 -> int32
    cast.floatToIntSaturating.u r3, r2: float64 -> uint32
    cast.intToFloat.s r3, r0: int64 -> float64
    cast.intToFloat.u r0, r1: uint64 -> float64
    cast.floatTruncate r0, r2: float64 -> float32
    cast.floatExtend r1, r0: float32 -> float64
    cast.bit r0, r3: float64 -> uint64
    return r0
}
"#,
    );
}
