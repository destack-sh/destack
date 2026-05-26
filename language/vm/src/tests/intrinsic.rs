use crate::diagnostic::Error;
use crate::tests::{
    assert_runtime_error_matches, run_mir, run_mir_expect, run_mir_ok, run_mir_with_frame,
    run_mir_with_frame_ok,
};
use crate::{Value, Word};
use destack_engine::UnsignedInt;
use destack_heap::Payload;
use destack_mir as mir;

#[test]
fn test_intrinsic_clz() {
    let mir = r#"
function test(v0: uint32): uint32 {
b0(v0: uint32):
    v1: uint32 = intrinsic.math.bits.leadingZeroCount(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Value::uint32(0x00800000)], Value::uint32(8));
}

#[test]
fn test_intrinsic_ctz() {
    let mir = r#"
function test(v0: uint32): uint32 {
b0(v0: uint32):
    v1: uint32 = intrinsic.math.bits.trailingZeroCount(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Value::uint32(0x80)], Value::uint32(7));
}

#[test]
fn test_intrinsic_popcnt() {
    let mir = r#"
function test(v0: uint32): uint32 {
b0(v0: uint32):
    v1: uint32 = intrinsic.math.bits.populationCount(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Value::uint32(0xFF)], Value::uint32(8));
}

#[test]
fn test_intrinsic_byte_swap() {
    let mir = r#"
function test(v0: uint32): uint32 {
b0(v0: uint32):
    v1: uint32 = intrinsic.math.bits.byteSwap(v0)
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
    let mir = r#"
function test(v0: uint32, v1: uint32): uint32 {
b0(v0: uint32, v1: uint32):
    v2: uint32 = intrinsic.math.bits.rotateLeft(v0, v1)
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
    let mir = r#"
function test(v0: uint32, v1: uint32): uint32 {
b0(v0: uint32, v1: uint32):
    v2: uint32 = intrinsic.math.bits.rotateRight(v0, v1)
    return v2
}"#;
    run_mir_expect(
        mir,
        "test",
        &[Value::uint32(0x00000003), Value::uint32(1)],
        Value::uint32(0x80000001),
    );
}

#[test]
fn test_intrinsic_add_overflow_no_overflow() {
    let mir = r#"
function testResult(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: (int32, boolean) = intrinsic.math.arithmetic.overflowing.add(v0, v1)
    v3: int32 = field.get v2, 0
    return v3
}"#;
    let output = run_mir_ok(mir, "testResult", &[Value::int32(10), Value::int32(20)]);
    assert_eq!(output, Value::int32(30));
    let mir = r#"
function testFlag(v0: int32, v1: int32): boolean {
b0(v0: int32, v1: int32):
    v2: (int32, boolean) = intrinsic.math.arithmetic.overflowing.add(v0, v1)
    v3: boolean = field.get v2, 1
    return v3
}"#;
    let output = run_mir_ok(mir, "testFlag", &[Value::int32(10), Value::int32(20)]);
    assert_eq!(output, Value::bool(false));
}

#[test]
fn test_intrinsic_add_overflow_with_overflow() {
    let mir = r#"
function test(v0: int32, v1: int32): boolean {
b0(v0: int32, v1: int32):
    v2: (int32, boolean) = intrinsic.math.arithmetic.overflowing.add(v0, v1)
    v3: boolean = field.get v2, 1
    return v3
}"#;
    let output = run_mir_ok(mir, "test", &[Value::int32(i32::MAX), Value::int32(1)]);
    assert_eq!(output, Value::bool(true));
}

#[test]
fn test_intrinsic_sub_overflow() {
    let mir = r#"
function test(v0: uint32, v1: uint32): boolean {
b0(v0: uint32, v1: uint32):
    v2: (uint32, boolean) = intrinsic.math.arithmetic.overflowing.subtract(v0, v1)
    v3: boolean = field.get v2, 1
    return v3
}"#;
    let output = run_mir_ok(mir, "test", &[Value::uint32(0), Value::uint32(1)]);
    assert_eq!(output, Value::bool(true));
}

#[test]
fn test_intrinsic_sat_add() {
    let mir = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = intrinsic.math.arithmetic.saturating.add(v0, v1)
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
    let mir = r#"
function test(v0: uint32, v1: uint32): uint32 {
b0(v0: uint32, v1: uint32):
    v2: uint32 = intrinsic.math.arithmetic.saturating.subtract(v0, v1)
    return v2
}"#;
    run_mir_expect(
        mir,
        "test",
        &[Value::uint32(0), Value::uint32(10)],
        Value::uint32(0),
    );
}

#[test]
fn test_atomic_cas_success_flag() {
    let mir = r#"
function test(): boolean {
b0:
    v0: ref<atomic<int32>, raw> = raw.alloc atomic<int32>
    v1: int32 = 10int32
    atomic.store v0, v1, relaxed
    v2: int32 = 10int32
    v3: int32 = 99int32
    v4: (int32, boolean) = atomic.cas v0, v2, v3, relaxed
    v5: boolean = field.get v4, 1
    return v5
}"#;
    run_mir_expect(mir, "test", &[], Value::bool(true));
}

#[test]
fn test_atomic_cas_success_value() {
    let mir = r#"
function test(): int32 {
b0:
    v0: ref<atomic<int32>, raw> = raw.alloc atomic<int32>
    v1: int32 = 10int32
    atomic.store v0, v1, relaxed
    v2: int32 = 10int32
    v3: int32 = 42int32
    v4: (int32, boolean) = atomic.cas v0, v2, v3, relaxed
    v5: int32 = field.get v4, 0
    return v5
}"#;
    run_mir_expect(mir, "test", &[], Value::int32(10));
}

#[test]
fn test_atomic_cas_failure_flag() {
    let mir = r#"
function test(): boolean {
b0:
    v0: ref<atomic<int32>, raw> = raw.alloc atomic<int32>
    v1: int32 = 10int32
    atomic.store v0, v1, relaxed
    v2: int32 = 11int32
    v3: int32 = 99int32
    v4: (int32, boolean) = atomic.cas v0, v2, v3, relaxed
    v5: boolean = field.get v4, 1
    return v5
}"#;
    run_mir_expect(mir, "test", &[], Value::bool(false));
}

#[test]
fn test_atomic_cas_weak_success() {
    let mir = r#"
function test(): boolean {
b0:
    v0: ref<atomic<int32>, raw> = raw.alloc atomic<int32>
    v1: int32 = 5int32
    atomic.store v0, v1, relaxed
    v2: int32 = 5int32
    v3: int32 = 6int32
    v4: (int32, boolean) = atomic.cas.weak v0, v2, v3, relaxed
    v5: boolean = field.get v4, 1
    return v5
}"#;
    run_mir_expect(mir, "test", &[], Value::bool(true));
}

#[test]
fn test_atomic_fetch_umin() {
    let mir = r#"
function test(): uint32 {
b0:
    v0: ref<atomic<uint32>, raw> = raw.alloc atomic<uint32>
    v1: uint32 = 40uint32
    atomic.store v0, v1, relaxed
    v2: uint32 = 10uint32
    v3: uint32 = atomic.rmw.umin v0, v2, relaxed
    v4: uint32 = atomic.load v0, relaxed
    v5: uint32 = int.add v3, v4
    return v5
}"#;
    run_mir_expect(mir, "test", &[], Value::uint32(50));
}

#[test]
fn test_atomic_fetch_umax() {
    let mir = r#"
function test(): uint32 {
b0:
    v0: ref<atomic<uint32>, raw> = raw.alloc atomic<uint32>
    v1: uint32 = 12uint32
    atomic.store v0, v1, relaxed
    v2: uint32 = 20uint32
    v3: uint32 = atomic.rmw.umax v0, v2, relaxed
    v4: uint32 = atomic.load v0, relaxed
    v5: uint32 = int.add v3, v4
    return v5
}"#;
    run_mir_expect(mir, "test", &[], Value::uint32(32));
}

#[test]
fn test_atomic_store_load_managed_heap() {
    let mir = r#"
function test(): int32 {
b0:
    v0: ref<atomic<int32>, managed> = new atomic<int32>
    v1: int32 = 42int32
    atomic.store v0, v1, relaxed
    v2: int32 = atomic.load v0, relaxed
    return v2
}"#;
    run_mir_expect(mir, "test", &[], Value::int32(42));
}

#[test]
fn test_atomic_store_load_shared_heap() {
    let mir = r#"
function test(): int32 {
b0:
    v0: ref<atomic<int32>, managed, space(shared)> = new atomic<int32>
    v1: int32 = 37int32
    atomic.store v0, v1, relaxed
    v2: int32 = atomic.load v0, relaxed
    return v2
}"#;
    run_mir_expect(mir, "test", &[], Value::int32(37));
}

#[test]
fn test_atomic_store_load_unique_heap() {
    let mir = r#"
function test(): int32 {
b0:
    v0: ref<atomic<int32>, unique> = new atomic<int32>
    v1: int32 = 43int32
    atomic.store v0, v1, relaxed
    v2: int32 = atomic.load v0, relaxed
    free v0
    return v2
}"#;
    run_mir_expect(mir, "test", &[], Value::int32(43));
}

#[test]
fn test_atomic_store_load_unique_shared_heap() {
    let mir = r#"
function test(): int32 {
b0:
    v0: ref<atomic<int32>, unique, space(shared)> = new atomic<int32>
    v1: int32 = 44int32
    atomic.store v0, v1, relaxed
    v2: int32 = atomic.load v0, relaxed
    free v0
    return v2
}"#;
    run_mir_expect(mir, "test", &[], Value::int32(44));
}

#[test]
fn test_atomic_store_load_shared_raw() {
    let mir = r#"
function test(): int32 {
b0:
    v0: ref<atomic<int32>, raw, space(shared)> = raw.alloc atomic<int32>
    v1: int32 = 45int32
    atomic.store v0, v1, relaxed
    v2: int32 = atomic.load v0, relaxed
    raw.free v0
    return v2
}"#;
    run_mir_expect(mir, "test", &[], Value::int32(45));
}

#[test]
fn test_atomic_fetch_fadd() {
    let mir = r#"
function test(): float64 {
b0:
    v0: ref<atomic<float64>, raw> = raw.alloc atomic<float64>
    v1: float64 = 1.5float64
    atomic.store v0, v1, relaxed
    v2: float64 = 2.25float64
    v3: float64 = atomic.rmw.fadd v0, v2, relaxed
    v4: float64 = atomic.load v0, relaxed
    v5: float64 = float.add v3, v4
    return v5
}"#;
    run_mir_expect(mir, "test", &[], Value::float64(5.25));
}

#[test]
fn test_atomic_fetch_fmin() {
    let mir = r#"
function test(): float64 {
b0:
    v0: ref<atomic<float64>, raw> = raw.alloc atomic<float64>
    v1: float64 = 3.5float64
    atomic.store v0, v1, relaxed
    v2: float64 = 1.25float64
    v3: float64 = atomic.rmw.fmin v0, v2, relaxed
    v4: float64 = atomic.load v0, relaxed
    v5: float64 = float.add v3, v4
    return v5
}"#;
    run_mir_expect(mir, "test", &[], Value::float64(4.75));
}

#[test]
fn test_atomic_fetch_fmax() {
    let mir = r#"
function test(): float64 {
b0:
    v0: ref<atomic<float64>, raw> = raw.alloc atomic<float64>
    v1: float64 = 3.5float64
    atomic.store v0, v1, relaxed
    v2: float64 = 7.25float64
    v3: float64 = atomic.rmw.fmax v0, v2, relaxed
    v4: float64 = atomic.load v0, relaxed
    v5: float64 = float.add v3, v4
    return v5
}"#;
    run_mir_expect(mir, "test", &[], Value::float64(10.75));
}

#[test]
fn test_intrinsic_add_unchecked() {
    let mir = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = intrinsic.math.arithmetic.unchecked.add(v0, v1)
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
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = intrinsic.math.arithmetic.unchecked.divide(v0, v1)
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
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = intrinsic.math.arithmetic.unchecked.divide(v0, v1)
    return v2
}"#;
    let result = run_mir(mir, "test", &[Value::int32(100), Value::int32(0)]);
    assert!(result.is_err(), "expected division by zero error");
}

#[test]
fn test_intrinsic_sqrt() {
    let mir = r#"
function test(v0: float64): float64 {
b0(v0: float64):
    v1: float64 = intrinsic.math.float.sqrt(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Value::float64(16.0)], Value::float64(4.0));
}

#[test]
fn test_intrinsic_abs() {
    let mir = r#"
function test(v0: float64): float64 {
b0(v0: float64):
    v1: float64 = intrinsic.math.float.abs(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Value::float64(-42.5)], Value::float64(42.5));
}

#[test]
fn test_intrinsic_floor() {
    let mir = r#"
function test(v0: float64): float64 {
b0(v0: float64):
    v1: float64 = intrinsic.math.float.floor(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Value::float64(3.7)], Value::float64(3.0));
}

#[test]
fn test_intrinsic_ceil() {
    let mir = r#"
function test(v0: float64): float64 {
b0(v0: float64):
    v1: float64 = intrinsic.math.float.ceil(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Value::float64(3.2)], Value::float64(4.0));
}

#[test]
fn test_intrinsic_round() {
    let mir = r#"
function test(v0: float64): float64 {
b0(v0: float64):
    v1: float64 = intrinsic.math.float.round(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Value::float64(3.5)], Value::float64(4.0));
}

#[test]
fn test_intrinsic_min() {
    let mir = r#"
function test(v0: float64, v1: float64): float64 {
b0(v0: float64, v1: float64):
    v2: float64 = intrinsic.math.float.min(v0, v1)
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
function test(v0: float64, v1: float64): float64 {
b0(v0: float64, v1: float64):
    v2: float64 = intrinsic.math.float.max(v0, v1)
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
function test(v0: float64, v1: float64): float64 {
b0(v0: float64, v1: float64):
    v2: float64 = intrinsic.math.float.pow(v0, v1)
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
    let mir = r#"
function test(v0: float64, v1: float64, v2: float64): float64 {
b0(v0: float64, v1: float64, v2: float64):
    v3: float64 = intrinsic.math.float.fma(v0, v1, v2)
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

#[test]
fn test_intrinsic_sin_cos() {
    let mir = r#"
function test(v0: float64): float64 {
b0(v0: float64):
    v1: float64 = intrinsic.math.float.sin(v0)
    v2: float64 = intrinsic.math.float.cos(v0)
    v3: float64 = float.add v1, v2
    return v3
}"#;
    run_mir_expect(mir, "test", &[Value::float64(0.0)], Value::float64(1.0));
}

#[test]
fn test_intrinsic_expect_true() {
    let mir = r#"
function test(v0: boolean): boolean {
b0(v0: boolean):
    v1: boolean = true
    v2: boolean = intrinsic.expect(v0, v1)
    return v2
}"#;
    run_mir_expect(mir, "test", &[Value::bool(true)], Value::bool(true));
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
    run_mir_expect(mir, "test", &[Value::bool(false)], Value::bool(false));
}

#[test]
fn test_intrinsic_black_box() {
    let mir = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = intrinsic.error.debug.blackBox(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Value::int32(42)], Value::int32(42));
}

#[test]
fn test_barrier_write_local_managed() {
    let mir = r#"
function test(v0: ref<int32, managed, readonly>, v1: uint64, v2: uint64): boolean {
b0(v0: ref<int32, managed, readonly>, v1: uint64, v2: uint64):
    barrier.write v0, v1, v2
    v3: boolean = true
    return v3
}"#;
    run_mir_with_frame(mir, "test", |isolate| {
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
        let shape = isolate
            .isolate
            .allocation_shape(layout_id)
            .expect("managed pointee layout should resolve");
        let layout = isolate.heap.allocation_plan(shape);
        let handle = isolate
            .heap
            .allocate(&layout, Payload::Zeroed)
            .expect("heap allocation should succeed");

        vec![
            Word::heap_reference(handle),
            Word::uint64(0),
            Word::uint64(4),
        ]
    })
    .expect("barrier write should succeed");
}

#[test]
fn test_barrier_write_shared_managed() {
    let mir = r#"
function test(v0: ref<int32, managed, readonly, space(shared)>, v1: uint64, v2: uint64): boolean {
b0(v0: ref<int32, managed, readonly, space(shared)>, v1: uint64, v2: uint64):
    barrier.write v0, v1, v2
    v3: boolean = true
    return v3
}"#;
    run_mir_with_frame(mir, "test", |isolate| {
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
            .expect("managed pointee should have one layout");
        let shape = isolate
            .isolate
            .allocation_shape(layout_id)
            .expect("managed pointee layout should resolve");
        let layout = isolate.shared_heap.allocation_plan(shape);
        let mut allocator = isolate.shared_heap.allocator();
        let handle = isolate
            .shared_heap
            .allocate_zeroed(&isolate.shared_gc, &mut allocator, &layout)
            .expect("shared heap allocation should succeed");
        isolate.shared_heap.flush_allocator(&mut allocator);

        vec![
            Word::shared_heap_reference(handle),
            Word::uint64(0),
            Word::uint64(4),
        ]
    })
    .expect("shared barrier write should succeed");
}

#[test]
fn test_barrier_write_rejects_invalid_range() {
    let mir = r#"
function test(v0: ref<int32, managed, readonly>, v1: uint64, v2: uint64): void {
b0(v0: ref<int32, managed, readonly>, v1: uint64, v2: uint64):
    barrier.write v0, v1, v2
    return
}"#;
    let result = run_mir_with_frame(mir, "test", |isolate| {
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
        let shape = isolate
            .isolate
            .allocation_shape(layout_id)
            .expect("managed pointee layout should resolve");
        let layout = isolate.heap.allocation_plan(shape);
        let handle = isolate
            .heap
            .allocate(&layout, Payload::Zeroed)
            .expect("heap allocation should succeed");

        vec![
            Word::heap_reference(handle),
            Word::uint64(4),
            Word::uint64(1),
        ]
    });

    assert_runtime_error_matches!(
        result,
        Error::InvariantViolation { ref context }
            if context == "invalid heap byte range: start 4, len 1, capacity 4"
    );
}

#[test]
fn test_intrinsic_raw_eq_true() {
    let mir = r#"
function test(v0: int32, v1: int32): boolean {
b0(v0: int32, v1: int32):
    v2: int32 = intrinsic.memory.raw.eq(v0, v1)
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
function test(v0: int32, v1: int32): boolean {
b0(v0: int32, v1: int32):
    v2: int32 = intrinsic.memory.raw.eq(v0, v1)
    return v2
}"#;
    run_mir_expect(
        mir,
        "test",
        &[Value::int32(42), Value::int32(43)],
        Value::bool(false),
    );
}

#[test]
fn test_intrinsic_breakpoint() {
    let mir = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    intrinsic.error.debug.breakpoint()
    return v0
}"#;
    run_mir_expect(mir, "test", &[Value::int32(42)], Value::int32(42));
}

#[test]
fn test_terminator_unreachable() {
    let mir = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    unreachable
}"#;
    let result = run_mir(mir, "test", &[Value::int32(42)]);
    assert!(result.is_err(), "expected unreachable error");
}

#[test]
fn test_terminator_trap_abort() {
    let mir = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    trap.abort
}"#;
    let result = run_mir(mir, "test", &[Value::int32(42)]);
    assert!(result.is_err(), "expected abort error");
}

#[test]
fn test_terminator_panic() {
    let mir = r#"
function test(): void {
b0:
    v0: ref<int32, managed, readonly> = new int32
    panic v0
}
"#;
    let result = run_mir(mir, "test", &[]);
    assert!(result.is_err(), "expected panic error");
}

#[test]
fn test_intrinsic_transmute() {
    let mir = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = intrinsic.memory.raw.transmute(v0)
    return v1
}"#;
    run_mir_expect(mir, "test", &[Value::int32(42)], Value::int32(42));
}

#[test]
fn test_intrinsic_return_address() {
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
    let output = run_mir_ok(mir, "test", &[]);
    let value = UnsignedInt::try_from(&output).expect("expected uint value");

    assert!(value.value != 0, "expected non-zero return address");
}

#[test]
fn test_intrinsic_return_address_no_caller() {
    let mir = r#"
function test(): uint64 {
b0:
    v0: uint64 = intrinsic.returnAddress()
    return v0
}"#;
    run_mir_expect(mir, "test", &[], Value::uint64(0));
}

#[test]
fn test_intrinsic_frame_address() {
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
    let value = UnsignedInt::try_from(&output).expect("expected uint value");

    assert!(
        value.value > 0x7FFF_0000_0000_0000u128,
        "expected synthetic frame address"
    );
}

#[test]
fn test_intrinsic_reduce_add() {
    let mir = r#"
function test(v0: vector<int32, 4>): int32 {
b0(v0: vector<int32, 4>):
    v1: int32 = vector.reduce add, v0
    return v1
}"#;
    let output = run_mir_with_frame_ok(mir, "test", |interp| {
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
    assert_eq!(output, Value::int32(10));
}

#[test]
fn test_intrinsic_reduce_mul() {
    let mir = r#"
function test(v0: vector<int32, 4>): int32 {
b0(v0: vector<int32, 4>):
    v1: int32 = vector.reduce mul, v0
    return v1
}"#;
    let output = run_mir_with_frame_ok(mir, "test", |interp| {
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
    assert_eq!(output, Value::int32(120));
}

#[test]
fn test_intrinsic_reduce_min() {
    let mir = r#"
function test(v0: vector<int32, 4>): int32 {
b0(v0: vector<int32, 4>):
    v1: int32 = vector.reduce min, v0
    return v1
}"#;
    let output = run_mir_with_frame_ok(mir, "test", |interp| {
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
    assert_eq!(output, Value::int32(1));
}

#[test]
fn test_intrinsic_reduce_max() {
    let mir = r#"
function test(v0: vector<int32, 4>): int32 {
b0(v0: vector<int32, 4>):
    v1: int32 = vector.reduce max, v0
    return v1
}"#;
    let output = run_mir_with_frame_ok(mir, "test", |interp| {
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
    assert_eq!(output, Value::int32(8));
}

#[test]
fn test_intrinsic_reduce_and() {
    let mir = r#"
function test(v0: vector<uint32, 4>): uint32 {
b0(v0: vector<uint32, 4>):
    v1: uint32 = vector.reduce and, v0
    return v1
}"#;
    let output = run_mir_with_frame_ok(mir, "test", |interp| {
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
    assert_eq!(output, Value::uint32(0b1000));
}

#[test]
fn test_intrinsic_reduce_or() {
    let mir = r#"
function test(v0: vector<uint32, 4>): uint32 {
b0(v0: vector<uint32, 4>):
    v1: uint32 = vector.reduce or, v0
    return v1
}"#;
    let output = run_mir_with_frame_ok(mir, "test", |interp| {
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
    assert_eq!(output, Value::uint32(0b1111));
}

#[test]
fn test_intrinsic_reduce_xor() {
    let mir = r#"
function test(v0: vector<uint32, 4>): uint32 {
b0(v0: vector<uint32, 4>):
    v1: uint32 = vector.reduce xor, v0
    return v1
}"#;
    let output = run_mir_with_frame_ok(mir, "test", |interp| {
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
    assert_eq!(output, Value::uint32(4));
}
