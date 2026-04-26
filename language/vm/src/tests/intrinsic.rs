//! Tests for intrinsic execution.

use crate::Word;
use crate::diagnostic::Error;
use crate::tests::{
    assert_runtime_error_matches, assert_value_word, run_mir, run_mir_expect, run_mir_ok,
    run_mir_with, run_mir_with_ok,
};
use destack_heap::Payload;
use destack_mir as mir;

// bit manipulation

#[test]
fn test_intrinsic_clz() {
    // count leading zeros: 0x00800000 has 8 leading zeros in 32-bit
    let mir = r#"
function test(v0: uint32): uint32 {
b0(v0: uint32):
    v1: uint32 = intrinsic.leadingZeroCount(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Word::uint32(0x00800000)], Word::uint32(8));
}

#[test]
fn test_intrinsic_ctz() {
    // count trailing zeros: 0x80 = 128 = 0b10000000 has 7 trailing zeros
    let mir = r#"
function test(v0: uint32): uint32 {
b0(v0: uint32):
    v1: uint32 = intrinsic.trailingZeroCount(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Word::uint32(0x80)], Word::uint32(7));
}

#[test]
fn test_intrinsic_popcnt() {
    // population count: 0xFF = 255 = 0b11111111 has 8 bits set
    let mir = r#"
function test(v0: uint32): uint32 {
b0(v0: uint32):
    v1: uint32 = intrinsic.populationCount(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Word::uint32(0xFF)], Word::uint32(8));
}

#[test]
fn test_intrinsic_byte_swap() {
    // byte swap: 0x12345678 -> 0x78563412
    let mir = r#"
function test(v0: uint32): uint32 {
b0(v0: uint32):
    v1: uint32 = intrinsic.byteSwap(v0)
    return v1
}"#;
    run_mir_expect(
        mir,
        "test",
        &[Word::uint32(0x12345678)],
        Word::uint32(0x78563412),
    );
}

#[test]
fn test_intrinsic_rotate_left() {
    // rotate left: 0x80000001 rotated left by 1 = 0x00000003
    let mir = r#"
function test(v0: uint32, v1: uint32): uint32 {
b0(v0: uint32, v1: uint32):
    v2: uint32 = intrinsic.rotateLeft(v0, v1)
    return v2
}"#;
    run_mir_expect(
        mir,
        "test",
        &[Word::uint32(0x80000001), Word::uint32(1)],
        Word::uint32(0x00000003),
    );
}

#[test]
fn test_intrinsic_rotate_right() {
    // rotate right: 0x00000003 rotated right by 1 = 0x80000001
    let mir = r#"
function test(v0: uint32, v1: uint32): uint32 {
b0(v0: uint32, v1: uint32):
    v2: uint32 = intrinsic.rotateRight(v0, v1)
    return v2
}"#;
    run_mir_expect(
        mir,
        "test",
        &[Word::uint32(0x00000003), Word::uint32(1)],
        Word::uint32(0x80000001),
    );
}

// checked arithmetic

#[test]
fn test_intrinsic_add_overflow_no_overflow() {
    // 10 + 20 = 30, no overflow - test result value
    let mir = r#"
function testResult(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: (int32, boolean) = intrinsic.add.overflow(v0, v1)
    v3: int32 = field.get v2, 0
    return v3
}"#;
    let output = run_mir_ok(mir, "testResult", &[Word::int32(10), Word::int32(20)]);
    assert_eq!(assert_value_word(&output.value), Word::int32(30));

    // test no overflow flag
    let mir = r#"
function testFlag(v0: int32, v1: int32): boolean {
b0(v0: int32, v1: int32):
    v2: (int32, boolean) = intrinsic.add.overflow(v0, v1)
    v3: boolean = field.get v2, 1
    return v3
}"#;
    let output = run_mir_ok(mir, "testFlag", &[Word::int32(10), Word::int32(20)]);
    assert_eq!(assert_value_word(&output.value), Word::bool(false));
}

#[test]
fn test_intrinsic_add_overflow_with_overflow() {
    // i32::MAX + 1 overflows - test overflow flag
    let mir = r#"
function test(v0: int32, v1: int32): boolean {
b0(v0: int32, v1: int32):
    v2: (int32, boolean) = intrinsic.add.overflow(v0, v1)
    v3: boolean = field.get v2, 1
    return v3
}"#;
    let output = run_mir_ok(mir, "test", &[Word::int32(i32::MAX), Word::int32(1)]);
    assert_eq!(
        assert_value_word(&output.value),
        Word::bool(true),
        "expected overflow flag to be true"
    );
}

#[test]
fn test_intrinsic_sub_overflow() {
    // 0 - 1 for unsigned overflows - test overflow flag
    let mir = r#"
function test(v0: uint32, v1: uint32): boolean {
b0(v0: uint32, v1: uint32):
    v2: (uint32, boolean) = intrinsic.sub.overflow(v0, v1)
    v3: boolean = field.get v2, 1
    return v3
}"#;
    let output = run_mir_ok(mir, "test", &[Word::uint32(0), Word::uint32(1)]);
    assert_eq!(
        assert_value_word(&output.value),
        Word::bool(true),
        "expected overflow flag to be true"
    );
}

// saturating arithmetic

#[test]
fn test_intrinsic_sat_add() {
    // i32::MAX + 10 saturates to i32::MAX
    let mir = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = intrinsic.add.sat(v0, v1)
    return v2
}"#;
    run_mir_expect(
        mir,
        "test",
        &[Word::int32(i32::MAX), Word::int32(10)],
        Word::int32(i32::MAX),
    );
}

#[test]
fn test_intrinsic_sat_sub() {
    // 0u32 - 10 saturates to 0
    let mir = r#"
function test(v0: uint32, v1: uint32): uint32 {
b0(v0: uint32, v1: uint32):
    v2: uint32 = intrinsic.sub.sat(v0, v1)
    return v2
}"#;
    run_mir_expect(
        mir,
        "test",
        &[Word::uint32(0), Word::uint32(10)],
        Word::uint32(0),
    );
}

// atomics

#[test]
fn test_intrinsic_atomic_cas_success_flag() {
    let mir = r#"
function test(): boolean {
b0:
    v0: ref<int32, raw> = raw.alloc int32
    v1: int32 = 10int32
    store v0, v1
    v2: int32 = 10int32
    v3: int32 = 99int32
    v4: (int32, boolean) = atomic.cas v0, v2, v3, relaxed, device, device, any
    v5: boolean = field.get v4, 1
    return v5
}"#;
    run_mir_expect(mir, "test", &[], Word::bool(true));
}

#[test]
fn test_intrinsic_atomic_cas_success_value() {
    let mir = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw> = raw.alloc int32
    v1: int32 = 10int32
    store v0, v1
    v2: int32 = 10int32
    v3: int32 = 42int32
    v4: (int32, boolean) = atomic.cas v0, v2, v3, relaxed, device, device, any
    v5: int32 = field.get v4, 0
    return v5
}"#;
    run_mir_expect(mir, "test", &[], Word::int32(10));
}

#[test]
fn test_intrinsic_atomic_cas_failure_flag() {
    let mir = r#"
function test(): boolean {
b0:
    v0: ref<int32, raw> = raw.alloc int32
    v1: int32 = 10int32
    store v0, v1
    v2: int32 = 11int32
    v3: int32 = 99int32
    v4: (int32, boolean) = atomic.cas v0, v2, v3, relaxed, device, device, any
    v5: boolean = field.get v4, 1
    return v5
}"#;
    run_mir_expect(mir, "test", &[], Word::bool(false));
}

#[test]
fn test_intrinsic_atomic_cas_weak_success() {
    let mir = r#"
function test(): boolean {
b0:
    v0: ref<int32, raw> = raw.alloc int32
    v1: int32 = 5int32
    store v0, v1
    v2: int32 = 5int32
    v3: int32 = 6int32
    v4: (int32, boolean) = atomic.cas.weak v0, v2, v3, relaxed, device, device, any
    v5: boolean = field.get v4, 1
    return v5
}"#;
    run_mir_expect(mir, "test", &[], Word::bool(true));
}

#[test]
fn test_intrinsic_atomic_fetch_umin() {
    let mir = r#"
function test(): uint32 {
b0:
    v0: ref<uint32, raw> = raw.alloc uint32
    v1: uint32 = 40uint32
    store v0, v1
    v2: uint32 = 10uint32
    v3: uint32 = atomic.rmw.umin v0, v2, relaxed, device, device, any
    v4: uint32 = load v0
    v5: uint32 = int.add v3, v4
    return v5
}"#;
    run_mir_expect(mir, "test", &[], Word::uint32(50));
}

#[test]
fn test_intrinsic_atomic_fetch_umax() {
    let mir = r#"
function test(): uint32 {
b0:
    v0: ref<uint32, raw> = raw.alloc uint32
    v1: uint32 = 12uint32
    store v0, v1
    v2: uint32 = 20uint32
    v3: uint32 = atomic.rmw.umax v0, v2, relaxed, device, device, any
    v4: uint32 = load v0
    v5: uint32 = int.add v3, v4
    return v5
}"#;
    run_mir_expect(mir, "test", &[], Word::uint32(32));
}

#[test]
fn test_intrinsic_atomic_fetch_fadd() {
    let mir = r#"
function test(): float64 {
b0:
    v0: ref<float64, raw> = raw.alloc float64
    v1: float64 = 1.5float64
    store v0, v1
    v2: float64 = 2.25float64
    v3: float64 = atomic.rmw.fadd v0, v2, relaxed, device, device, any
    v4: float64 = load v0
    v5: float64 = float.add v3, v4
    return v5
}"#;
    run_mir_expect(mir, "test", &[], Word::float64(5.25));
}

#[test]
fn test_intrinsic_atomic_fetch_fmin() {
    let mir = r#"
function test(): float64 {
b0:
    v0: ref<float64, raw> = raw.alloc float64
    v1: float64 = 3.5float64
    store v0, v1
    v2: float64 = 1.25float64
    v3: float64 = atomic.rmw.fmin v0, v2, relaxed, device, device, any
    v4: float64 = load v0
    v5: float64 = float.add v3, v4
    return v5
}"#;
    run_mir_expect(mir, "test", &[], Word::float64(4.75));
}

#[test]
fn test_intrinsic_atomic_fetch_fmax() {
    let mir = r#"
function test(): float64 {
b0:
    v0: ref<float64, raw> = raw.alloc float64
    v1: float64 = 3.5float64
    store v0, v1
    v2: float64 = 7.25float64
    v3: float64 = atomic.rmw.fmax v0, v2, relaxed, device, device, any
    v4: float64 = load v0
    v5: float64 = float.add v3, v4
    return v5
}"#;
    run_mir_expect(mir, "test", &[], Word::float64(10.75));
}

// unchecked arithmetic

#[test]
fn test_intrinsic_add_unchecked() {
    let mir = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = intrinsic.add.unchecked(v0, v1)
    return v2
}"#;
    run_mir_expect(
        mir,
        "test",
        &[Word::int32(100), Word::int32(200)],
        Word::int32(300),
    );
}

#[test]
fn test_intrinsic_div_unchecked() {
    let mir = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = intrinsic.div.unchecked(v0, v1)
    return v2
}"#;
    run_mir_expect(
        mir,
        "test",
        &[Word::int32(100), Word::int32(10)],
        Word::int32(10),
    );
}

#[test]
fn test_intrinsic_div_by_zero_unchecked() {
    let mir = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = intrinsic.div.unchecked(v0, v1)
    return v2
}"#;
    let result = run_mir(mir, "test", &[Word::int32(100), Word::int32(0)]);
    assert!(result.is_err(), "expected division by zero error");
}

// float math

#[test]
fn test_intrinsic_sqrt() {
    let mir = r#"
function test(v0: float64): float64 {
b0(v0: float64):
    v1: float64 = intrinsic.sqrt(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Word::float64(16.0)], Word::float64(4.0));
}

#[test]
fn test_intrinsic_abs() {
    let mir = r#"
function test(v0: float64): float64 {
b0(v0: float64):
    v1: float64 = intrinsic.abs(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Word::float64(-42.5)], Word::float64(42.5));
}

#[test]
fn test_intrinsic_floor() {
    let mir = r#"
function test(v0: float64): float64 {
b0(v0: float64):
    v1: float64 = intrinsic.floor(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Word::float64(3.7)], Word::float64(3.0));
}

#[test]
fn test_intrinsic_ceil() {
    let mir = r#"
function test(v0: float64): float64 {
b0(v0: float64):
    v1: float64 = intrinsic.ceil(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Word::float64(3.2)], Word::float64(4.0));
}

#[test]
fn test_intrinsic_round() {
    let mir = r#"
function test(v0: float64): float64 {
b0(v0: float64):
    v1: float64 = intrinsic.round(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Word::float64(3.5)], Word::float64(4.0));
}

#[test]
fn test_intrinsic_min() {
    let mir = r#"
function test(v0: float64, v1: float64): float64 {
b0(v0: float64, v1: float64):
    v2: float64 = intrinsic.min(v0, v1)
    return v2
}"#;
    run_mir_expect(
        mir,
        "test",
        &[Word::float64(10.0), Word::float64(5.0)],
        Word::float64(5.0),
    );
}

#[test]
fn test_intrinsic_max() {
    let mir = r#"
function test(v0: float64, v1: float64): float64 {
b0(v0: float64, v1: float64):
    v2: float64 = intrinsic.max(v0, v1)
    return v2
}"#;
    run_mir_expect(
        mir,
        "test",
        &[Word::float64(10.0), Word::float64(5.0)],
        Word::float64(10.0),
    );
}

#[test]
fn test_intrinsic_pow() {
    let mir = r#"
function test(v0: float64, v1: float64): float64 {
b0(v0: float64, v1: float64):
    v2: float64 = intrinsic.pow(v0, v1)
    return v2
}"#;
    run_mir_expect(
        mir,
        "test",
        &[Word::float64(2.0), Word::float64(10.0)],
        Word::float64(1024.0),
    );
}

#[test]
fn test_intrinsic_fma() {
    // fma(2.0, 3.0, 4.0) = 2.0 * 3.0 + 4.0 = 10.0
    let mir = r#"
function test(v0: float64, v1: float64, v2: float64): float64 {
b0(v0: float64, v1: float64, v2: float64):
    v3: float64 = intrinsic.fma(v0, v1, v2)
    return v3
}"#;
    run_mir_expect(
        mir,
        "test",
        &[Word::float64(2.0), Word::float64(3.0), Word::float64(4.0)],
        Word::float64(10.0),
    );
}

// trigonometry (just test they execute without error)

#[test]
fn test_intrinsic_sin_cos() {
    let mir = r#"
function test(v0: float64): float64 {
b0(v0: float64):
    v1: float64 = intrinsic.sin(v0)
    v2: float64 = intrinsic.cos(v0)
    v3: float64 = float.add v1, v2
    return v3
}"#;
    // sin(0) = 0, cos(0) = 1, so result = 1
    run_mir_expect(mir, "test", &[Word::float64(0.0)], Word::float64(1.0));
}

// branch hints (passthrough)

#[test]
fn test_intrinsic_expect_true() {
    let mir = r#"
function test(v0: boolean): boolean {
b0(v0: boolean):
    v1: boolean = true
    v2: boolean = intrinsic.expect(v0, v1)
    return v2
}"#;
    run_mir_expect(mir, "test", &[Word::bool(true)], Word::bool(true));
}

#[test]
fn test_intrinsic_expect_false() {
    let mir = r#"
function test(v0: boolean): boolean {
b0(v0: boolean):
    v1: boolean = false
    v2: boolean = intrinsic.expect(v0, v1)
    return v2
}"#;
    run_mir_expect(mir, "test", &[Word::bool(false)], Word::bool(false));
}

#[test]
fn test_intrinsic_black_box() {
    let mir = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = intrinsic.blackBox(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Word::int32(42)], Word::int32(42));
}

#[test]
fn test_intrinsic_write_barrier_local_managed() {
    let mir = r#"
function test(v0: ref<int32, managed, readonly>, v1: uint64, v2: uint64): boolean {
b0(v0: ref<int32, managed, readonly>, v1: uint64, v2: uint64):
    intrinsic.writeBarrier(v0, v1, v2)
    v3: boolean = true
    return v3
}"#;
    run_mir_with(mir, "test", |isolate| {
        let reference_type = isolate.parameter_type("test", 0);
        let pointee_type = match isolate.isolate.tree().get(reference_type) {
            mir::Type::Reference { pointee, .. } => pointee
                .ty()
                .expect("test parameter pointee should be concrete"),
            _ => panic!("test parameter should be one heap reference"),
        };
        let layout_id = isolate
            .isolate
            .layout_id_for_type(pointee_type)
            .expect("managed pointee should have one layout");
        let layout = isolate
            .isolate
            .allocation_layout(layout_id)
            .expect("managed pointee layout should resolve");
        let handle = isolate
            .heap
            .allocate(layout, Payload::Zeroed)
            .expect("heap allocation should succeed");

        vec![
            Word::heap_reference(handle),
            Word::uint64(0),
            Word::uint64(4),
        ]
    })
    .expect("write barrier should succeed");
}

#[test]
fn test_intrinsic_write_barrier_shared_managed() {
    let mir = r#"
function test(v0: ref<int32, managed, readonly, space(shared)>, v1: uint64, v2: uint64): boolean {
b0(v0: ref<int32, managed, readonly, space(shared)>, v1: uint64, v2: uint64):
    intrinsic.writeBarrier(v0, v1, v2)
    v3: boolean = true
    return v3
}"#;
    let output = run_mir_with_ok(mir, "test", |isolate| {
        let reference_type = isolate.parameter_type("test", 0);
        let pointee_type = match isolate.isolate.tree().get(reference_type) {
            mir::Type::Reference { pointee, .. } => pointee
                .ty()
                .expect("test parameter pointee should be concrete"),
            _ => panic!("test parameter should be one shared heap reference"),
        };
        let layout_id = isolate
            .isolate
            .layout_id_for_type(pointee_type)
            .expect("shared managed pointee should have one layout");
        let layout = isolate
            .isolate
            .allocation_layout(layout_id)
            .expect("shared managed pointee layout should resolve");
        let mut allocator = isolate.shared.allocator();
        let handle = isolate
            .shared
            .allocate(&mut allocator, layout, Payload::Zeroed)
            .expect("shared heap allocation should succeed");

        vec![
            Word::shared_heap_reference(handle),
            Word::uint64(0),
            Word::uint64(4),
        ]
    });

    let value = assert_value_word(&output.value);
    assert_eq!(value, Word::bool(true));
}

#[test]
fn test_intrinsic_write_barrier_rejects_invalid_range() {
    let mir = r#"
function test(v0: ref<int32, managed, readonly>, v1: uint64, v2: uint64): void {
b0(v0: ref<int32, managed, readonly>, v1: uint64, v2: uint64):
    intrinsic.writeBarrier(v0, v1, v2)
    return
}"#;
    let result = run_mir_with(mir, "test", |isolate| {
        let reference_type = isolate.parameter_type("test", 0);
        let pointee_type = match isolate.isolate.tree().get(reference_type) {
            mir::Type::Reference { pointee, .. } => pointee
                .ty()
                .expect("test parameter pointee should be concrete"),
            _ => panic!("test parameter should be one heap reference"),
        };
        let layout_id = isolate
            .isolate
            .layout_id_for_type(pointee_type)
            .expect("managed pointee should have one layout");
        let layout = isolate
            .isolate
            .allocation_layout(layout_id)
            .expect("managed pointee layout should resolve");
        let handle = isolate
            .heap
            .allocate(layout, Payload::Zeroed)
            .expect("heap allocation should succeed");

        vec![
            Word::heap_reference(handle),
            Word::uint64(4),
            Word::uint64(1),
        ]
    });

    assert_runtime_error_matches!(
        result,
        Error::Panic { ref message } if message.contains("invalid heap byte range")
    );
}

// comparison

#[test]
fn test_intrinsic_raw_eq_true() {
    let mir = r#"
function test(v0: int32, v1: int32): boolean {
b0(v0: int32, v1: int32):
    v2: int32 = intrinsic.rawEq(v0, v1)
    return v2
}"#;
    run_mir_expect(
        mir,
        "test",
        &[Word::int32(42), Word::int32(42)],
        Word::bool(true),
    );
}

#[test]
fn test_intrinsic_raw_eq_false() {
    let mir = r#"
function test(v0: int32, v1: int32): boolean {
b0(v0: int32, v1: int32):
    v2: int32 = intrinsic.rawEq(v0, v1)
    return v2
}"#;
    run_mir_expect(
        mir,
        "test",
        &[Word::int32(42), Word::int32(43)],
        Word::bool(false),
    );
}

// control flow

#[test]
fn test_intrinsic_breakpoint() {
    // breakpoint should be a no-op
    let mir = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    intrinsic.breakpoint()
    return v0
}"#;
    run_mir_expect(mir, "test", &[Word::int32(42)], Word::int32(42));
}

#[test]
fn test_terminator_unreachable() {
    let mir = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    unreachable
}"#;
    let result = run_mir(mir, "test", &[Word::int32(42)]);
    assert!(result.is_err(), "expected unreachable error");
}

#[test]
fn test_terminator_trap_abort() {
    let mir = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    trap.abort
}"#;
    let result = run_mir(mir, "test", &[Word::int32(42)]);
    assert!(result.is_err(), "expected abort error");
}

#[test]
fn test_terminator_trap_panic() {
    let mir = r#"
function test(): void {
b0:
    v0: ref<int32, managed, readonly> = new int32
    trap.panic v0
}
"#;
    let result = run_mir(mir, "test", &[]);
    assert!(result.is_err(), "expected panic error");
}

// transmute

#[test]
fn test_intrinsic_transmute() {
    // transmute just returns the same bits
    let mir = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = intrinsic.transmute(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Word::int32(42)], Word::int32(42));
}

// runtime introspection

#[test]
fn test_intrinsic_return_address() {
    // return_address returns a synthetic address when there's a caller
    let mir = r#"
function inner(): uint64 {
b0:
    v0: uint64 = intrinsic.returnAddress()
    return v0
}
function test(): uint64 {
b0:
    v0: uint64 = call inner(): () -> uint64
    return v0
}"#;
    // should return non-zero since there's a caller
    let output = run_mir_ok(mir, "test", &[]);
    let value = assert_value_word(&output.value).as_uint();
    assert!(value != 0, "expected non-zero return address");
}

#[test]
fn test_intrinsic_return_address_no_caller() {
    // return_address returns 0 when there's no caller
    let mir = r#"
function test(): uint64 {
b0:
    v0: uint64 = intrinsic.returnAddress()
    return v0
}"#;
    run_mir_expect(mir, "test", &[], Word::uint64(0));
}

#[test]
fn test_intrinsic_frame_address() {
    // frame_address returns a synthetic address based on call depth
    let mir = r#"
function inner(): uint64 {
b0:
    v0: uint64 = intrinsic.frameAddress()
    return v0
}
function test(): uint64 {
b0:
    v0: uint64 = call inner(): () -> uint64
    return v0
}"#;
    let output = run_mir_ok(mir, "test", &[]);
    let value = assert_value_word(&output.value).as_uint();
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
function test(v0: vector<int32, 4>): int32 {
b0(v0: vector<int32, 4>):
    v1: int32 = vector.reduce add, v0
    return v1
}"#;
    let output = run_mir_with_ok(mir, "test", |interp| {
        let ty = interp.parameter_type("test", 0);
        let input = interp.materialize_value_for_type(
            ty,
            vec![
                Word::int32(1),
                Word::int32(2),
                Word::int32(3),
                Word::int32(4),
            ],
        );
        vec![input]
    });
    assert_eq!(assert_value_word(&output.value), Word::int32(10));
}

#[test]
fn test_intrinsic_reduce_mul() {
    let mir = r#"
function test(v0: vector<int32, 4>): int32 {
b0(v0: vector<int32, 4>):
    v1: int32 = vector.reduce mul, v0
    return v1
}"#;
    let output = run_mir_with_ok(mir, "test", |interp| {
        let ty = interp.parameter_type("test", 0);
        let input = interp.materialize_value_for_type(
            ty,
            vec![
                Word::int32(2),
                Word::int32(3),
                Word::int32(4),
                Word::int32(5),
            ],
        );
        vec![input]
    });
    assert_eq!(assert_value_word(&output.value), Word::int32(120));
}

#[test]
fn test_intrinsic_reduce_min() {
    let mir = r#"
function test(v0: vector<int32, 4>): int32 {
b0(v0: vector<int32, 4>):
    v1: int32 = vector.reduce min, v0
    return v1
}"#;
    let output = run_mir_with_ok(mir, "test", |interp| {
        let ty = interp.parameter_type("test", 0);
        let input = interp.materialize_value_for_type(
            ty,
            vec![
                Word::int32(5),
                Word::int32(2),
                Word::int32(8),
                Word::int32(1),
            ],
        );
        vec![input]
    });
    assert_eq!(assert_value_word(&output.value), Word::int32(1));
}

#[test]
fn test_intrinsic_reduce_max() {
    let mir = r#"
function test(v0: vector<int32, 4>): int32 {
b0(v0: vector<int32, 4>):
    v1: int32 = vector.reduce max, v0
    return v1
}"#;
    let output = run_mir_with_ok(mir, "test", |interp| {
        let ty = interp.parameter_type("test", 0);
        let input = interp.materialize_value_for_type(
            ty,
            vec![
                Word::int32(5),
                Word::int32(2),
                Word::int32(8),
                Word::int32(1),
            ],
        );
        vec![input]
    });
    assert_eq!(assert_value_word(&output.value), Word::int32(8));
}

#[test]
fn test_intrinsic_reduce_and() {
    let mir = r#"
function test(v0: vector<uint32, 4>): uint32 {
b0(v0: vector<uint32, 4>):
    v1: uint32 = vector.reduce and, v0
    return v1
}"#;
    let output = run_mir_with_ok(mir, "test", |interp| {
        let ty = interp.parameter_type("test", 0);
        let input = interp.materialize_value_for_type(
            ty,
            vec![
                Word::uint32(0b1111),
                Word::uint32(0b1110),
                Word::uint32(0b1100),
                Word::uint32(0b1000),
            ],
        );
        vec![input]
    });
    assert_eq!(assert_value_word(&output.value), Word::uint32(0b1000));
}

#[test]
fn test_intrinsic_reduce_or() {
    let mir = r#"
function test(v0: vector<uint32, 4>): uint32 {
b0(v0: vector<uint32, 4>):
    v1: uint32 = vector.reduce or, v0
    return v1
}"#;
    let output = run_mir_with_ok(mir, "test", |interp| {
        let ty = interp.parameter_type("test", 0);
        let input = interp.materialize_value_for_type(
            ty,
            vec![
                Word::uint32(0b0001),
                Word::uint32(0b0010),
                Word::uint32(0b0100),
                Word::uint32(0b1000),
            ],
        );
        vec![input]
    });
    assert_eq!(assert_value_word(&output.value), Word::uint32(0b1111));
}

#[test]
fn test_intrinsic_reduce_xor() {
    let mir = r#"
function test(v0: vector<uint32, 4>): uint32 {
b0(v0: vector<uint32, 4>):
    v1: uint32 = vector.reduce xor, v0
    return v1
}"#;
    // 1 ^ 2 ^ 3 ^ 4 = 4
    let output = run_mir_with_ok(mir, "test", |interp| {
        let ty = interp.parameter_type("test", 0);
        let input = interp.materialize_value_for_type(
            ty,
            vec![
                Word::uint32(1),
                Word::uint32(2),
                Word::uint32(3),
                Word::uint32(4),
            ],
        );
        vec![input]
    });
    assert_eq!(assert_value_word(&output.value), Word::uint32(4));
}
