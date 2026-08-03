use super::assert_format_eq;

/// Format scalar operations and multi-result assignments canonically.
#[test]
fn test_format_scalar_operations() {
    assert_format_eq(
        r#"
function f0 {
    int.add.overflowing r2,r3, r0,r1: int32
return r2:r3
}
"#,
        r#"
function f0 {
    int.add.overflowing r2, r3, r0, r1: int32
    return r2:r3
}
"#,
    );
}

/// Format scalar and pointer conversions with one canonical cast form.
#[test]
fn test_format_casts() {
    assert_format_eq(
        r#"
function f0 {
    cast.truncate r4, r0: int64 -> int32
cast.floatToInt.u r5, r1: float64 -> uint64
cast.intToPointer r6, r3: uint64 -> pointer
cast.pointerToInt r7, r2: pointer -> uint64
cast.floatTruncate r8, r1: float64 -> float32
return r4:r8
}
"#,
        r#"
function f0 {
    cast.truncate r4, r0: int64 -> int32
    cast.floatToInt.u r5, r1: float64 -> uint64
    cast.intToPointer r6, r3: uint64 -> pointer
    cast.pointerToInt r7, r2: pointer -> uint64
    cast.floatTruncate r8, r1: float64 -> float32
    return r4:r8
}
"#,
    );
}

/// Preserve signed and unsigned 128-bit literal interpretation.
#[test]
fn test_format_wide_literals() {
    assert_format_eq(
        r#"
function f0 {
    constant r0, -1: int128
constant r2, 340282366920938463463374607431768211455: uint128
return r0:r3
}
"#,
        r#"
function f0 {
    constant r0, -1: int128
    constant r2, 340282366920938463463374607431768211455: uint128
    return r0:r3
}
"#,
    );
}

/// Preserve non-finite floating-point values and exact NaN payloads.
#[test]
fn test_format_non_finite_literals() {
    assert_format_eq(
        r#"
function f0 {
    constant r0, bits(0x7e01): float16
constant r1, Infinity: float32
constant r2, -Infinity: float64
return r0:r2
}
"#,
        r#"
function f0 {
    constant r0, bits(0x7e01): float16
    constant r1, Infinity: float32
    constant r2, -Infinity: float64
    return r0:r2
}
"#,
    );
}
