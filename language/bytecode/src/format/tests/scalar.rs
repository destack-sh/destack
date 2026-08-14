use super::assert_format_eq;

/// Format scalar operations and multi-result assignments canonically.
#[test]
fn test_format_scalar_operations() {
    assert_format_eq(
        r#"
function f0 {
    add.overflowing.int32 r2,r3, r0,r1
return r2:r3
}
"#,
        r#"
function f0 {
    add.overflowing.int32 r2, r3, r0, r1
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
    truncate.int64.int32 r4, r0
truncate.float64.uint64 r5, r1
reinterpret.uint64.pointer r6, r3
reinterpret.pointer.uint64 r7, r2
demote.float64.float32 r8, r1
return r4:r8
}
"#,
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
    );
}

/// Preserve signed and unsigned 128-bit literal interpretation.
#[test]
fn test_format_wide_literals() {
    assert_format_eq(
        r#"
function f0 {
    constant.int128 r0, -1
constant.uint128 r2, 340282366920938463463374607431768211455
return r0:r3
}
"#,
        r#"
function f0 {
    constant.int128 r0, -1
    constant.uint128 r2, 340282366920938463463374607431768211455
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
    constant.float16 r0, bits(0x7e01)
constant.float32 r1, Infinity
constant.float64 r2, -Infinity
return r0:r2
}
"#,
        r#"
function f0 {
    constant.float16 r0, bits(0x7e01)
    constant.float32 r1, Infinity
    constant.float64 r2, -Infinity
    return r0:r2
}
"#,
    );
}
