//! Tests for intrinsic execution.

use crate::tests::{run_mir, run_mir_expect, run_mir_ok, run_mir_with_ok};
use destack_heap::{STRING_TYPE_ALIAS, Value};

// bit manipulation

#[test]
fn test_intrinsic_clz() {
    // count leading zeros: 0x00800000 has 8 leading zeros in 32-bit
    let mir = r#"
function @test(v0: u32) -> u32 {
block0(v0: u32):
    v1: u32 = intrinsic.clz(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Value::uint32(0x00800000)], Value::uint32(8));
}

#[test]
fn test_intrinsic_ctz() {
    // count trailing zeros: 0x80 = 128 = 0b10000000 has 7 trailing zeros
    let mir = r#"
function @test(v0: u32) -> u32 {
block0(v0: u32):
    v1: u32 = intrinsic.ctz(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Value::uint32(0x80)], Value::uint32(7));
}

#[test]
fn test_intrinsic_popcnt() {
    // population count: 0xFF = 255 = 0b11111111 has 8 bits set
    let mir = r#"
function @test(v0: u32) -> u32 {
block0(v0: u32):
    v1: u32 = intrinsic.popcnt(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Value::uint32(0xFF)], Value::uint32(8));
}

#[test]
fn test_intrinsic_byte_swap() {
    // byte swap: 0x12345678 -> 0x78563412
    let mir = r#"
function @test(v0: u32) -> u32 {
block0(v0: u32):
    v1: u32 = intrinsic.byte_swap(v0)
    return v1
}"#;
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
    v2: u32 = intrinsic.rotate_left(v0, v1)
    return v2
}"#;
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
    v2: u32 = intrinsic.rotate_right(v0, v1)
    return v2
}"#;
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
    // 10 + 20 = 30, no overflow - test result value
    let mir = r#"
function @test_result(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2: (i32, bool) = intrinsic.add.overflow(v0, v1)
    v3: i32 = field.get v2, 0
    return v3
}"#;
    let output = run_mir_ok(mir, "test_result", &[Value::int32(10), Value::int32(20)]);
    assert_eq!(output.value, Value::int32(30));

    // test no overflow flag
    let mir = r#"
function @test_flag(v0: i32, v1: i32) -> bool {
block0(v0: i32, v1: i32):
    v2: (i32, bool) = intrinsic.add.overflow(v0, v1)
    v3: bool = field.get v2, 1
    return v3
}"#;
    let output = run_mir_ok(mir, "test_flag", &[Value::int32(10), Value::int32(20)]);
    assert_eq!(output.value, Value::bool(false));
}

#[test]
fn test_intrinsic_add_overflow_with_overflow() {
    // i32::MAX + 1 overflows - test overflow flag
    let mir = r#"
function @test(v0: i32, v1: i32) -> bool {
block0(v0: i32, v1: i32):
    v2: (i32, bool) = intrinsic.add.overflow(v0, v1)
    v3: bool = field.get v2, 1
    return v3
}"#;
    let output = run_mir_ok(mir, "test", &[Value::int32(i32::MAX), Value::int32(1)]);
    assert_eq!(
        output.value,
        Value::bool(true),
        "expected overflow flag to be true"
    );
}

#[test]
fn test_intrinsic_sub_overflow() {
    // 0 - 1 for unsigned overflows - test overflow flag
    let mir = r#"
function @test(v0: u32, v1: u32) -> bool {
block0(v0: u32, v1: u32):
    v2: (u32, bool) = intrinsic.sub.overflow(v0, v1)
    v3: bool = field.get v2, 1
    return v3
}"#;
    let output = run_mir_ok(mir, "test", &[Value::uint32(0), Value::uint32(1)]);
    assert_eq!(
        output.value,
        Value::bool(true),
        "expected overflow flag to be true"
    );
}

// saturating arithmetic

#[test]
fn test_intrinsic_sat_add() {
    // i32::MAX + 10 saturates to i32::MAX
    let mir = r#"
function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2: i32 = intrinsic.add.sat(v0, v1)
    return v2
}"#;
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
    v2: u32 = intrinsic.sub.sat(v0, v1)
    return v2
}"#;
    run_mir_expect(
        mir,
        "test",
        &[Value::uint32(0), Value::uint32(10)],
        Value::uint32(0),
    );
}

// atomics

#[test]
fn test_intrinsic_atomic_cas_success_flag() {
    let mir = r#"
function @test() -> bool {
block0:
    v0: ref<raw i32> = raw.alloc i32
    v1: i32 = iconst 10i32
    store v0, v1
    v2: i32 = iconst 10i32
    v3: i32 = iconst 99i32
    v4: (i32, bool) = atomic.cas v0, v2, v3, ordering=relaxed, scope=device, memory_scope=device, semantics=any
    v5: bool = field.get v4, 1
    return v5
}"#;
    run_mir_expect(mir, "test", &[], Value::bool(true));
}

#[test]
fn test_intrinsic_atomic_cas_success_value() {
    let mir = r#"
function @test() -> i32 {
block0:
    v0: ref<raw i32> = raw.alloc i32
    v1: i32 = iconst 10i32
    store v0, v1
    v2: i32 = iconst 10i32
    v3: i32 = iconst 42i32
    v4: (i32, bool) = atomic.cas v0, v2, v3, ordering=relaxed, scope=device, memory_scope=device, semantics=any
    v5: i32 = field.get v4, 0
    return v5
}"#;
    run_mir_expect(mir, "test", &[], Value::int32(10));
}

#[test]
fn test_intrinsic_atomic_cas_failure_flag() {
    let mir = r#"
function @test() -> bool {
block0:
    v0: ref<raw i32> = raw.alloc i32
    v1: i32 = iconst 10i32
    store v0, v1
    v2: i32 = iconst 11i32
    v3: i32 = iconst 99i32
    v4: (i32, bool) = atomic.cas v0, v2, v3, ordering=relaxed, scope=device, memory_scope=device, semantics=any
    v5: bool = field.get v4, 1
    return v5
}"#;
    run_mir_expect(mir, "test", &[], Value::bool(false));
}

#[test]
fn test_intrinsic_atomic_cas_weak_success() {
    let mir = r#"
function @test() -> bool {
block0:
    v0: ref<raw i32> = raw.alloc i32
    v1: i32 = iconst 5i32
    store v0, v1
    v2: i32 = iconst 5i32
    v3: i32 = iconst 6i32
    v4: (i32, bool) = atomic.cas.weak v0, v2, v3, ordering=relaxed, scope=device, memory_scope=device, semantics=any
    v5: bool = field.get v4, 1
    return v5
}"#;
    run_mir_expect(mir, "test", &[], Value::bool(true));
}

#[test]
fn test_intrinsic_atomic_fetch_umin() {
    let mir = r#"
function @test() -> u32 {
block0:
    v0: ref<raw u32> = raw.alloc u32
    v1: u32 = iconst 40u32
    store v0, v1
    v2: u32 = iconst 10u32
    v3: u32 = atomic.rmw.umin v0, v2, ordering=relaxed, scope=device, memory_scope=device, semantics=any
    v4: u32 = load v0
    v5: u32 = iadd v3, v4
    return v5
}"#;
    run_mir_expect(mir, "test", &[], Value::uint32(50));
}

#[test]
fn test_intrinsic_atomic_fetch_umax() {
    let mir = r#"
function @test() -> u32 {
block0:
    v0: ref<raw u32> = raw.alloc u32
    v1: u32 = iconst 12u32
    store v0, v1
    v2: u32 = iconst 20u32
    v3: u32 = atomic.rmw.umax v0, v2, ordering=relaxed, scope=device, memory_scope=device, semantics=any
    v4: u32 = load v0
    v5: u32 = iadd v3, v4
    return v5
}"#;
    run_mir_expect(mir, "test", &[], Value::uint32(32));
}

#[test]
fn test_intrinsic_atomic_fetch_fadd() {
    let mir = r#"
function @test() -> f64 {
block0:
    v0: ref<raw f64> = raw.alloc f64
    v1: f64 = iconst 1.5f64
    store v0, v1
    v2: f64 = iconst 2.25f64
    v3: f64 = atomic.rmw.fadd v0, v2, ordering=relaxed, scope=device, memory_scope=device, semantics=any
    v4: f64 = load v0
    v5: f64 = fadd v3, v4
    return v5
}"#;
    run_mir_expect(mir, "test", &[], Value::float64(5.25));
}

#[test]
fn test_intrinsic_atomic_fetch_fmin() {
    let mir = r#"
function @test() -> f64 {
block0:
    v0: ref<raw f64> = raw.alloc f64
    v1: f64 = iconst 3.5f64
    store v0, v1
    v2: f64 = iconst 1.25f64
    v3: f64 = atomic.rmw.fmin v0, v2, ordering=relaxed, scope=device, memory_scope=device, semantics=any
    v4: f64 = load v0
    v5: f64 = fadd v3, v4
    return v5
}"#;
    run_mir_expect(mir, "test", &[], Value::float64(4.75));
}

#[test]
fn test_intrinsic_atomic_fetch_fmax() {
    let mir = r#"
function @test() -> f64 {
block0:
    v0: ref<raw f64> = raw.alloc f64
    v1: f64 = iconst 3.5f64
    store v0, v1
    v2: f64 = iconst 7.25f64
    v3: f64 = atomic.rmw.fmax v0, v2, ordering=relaxed, scope=device, memory_scope=device, semantics=any
    v4: f64 = load v0
    v5: f64 = fadd v3, v4
    return v5
}"#;
    run_mir_expect(mir, "test", &[], Value::float64(10.75));
}

// unchecked arithmetic

#[test]
fn test_intrinsic_add_unchecked() {
    let mir = r#"
function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2: i32 = intrinsic.add.unchecked(v0, v1)
    return v2
}"#;
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
    v2: i32 = intrinsic.div.unchecked(v0, v1)
    return v2
}"#;
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
    v2: i32 = intrinsic.div.unchecked(v0, v1)
    return v2
}"#;
    let result = run_mir(mir, "test", &[Value::int32(100), Value::int32(0)]);
    assert!(result.is_err(), "expected division by zero error");
}

// float math

#[test]
fn test_intrinsic_sqrt() {
    let mir = r#"
function @test(v0: f64) -> f64 {
block0(v0: f64):
    v1: f64 = intrinsic.sqrt(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Value::float64(16.0)], Value::float64(4.0));
}

#[test]
fn test_intrinsic_abs() {
    let mir = r#"
function @test(v0: f64) -> f64 {
block0(v0: f64):
    v1: f64 = intrinsic.abs(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Value::float64(-42.5)], Value::float64(42.5));
}

#[test]
fn test_intrinsic_floor() {
    let mir = r#"
function @test(v0: f64) -> f64 {
block0(v0: f64):
    v1: f64 = intrinsic.floor(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Value::float64(3.7)], Value::float64(3.0));
}

#[test]
fn test_intrinsic_ceil() {
    let mir = r#"
function @test(v0: f64) -> f64 {
block0(v0: f64):
    v1: f64 = intrinsic.ceil(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Value::float64(3.2)], Value::float64(4.0));
}

#[test]
fn test_intrinsic_round() {
    let mir = r#"
function @test(v0: f64) -> f64 {
block0(v0: f64):
    v1: f64 = intrinsic.round(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Value::float64(3.5)], Value::float64(4.0));
}

#[test]
fn test_intrinsic_min() {
    let mir = r#"
function @test(v0: f64, v1: f64) -> f64 {
block0(v0: f64, v1: f64):
    v2: f64 = intrinsic.min(v0, v1)
    return v2
}"#;
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
    v2: f64 = intrinsic.max(v0, v1)
    return v2
}"#;
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
    v2: f64 = intrinsic.pow(v0, v1)
    return v2
}"#;
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
    v3: f64 = intrinsic.fma(v0, v1, v2)
    return v3
}"#;
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
    v1: f64 = intrinsic.sin(v0)
    v2: f64 = intrinsic.cos(v0)
    v3: f64 = fadd v1, v2
    return v3
}"#;
    // sin(0) = 0, cos(0) = 1, so result = 1
    run_mir_expect(mir, "test", &[Value::float64(0.0)], Value::float64(1.0));
}

// branch hints (passthrough)

#[test]
fn test_intrinsic_expect_true() {
    let mir = r#"
function @test(v0: bool) -> bool {
block0(v0: bool):
    v1: bool = iconst true
    v2: bool = intrinsic.expect(v0, v1)
    return v2
}"#;
    run_mir_expect(mir, "test", &[Value::bool(true)], Value::bool(true));
}

#[test]
fn test_intrinsic_expect_false() {
    let mir = r#"
function @test(v0: bool) -> bool {
block0(v0: bool):
    v1: bool = iconst false
    v2: bool = intrinsic.expect(v0, v1)
    return v2
}"#;
    run_mir_expect(mir, "test", &[Value::bool(false)], Value::bool(false));
}

#[test]
fn test_intrinsic_black_box() {
    let mir = r#"
function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = intrinsic.black_box(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Value::int32(42)], Value::int32(42));
}

// comparison

#[test]
fn test_intrinsic_raw_eq_true() {
    let mir = r#"
function @test(v0: i32, v1: i32) -> bool {
block0(v0: i32, v1: i32):
    v2: i32 = intrinsic.raw_eq(v0, v1)
    return v2
}"#;
    run_mir_expect(
        mir,
        "test",
        &[Value::int32(42), Value::int32(42)],
        Value::bool(true),
    );
}

#[test]
fn test_intrinsic_raw_eq_false() {
    let mir = r#"
function @test(v0: i32, v1: i32) -> bool {
block0(v0: i32, v1: i32):
    v2: i32 = intrinsic.raw_eq(v0, v1)
    return v2
}"#;
    run_mir_expect(
        mir,
        "test",
        &[Value::int32(42), Value::int32(43)],
        Value::bool(false),
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
}"#;
    run_mir_expect(mir, "test", &[Value::int32(42)], Value::int32(42));
}

#[test]
fn test_terminator_unreachable() {
    let mir = r#"
function @test(v0: i32) -> i32 {
block0(v0: i32):
    unreachable
}"#;
    let result = run_mir(mir, "test", &[Value::int32(42)]);
    assert!(result.is_err(), "expected unreachable error");
}

#[test]
fn test_terminator_trap_abort() {
    let mir = r#"
function @test(v0: i32) -> i32 {
block0(v0: i32):
    trap abort
}"#;
    let result = run_mir(mir, "test", &[Value::int32(42)]);
    assert!(result.is_err(), "expected abort error");
}

#[test]
fn test_terminator_trap_panic() {
    let mir = [
        STRING_TYPE_ALIAS,
        r#"
global @message: ref<managed readonly @String> = "boom" ; readonly

function @test() -> void {
block0:
    v0: ref<managed readonly @String> = global.const @message
    trap panic v0
}
"#,
    ]
    .concat();
    let result = run_mir(&mir, "test", &[]);
    assert!(result.is_err(), "expected panic error");
}

// transmute

#[test]
fn test_intrinsic_transmute() {
    // transmute just returns the same bits
    let mir = r#"
function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = intrinsic.transmute(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Value::int32(42)], Value::int32(42));
}

// runtime introspection

#[test]
fn test_intrinsic_return_address() {
    // return_address returns a synthetic address when there's a caller
    let mir = r#"
function @inner() -> u64 {
block0:
    v0: u64 = intrinsic.return_address()
    return v0
}

function @test() -> u64 {
block0:
    v0: u64 = call @inner()
    return v0
}"#;
    // should return non-zero since there's a caller
    let output = run_mir_ok(mir, "test", &[]);
    let (value, width) = output.value.as_uint_with_width().expect("expected UInt");
    assert_eq!(width, 64);
    assert!(value != 0, "expected non-zero return address");
}

#[test]
fn test_intrinsic_return_address_no_caller() {
    // return_address returns 0 when there's no caller
    let mir = r#"
function @test() -> u64 {
block0:
    v0: u64 = intrinsic.return_address()
    return v0
}"#;
    run_mir_expect(mir, "test", &[], Value::uint64(0));
}

#[test]
fn test_intrinsic_frame_address() {
    // frame_address returns a synthetic address based on call depth
    let mir = r#"
function @inner() -> u64 {
block0:
    v0: u64 = intrinsic.frame_address()
    return v0
}

function @test() -> u64 {
block0:
    v0: u64 = call @inner()
    return v0
}"#;
    let output = run_mir_ok(mir, "test", &[]);
    let (value, width) = output.value.as_uint_with_width().expect("expected UInt");
    assert_eq!(width, 64);
    // should have high bits set (0x7FFF_0000_0000_0000) plus frame index
    assert!(
        value > 0x7FFF_0000_0000_0000u64,
        "expected synthetic frame address"
    );
}

// SIMD horizontal reductions

#[test]
fn test_intrinsic_reduce_add() {
    let mir = r#"
function @test(v0: vector<i32, 4>) -> i32 {
block0(v0: vector<i32, 4>):
    v1: i32 = vector.reduce add, v0
    return v1
}"#;
    let output = run_mir_with_ok(mir, "test", |interp| {
        let ty = interp.parameter_type("test", 0);
        let input = interp.materialize_value_for_type(
            ty,
            vec![
                Value::int32(1),
                Value::int32(2),
                Value::int32(3),
                Value::int32(4),
            ],
        );
        vec![input]
    });
    assert_eq!(output.value, Value::int32(10));
}

#[test]
fn test_intrinsic_reduce_mul() {
    let mir = r#"
function @test(v0: vector<i32, 4>) -> i32 {
block0(v0: vector<i32, 4>):
    v1: i32 = vector.reduce mul, v0
    return v1
}"#;
    let output = run_mir_with_ok(mir, "test", |interp| {
        let ty = interp.parameter_type("test", 0);
        let input = interp.materialize_value_for_type(
            ty,
            vec![
                Value::int32(2),
                Value::int32(3),
                Value::int32(4),
                Value::int32(5),
            ],
        );
        vec![input]
    });
    assert_eq!(output.value, Value::int32(120));
}

#[test]
fn test_intrinsic_reduce_min() {
    let mir = r#"
function @test(v0: vector<i32, 4>) -> i32 {
block0(v0: vector<i32, 4>):
    v1: i32 = vector.reduce min, v0
    return v1
}"#;
    let output = run_mir_with_ok(mir, "test", |interp| {
        let ty = interp.parameter_type("test", 0);
        let input = interp.materialize_value_for_type(
            ty,
            vec![
                Value::int32(5),
                Value::int32(2),
                Value::int32(8),
                Value::int32(1),
            ],
        );
        vec![input]
    });
    assert_eq!(output.value, Value::int32(1));
}

#[test]
fn test_intrinsic_reduce_max() {
    let mir = r#"
function @test(v0: vector<i32, 4>) -> i32 {
block0(v0: vector<i32, 4>):
    v1: i32 = vector.reduce max, v0
    return v1
}"#;
    let output = run_mir_with_ok(mir, "test", |interp| {
        let ty = interp.parameter_type("test", 0);
        let input = interp.materialize_value_for_type(
            ty,
            vec![
                Value::int32(5),
                Value::int32(2),
                Value::int32(8),
                Value::int32(1),
            ],
        );
        vec![input]
    });
    assert_eq!(output.value, Value::int32(8));
}

#[test]
fn test_intrinsic_reduce_and() {
    let mir = r#"
function @test(v0: vector<u32, 4>) -> u32 {
block0(v0: vector<u32, 4>):
    v1: u32 = vector.reduce and, v0
    return v1
}"#;
    let output = run_mir_with_ok(mir, "test", |interp| {
        let ty = interp.parameter_type("test", 0);
        let input = interp.materialize_value_for_type(
            ty,
            vec![
                Value::uint32(0b1111),
                Value::uint32(0b1110),
                Value::uint32(0b1100),
                Value::uint32(0b1000),
            ],
        );
        vec![input]
    });
    assert_eq!(output.value, Value::uint32(0b1000));
}

#[test]
fn test_intrinsic_reduce_or() {
    let mir = r#"
function @test(v0: vector<u32, 4>) -> u32 {
block0(v0: vector<u32, 4>):
    v1: u32 = vector.reduce or, v0
    return v1
}"#;
    let output = run_mir_with_ok(mir, "test", |interp| {
        let ty = interp.parameter_type("test", 0);
        let input = interp.materialize_value_for_type(
            ty,
            vec![
                Value::uint32(0b0001),
                Value::uint32(0b0010),
                Value::uint32(0b0100),
                Value::uint32(0b1000),
            ],
        );
        vec![input]
    });
    assert_eq!(output.value, Value::uint32(0b1111));
}

#[test]
fn test_intrinsic_reduce_xor() {
    let mir = r#"
function @test(v0: vector<u32, 4>) -> u32 {
block0(v0: vector<u32, 4>):
    v1: u32 = vector.reduce xor, v0
    return v1
}"#;
    // 1 ^ 2 ^ 3 ^ 4 = 4
    let output = run_mir_with_ok(mir, "test", |interp| {
        let ty = interp.parameter_type("test", 0);
        let input = interp.materialize_value_for_type(
            ty,
            vec![
                Value::uint32(1),
                Value::uint32(2),
                Value::uint32(3),
                Value::uint32(4),
            ],
        );
        vec![input]
    });
    assert_eq!(output.value, Value::uint32(4));
}
