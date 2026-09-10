use super::assert_format;

/// Formats vector operations canonically.
#[test]
fn test_format_vector_ops() {
    assert_format(
        r#"
function vectorOps(v0: vector<int32, 4>, v1: int32): vector<int32, 4> {
entry(v0: vector<int32, 4>, v1: int32):
    v4: vector<int32, 4> = vector.splat v1
    v5: int32 = vector.extract v4, v1
    v6: vector<int32, 4> = vector.insert v4, v1, v1
    v7: vector<int32, 4> = vector.shuffle v4, v6, [0, 1, 2, 3]
    v8: int32 = vector.reduce add, v7
    v9: vector<boolean, 4> = vector.compare eq, v4, v6
    v10: vector<int32, 4> = vector.convert exact, v4
    return v10
}
"#,
    );
}

/// Formats check terminators and marker instructions canonically.
#[test]
fn test_format_check_and_assume() {
    assert_format(
        r#"
function guard(v0: uint32, v1: uint32, v2: [int32; 4]): int32 {
entry(v0: uint32, v1: uint32, v2: [int32; 4]):
    v3: boolean = lt v0, v1
    assume v3
    breakpoint
    profile.increment counter(0)
    profile.sample sampler(1), v3
    check bounds.u v0, v1, v2 => b1(v0) | b2

b1(v4: uint32):
    v5: int32 = 0
    return v5

b2:
    unreachable
}
"#,
    );
}

/// Formats every instruction call dispatch canonically.
#[test]
fn test_format_calls() {
    assert_format(
        r#"
external function callee(int32, int32): int32

function caller(): int32 {
entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: int32 = call callee(v0, v1): (int32, int32) => int32
    v3: fn(int32, int32) => int32 = function.address callee
    v4: int32 = call.indirect v3(v0, v1): (int32, int32) => int32
    v5: int32 = call.virtual v0, int32, 0(v0, v1): (int32, int32) => int32
    v6: int32 = call.dynamic v0, int32, 0(v0, v1): (int32, int32) => int32
    return v6
}
"#,
    );
}

/// Formats void calls with callable type arguments canonically.
#[test]
fn test_format_void_call_with_callable_argument() {
    assert_format(
        r#"
external function consume(function<() => int32, repeatable, managed, mutable, local>): void

function caller(v0: function<() => int32, repeatable, managed, mutable, local>): void {
entry(v0: function<() => int32, repeatable, managed, mutable, local>):
    call consume(v0): (function<() => int32, repeatable, managed, mutable, local>) => void
    return
}
"#,
    );
}

/// Formats scalar instruction families canonically.
#[test]
fn test_format_scalar_instruction_families() {
    assert_format(
        r#"
function scalarOps(v0: int32, v1: int32, v2: boolean, v3: float64): int64 {
entry(v0: int32, v1: int32, v2: boolean, v3: float64):
    v4: boolean = lt v0, v1
    v5: int32 = select v2, v0, v1
    v6: int32 = negate v5
    v7: int32 = not v6
    v8: float64 = negate v3
    v9: int64 = cast.intToInt v7 -> int64
    v10: float64 = intrinsic.math.float.sqrt(v3)
    v11: float64 = intrinsic.math.float.min(v8, v3)
    v12: float64 = intrinsic.math.float.fma(v8, v3, v11)
    v13: (int32, boolean) = intrinsic.math.arithmetic.overflowing.add(v0, v1)
    return v9
}
"#,
    );
}

/// Infinite and NaN float literals round-trip through the formatter.
#[test]
fn test_format_float_infinity_and_nan() {
    assert_format(
        r#"
function floatLimits(): float32 {
entry:
    v0: float32 = inf
    v1: float32 = -inf
    v2: float32 = NaN
    return v1
}
"#,
    );
}
