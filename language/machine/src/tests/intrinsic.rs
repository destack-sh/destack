//! Tests for intrinsic execution.

use crate::memory::Value;
use crate::tests::{run_mir, run_mir_expect, run_mir_ok};

// bit manipulation

#[test]
fn test_intrinsic_clz() {
    // count leading zeros: 0x00800000 has 8 leading zeros in 32-bit
    let mir = r#"
function @test(v0: u32) -> u32 {
block0(v0: u32):
    v1 = intrinsic.clz(v0)
    return v1
}
"#;
    run_mir_expect(mir, "test", &[Value::uint32(0x00800000)], Value::uint32(8));
}

#[test]
fn test_intrinsic_ctz() {
    // count trailing zeros: 0x80 = 128 = 0b10000000 has 7 trailing zeros
    let mir = r#"
function @test(v0: u32) -> u32 {
block0(v0: u32):
    v1 = intrinsic.ctz(v0)
    return v1
}
"#;
    run_mir_expect(mir, "test", &[Value::uint32(0x80)], Value::uint32(7));
}

#[test]
fn test_intrinsic_popcnt() {
    // population count: 0xFF = 255 = 0b11111111 has 8 bits set
    let mir = r#"
function @test(v0: u32) -> u32 {
block0(v0: u32):
    v1 = intrinsic.popcnt(v0)
    return v1
}
"#;
    run_mir_expect(mir, "test", &[Value::uint32(0xFF)], Value::uint32(8));
}

#[test]
fn test_intrinsic_byte_swap() {
    // byte swap: 0x12345678 -> 0x78563412
    let mir = r#"
function @test(v0: u32) -> u32 {
block0(v0: u32):
    v1 = intrinsic.byte_swap(v0)
    return v1
}
"#;
    run_mir_expect(
        mir,
        "test",
        &[Value::uint32(0x12345678)],
        Value::uint32(0x78563412),
    );
}

#[test]
fn test_intrinsic_rotate_left() {
    // rotate left: 0x80000001 rotated left by 1 = 0x00000003
    let mir = r#"
function @test(v0: u32, v1: u32) -> u32 {
block0(v0: u32, v1: u32):
    v2 = intrinsic.rotate_left(v0, v1)
    return v2
}
"#;
    run_mir_expect(
        mir,
        "test",
        &[Value::uint32(0x80000001), Value::uint32(1)],
        Value::uint32(0x00000003),
    );
}

#[test]
fn test_intrinsic_rotate_right() {
    // rotate right: 0x00000003 rotated right by 1 = 0x80000001
    let mir = r#"
function @test(v0: u32, v1: u32) -> u32 {
block0(v0: u32, v1: u32):
    v2 = intrinsic.rotate_right(v0, v1)
    return v2
}
"#;
    run_mir_expect(
        mir,
        "test",
        &[Value::uint32(0x00000003), Value::uint32(1)],
        Value::uint32(0x80000001),
    );
}

// checked arithmetic

#[test]
fn test_intrinsic_add_overflow_no_overflow() {
    // 10 + 20 = 30, no overflow
    let mir = r#"
function @test(v0: i32, v1: i32) -> (i32, bool) {
block0(v0: i32, v1: i32):
    v2 = intrinsic.add.overflow(v0, v1)
    return v2
}
"#;
    let output = run_mir_ok(mir, "test", &[Value::int32(10), Value::int32(20)]);
    if let Value::Aggregate(fields) = &output.value {
        assert_eq!(fields[0], Value::int32(30));
        assert_eq!(fields[1], Value::Bool(false));
    } else {
        panic!("expected aggregate, got {:?}", output.value);
    }
}

#[test]
fn test_intrinsic_add_overflow_with_overflow() {
    // i32::MAX + 1 overflows
    let mir = r#"
function @test(v0: i32, v1: i32) -> (i32, bool) {
block0(v0: i32, v1: i32):
    v2 = intrinsic.add.overflow(v0, v1)
    return v2
}
"#;
    let output = run_mir_ok(mir, "test", &[Value::int32(i32::MAX), Value::int32(1)]);
    if let Value::Aggregate(fields) = &output.value {
        assert_eq!(
            fields[1],
            Value::Bool(true),
            "expected overflow flag to be true"
        );
    } else {
        panic!("expected aggregate, got {:?}", output.value);
    }
}

#[test]
fn test_intrinsic_sub_overflow() {
    // 0 - 1 for unsigned overflows
    let mir = r#"
function @test(v0: u32, v1: u32) -> (u32, bool) {
block0(v0: u32, v1: u32):
    v2 = intrinsic.sub.overflow(v0, v1)
    return v2
}
"#;
    let output = run_mir_ok(mir, "test", &[Value::uint32(0), Value::uint32(1)]);
    if let Value::Aggregate(fields) = &output.value {
        assert_eq!(
            fields[1],
            Value::Bool(true),
            "expected overflow flag to be true"
        );
    } else {
        panic!("expected aggregate, got {:?}", output.value);
    }
}

// saturating arithmetic

#[test]
fn test_intrinsic_sat_add() {
    // i32::MAX + 10 saturates to i32::MAX
    let mir = r#"
function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = intrinsic.add.sat(v0, v1)
    return v2
}
"#;
    run_mir_expect(
        mir,
        "test",
        &[Value::int32(i32::MAX), Value::int32(10)],
        Value::int32(i32::MAX),
    );
}

#[test]
fn test_intrinsic_sat_sub() {
    // 0u32 - 10 saturates to 0
    let mir = r#"
function @test(v0: u32, v1: u32) -> u32 {
block0(v0: u32, v1: u32):
    v2 = intrinsic.sub.sat(v0, v1)
    return v2
}
"#;
    run_mir_expect(
        mir,
        "test",
        &[Value::uint32(0), Value::uint32(10)],
        Value::uint32(0),
    );
}

// unchecked arithmetic

#[test]
fn test_intrinsic_add_unchecked() {
    let mir = r#"
function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = intrinsic.add.unchecked(v0, v1)
    return v2
}
"#;
    run_mir_expect(
        mir,
        "test",
        &[Value::int32(100), Value::int32(200)],
        Value::int32(300),
    );
}

#[test]
fn test_intrinsic_div_unchecked() {
    let mir = r#"
function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = intrinsic.div.unchecked(v0, v1)
    return v2
}
"#;
    run_mir_expect(
        mir,
        "test",
        &[Value::int32(100), Value::int32(10)],
        Value::int32(10),
    );
}

#[test]
fn test_intrinsic_div_by_zero_unchecked() {
    let mir = r#"
function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = intrinsic.div.unchecked(v0, v1)
    return v2
}
"#;
    let result = run_mir(mir, "test", &[Value::int32(100), Value::int32(0)]);
    assert!(result.is_err(), "expected division by zero error");
}

// float math

#[test]
fn test_intrinsic_sqrt() {
    let mir = r#"
function @test(v0: f64) -> f64 {
block0(v0: f64):
    v1 = intrinsic.sqrt(v0)
    return v1
}
"#;
    run_mir_expect(mir, "test", &[Value::float64(16.0)], Value::float64(4.0));
}

#[test]
fn test_intrinsic_abs() {
    let mir = r#"
function @test(v0: f64) -> f64 {
block0(v0: f64):
    v1 = intrinsic.abs(v0)
    return v1
}
"#;
    run_mir_expect(mir, "test", &[Value::float64(-42.5)], Value::float64(42.5));
}

#[test]
fn test_intrinsic_floor() {
    let mir = r#"
function @test(v0: f64) -> f64 {
block0(v0: f64):
    v1 = intrinsic.floor(v0)
    return v1
}
"#;
    run_mir_expect(mir, "test", &[Value::float64(3.7)], Value::float64(3.0));
}

#[test]
fn test_intrinsic_ceil() {
    let mir = r#"
function @test(v0: f64) -> f64 {
block0(v0: f64):
    v1 = intrinsic.ceil(v0)
    return v1
}
"#;
    run_mir_expect(mir, "test", &[Value::float64(3.2)], Value::float64(4.0));
}

#[test]
fn test_intrinsic_round() {
    let mir = r#"
function @test(v0: f64) -> f64 {
block0(v0: f64):
    v1 = intrinsic.round(v0)
    return v1
}
"#;
    run_mir_expect(mir, "test", &[Value::float64(3.5)], Value::float64(4.0));
}

#[test]
fn test_intrinsic_min() {
    let mir = r#"
function @test(v0: f64, v1: f64) -> f64 {
block0(v0: f64, v1: f64):
    v2 = intrinsic.min(v0, v1)
    return v2
}
"#;
    run_mir_expect(
        mir,
        "test",
        &[Value::float64(10.0), Value::float64(5.0)],
        Value::float64(5.0),
    );
}

#[test]
fn test_intrinsic_max() {
    let mir = r#"
function @test(v0: f64, v1: f64) -> f64 {
block0(v0: f64, v1: f64):
    v2 = intrinsic.max(v0, v1)
    return v2
}
"#;
    run_mir_expect(
        mir,
        "test",
        &[Value::float64(10.0), Value::float64(5.0)],
        Value::float64(10.0),
    );
}

#[test]
fn test_intrinsic_pow() {
    let mir = r#"
function @test(v0: f64, v1: f64) -> f64 {
block0(v0: f64, v1: f64):
    v2 = intrinsic.pow(v0, v1)
    return v2
}
"#;
    run_mir_expect(
        mir,
        "test",
        &[Value::float64(2.0), Value::float64(10.0)],
        Value::float64(1024.0),
    );
}

#[test]
fn test_intrinsic_fma() {
    // fma(2.0, 3.0, 4.0) = 2.0 * 3.0 + 4.0 = 10.0
    let mir = r#"
function @test(v0: f64, v1: f64, v2: f64) -> f64 {
block0(v0: f64, v1: f64, v2: f64):
    v3 = intrinsic.fma(v0, v1, v2)
    return v3
}
"#;
    run_mir_expect(
        mir,
        "test",
        &[
            Value::float64(2.0),
            Value::float64(3.0),
            Value::float64(4.0),
        ],
        Value::float64(10.0),
    );
}

// trigonometry (just test they execute without error)

#[test]
fn test_intrinsic_sin_cos() {
    let mir = r#"
function @test(v0: f64) -> f64 {
block0(v0: f64):
    v1 = intrinsic.sin(v0)
    v2 = intrinsic.cos(v0)
    v3 = fadd v1, v2
    return v3
}
"#;
    // sin(0) = 0, cos(0) = 1, so result = 1
    run_mir_expect(mir, "test", &[Value::float64(0.0)], Value::float64(1.0));
}

// branch hints (passthrough)

#[test]
fn test_intrinsic_likely() {
    let mir = r#"
function @test(v0: bool) -> bool {
block0(v0: bool):
    v1 = intrinsic.likely(v0)
    return v1
}
"#;
    run_mir_expect(mir, "test", &[Value::Bool(true)], Value::Bool(true));
}

#[test]
fn test_intrinsic_unlikely() {
    let mir = r#"
function @test(v0: bool) -> bool {
block0(v0: bool):
    v1 = intrinsic.unlikely(v0)
    return v1
}
"#;
    run_mir_expect(mir, "test", &[Value::Bool(false)], Value::Bool(false));
}

#[test]
fn test_intrinsic_black_box() {
    let mir = r#"
function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = intrinsic.black_box(v0)
    return v1
}
"#;
    run_mir_expect(mir, "test", &[Value::int32(42)], Value::int32(42));
}

// comparison

#[test]
fn test_intrinsic_raw_eq_true() {
    let mir = r#"
function @test(v0: i32, v1: i32) -> bool {
block0(v0: i32, v1: i32):
    v2 = intrinsic.raw_eq(v0, v1)
    return v2
}
"#;
    run_mir_expect(
        mir,
        "test",
        &[Value::int32(42), Value::int32(42)],
        Value::Bool(true),
    );
}

#[test]
fn test_intrinsic_raw_eq_false() {
    let mir = r#"
function @test(v0: i32, v1: i32) -> bool {
block0(v0: i32, v1: i32):
    v2 = intrinsic.raw_eq(v0, v1)
    return v2
}
"#;
    run_mir_expect(
        mir,
        "test",
        &[Value::int32(42), Value::int32(43)],
        Value::Bool(false),
    );
}

// control flow

#[test]
fn test_intrinsic_breakpoint() {
    // breakpoint should be a no-op
    let mir = r#"
function @test(v0: i32) -> i32 {
block0(v0: i32):
    intrinsic.breakpoint()
    return v0
}
"#;
    run_mir_expect(mir, "test", &[Value::int32(42)], Value::int32(42));
}

#[test]
fn test_intrinsic_unreachable() {
    let mir = r#"
function @test(v0: i32) -> i32 {
block0(v0: i32):
    intrinsic.unreachable()
    return v0
}
"#;
    let result = run_mir(mir, "test", &[Value::int32(42)]);
    assert!(result.is_err(), "expected unreachable error");
}

#[test]
fn test_intrinsic_abort() {
    let mir = r#"
function @test(v0: i32) -> i32 {
block0(v0: i32):
    intrinsic.abort()
    return v0
}
"#;
    let result = run_mir(mir, "test", &[Value::int32(42)]);
    assert!(result.is_err(), "expected abort error");
}

// transmute

#[test]
fn test_intrinsic_transmute() {
    // transmute just returns the same bits
    let mir = r#"
function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = intrinsic.transmute(v0)
    return v1
}
"#;
    run_mir_expect(mir, "test", &[Value::int32(42)], Value::int32(42));
}

// runtime introspection

#[test]
fn test_intrinsic_return_address() {
    // return_address returns a synthetic address when there's a caller
    let mir = r#"
function @inner() -> u64 {
block0:
    v0 = intrinsic.return_address()
    return v0
}

function @test() -> u64 {
block0:
    v0 = call @inner()
    return v0
}
"#;
    // should return non-zero since there's a caller
    let output = run_mir_ok(mir, "test", &[]);
    if let Value::UInt { value, width: 64 } = output.value {
        assert!(value != 0, "expected non-zero return address");
    } else {
        panic!("expected u64, got {:?}", output.value);
    }
}

#[test]
fn test_intrinsic_return_address_no_caller() {
    // return_address returns 0 when there's no caller
    let mir = r#"
function @test() -> u64 {
block0:
    v0 = intrinsic.return_address()
    return v0
}
"#;
    run_mir_expect(
        mir,
        "test",
        &[],
        Value::UInt {
            value: 0,
            width: 64,
        },
    );
}

#[test]
fn test_intrinsic_frame_address() {
    // frame_address returns a synthetic address based on call depth
    let mir = r#"
function @inner() -> u64 {
block0:
    v0 = intrinsic.frame_address()
    return v0
}

function @test() -> u64 {
block0:
    v0 = call @inner()
    return v0
}
"#;
    let output = run_mir_ok(mir, "test", &[]);
    if let Value::UInt { value, width: 64 } = output.value {
        // should have high bits set (0x7FFF_0000_0000_0000) plus frame index
        assert!(
            value > 0x7FFF_0000_0000_0000u64,
            "expected synthetic frame address"
        );
    } else {
        panic!("expected u64, got {:?}", output.value);
    }
}

// SIMD horizontal reductions

#[test]
fn test_intrinsic_reduce_add() {
    let mir = r#"
function @test(v0: (i32, i32, i32, i32)) -> i32 {
block0(v0: (i32, i32, i32, i32)):
    v1 = intrinsic.reduce.add(v0)
    return v1
}
"#;
    let input = Value::Aggregate(
        vec![
            Value::int32(1),
            Value::int32(2),
            Value::int32(3),
            Value::int32(4),
        ]
        .into(),
    );
    run_mir_expect(mir, "test", &[input], Value::int32(10));
}

#[test]
fn test_intrinsic_reduce_mul() {
    let mir = r#"
function @test(v0: (i32, i32, i32, i32)) -> i32 {
block0(v0: (i32, i32, i32, i32)):
    v1 = intrinsic.reduce.mul(v0)
    return v1
}
"#;
    let input = Value::Aggregate(
        vec![
            Value::int32(2),
            Value::int32(3),
            Value::int32(4),
            Value::int32(5),
        ]
        .into(),
    );
    run_mir_expect(mir, "test", &[input], Value::int32(120));
}

#[test]
fn test_intrinsic_reduce_min() {
    let mir = r#"
function @test(v0: (i32, i32, i32, i32)) -> i32 {
block0(v0: (i32, i32, i32, i32)):
    v1 = intrinsic.reduce.min(v0)
    return v1
}
"#;
    let input = Value::Aggregate(
        vec![
            Value::int32(5),
            Value::int32(2),
            Value::int32(8),
            Value::int32(1),
        ]
        .into(),
    );
    run_mir_expect(mir, "test", &[input], Value::int32(1));
}

#[test]
fn test_intrinsic_reduce_max() {
    let mir = r#"
function @test(v0: (i32, i32, i32, i32)) -> i32 {
block0(v0: (i32, i32, i32, i32)):
    v1 = intrinsic.reduce.max(v0)
    return v1
}
"#;
    let input = Value::Aggregate(
        vec![
            Value::int32(5),
            Value::int32(2),
            Value::int32(8),
            Value::int32(1),
        ]
        .into(),
    );
    run_mir_expect(mir, "test", &[input], Value::int32(8));
}

#[test]
fn test_intrinsic_reduce_and() {
    let mir = r#"
function @test(v0: (u32, u32, u32, u32)) -> u32 {
block0(v0: (u32, u32, u32, u32)):
    v1 = intrinsic.reduce.and(v0)
    return v1
}
"#;
    let input = Value::Aggregate(
        vec![
            Value::uint32(0b1111),
            Value::uint32(0b1110),
            Value::uint32(0b1100),
            Value::uint32(0b1000),
        ]
        .into(),
    );
    run_mir_expect(mir, "test", &[input], Value::uint32(0b1000));
}

#[test]
fn test_intrinsic_reduce_or() {
    let mir = r#"
function @test(v0: (u32, u32, u32, u32)) -> u32 {
block0(v0: (u32, u32, u32, u32)):
    v1 = intrinsic.reduce.or(v0)
    return v1
}
"#;
    let input = Value::Aggregate(
        vec![
            Value::uint32(0b0001),
            Value::uint32(0b0010),
            Value::uint32(0b0100),
            Value::uint32(0b1000),
        ]
        .into(),
    );
    run_mir_expect(mir, "test", &[input], Value::uint32(0b1111));
}

#[test]
fn test_intrinsic_reduce_xor() {
    let mir = r#"
function @test(v0: (u32, u32, u32, u32)) -> u32 {
block0(v0: (u32, u32, u32, u32)):
    v1 = intrinsic.reduce.xor(v0)
    return v1
}
"#;
    // 1 ^ 2 ^ 3 ^ 4 = 4
    let input = Value::Aggregate(
        vec![
            Value::uint32(1),
            Value::uint32(2),
            Value::uint32(3),
            Value::uint32(4),
        ]
        .into(),
    );
    run_mir_expect(mir, "test", &[input], Value::uint32(4));
}
