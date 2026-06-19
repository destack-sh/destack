use crate::diagnostic::{Error, Trap};
use crate::tests::{assert_runtime_error_matches, run_mir, run_mir_expect, run_mir_ok};
use destack_program::Value;

/// Global constant can be read.
#[test]
fn test_global_get_constant() {
    let mir = r#"
readonly global value: int32 = 42

function read(): int32 {
entry:
    v0: ref<int32, raw, readonly> = global.address value
    v1: int32 = load v0
    return v1
}
"#;
    run_mir_expect(mir, "read", &[], Value::int32(42));
}

/// Mutable global can be written and read back.
#[test]
fn test_global_set() {
    let mir = r#"
global counter: int32 = 0

function increment(): int32 {
entry:
    v0: ref<int32, raw, mutable> = global.address counter
    v1: int32 = load v0
    v2: int32 = 1
    v3: int32 = int.add v1, v2
    store v0, v3
    v4: int32 = load v0
    return v4
}
"#;
    run_mir_expect(mir, "increment", &[], Value::int32(1));
}

/// Writing to immutable global via pointer produces an error.
#[test]
fn test_global_immutable_write() {
    let mir = r#"
readonly global CONST: int32 = 42

function badWrite(): void {
entry:
    v0: ref<int32, raw, readonly> = global.address CONST
    v1: int32 = 99
    store v0, v1
    return
}
"#;
    let result = run_mir(mir, "badWrite", &[]);

    assert_runtime_error_matches!(
        result,
        Error::Trap {
            reason: Trap::ImmutableGlobalWrite { .. },
        },
    );
}

/// Global state persists across function calls.
#[test]
fn test_global_persists_across_calls() {
    let mir = r#"
global counter: int32 = 0

function inc(): void {
entry:
    v0: ref<int32, raw, mutable> = global.address counter
    v1: int32 = load v0
    v2: int32 = 1
    v3: int32 = int.add v1, v2
    store v0, v3
    return
}

function get(): int32 {
entry:
    v0: ref<int32, raw, mutable> = global.address counter
    v1: int32 = load v0
    return v1
}

function main(): int32 {
entry:
    call inc()
    call inc()
    call inc()
    v0: int32 = call get()
    return v0
}
"#;
    run_mir_expect(mir, "main", &[], Value::int32(3));
}

/// Zero-initialized global starts at typed zero.
#[test]
fn test_global_zeroinit() {
    let mir = r#"
global data: int32 = zeroInit

function read(): int32 {
entry:
    v0: ref<int32, raw, mutable> = global.address data
    v1: int32 = load v0
    return v1
}
"#;
    run_mir_expect(mir, "read", &[], Value::int32(0));
}

/// Zero-initialized float global.
#[test]
fn test_global_zeroinit_float() {
    let mir = r#"
global data: float64 = zeroInit

function read(): float64 {
entry:
    v0: ref<float64, raw, mutable> = global.address data
    v1: float64 = load v0
    return v1
}
"#;
    run_mir_expect(mir, "read", &[], Value::float64(0.0));
}

/// Zero-initialized bool global.
#[test]
fn test_global_zeroinit_bool() {
    let mir = r#"
global flag: boolean = zeroInit

function read(): boolean {
entry:
    v0: ref<boolean, raw, mutable> = global.address flag
    v1: boolean = load v0
    return v1
}
"#;
    run_mir_expect(mir, "read", &[], Value::bool(false));
}

/// Aggregate global initializer stores scalar fields.
#[test]
fn test_global_aggregate() {
    let mir = r#"
readonly global pair: (int32, int32) = {10int32, 20int32}

function getSecond(): int32 {
entry:
    v0: ref<(int32, int32), raw, readonly> = global.address pair
    v1: (int32, int32) = load v0
    v2: int32 = field.get v1, 1
    return v2
}
"#;
    run_mir_expect(mir, "getSecond", &[], Value::int32(20));
}

/// String global initializer materializes as typed UTF-8 byte storage.
#[test]
fn test_global_string_bytes() {
    let mir = r#"
readonly global message: [uint8; 4] = b"boom"

function readSecond(): uint8 {
entry:
    v0: ref<[uint8; 4], raw, readonly> = global.address message
    v1: [uint8; 4] = load v0
    v2: uint8 = field.get v1, 1
    return v2
}
"#;
    run_mir_expect(mir, "readSecond", &[], Value::uint8(b'o'));
}

/// Multiple globals can coexist.
#[test]
fn test_multiple_globals() {
    let mir = r#"
readonly global first: int32 = 10

readonly global second: int32 = 20

global third: int32 = 30

function sum(): int32 {
entry:
    v0: ref<int32, raw, readonly> = global.address first
    v1: int32 = load v0
    v2: ref<int32, raw, readonly> = global.address second
    v3: int32 = load v2
    v4: ref<int32, raw, mutable> = global.address third
    v5: int32 = load v4
    v6: int32 = int.add v1, v3
    v7: int32 = int.add v6, v5
    return v7
}
"#;
    run_mir_expect(mir, "sum", &[], Value::int32(60));
}

/// Global can be modified multiple times.
#[test]
fn test_global_multiple_writes() {
    let mir = r#"
global value: int32 = 0

function test(): int32 {
entry:
    v0: ref<int32, raw, mutable> = global.address value
    v1: int32 = 10
    store v0, v1
    v2: int32 = 20
    store v0, v2
    v3: int32 = 30
    store v0, v3
    v4: int32 = load v0
    return v4
}
"#;
    run_mir_expect(mir, "test", &[], Value::int32(30));
}

/// Global with negative initial value.
#[test]
fn test_global_negative_init() {
    let mir = r#"
readonly global neg: int32 = -42

function read(): int32 {
entry:
    v0: ref<int32, raw, readonly> = global.address neg
    v1: int32 = load v0
    return v1
}
"#;
    run_mir_expect(mir, "read", &[], Value::int32(-42));
}

/// Global float with initial value.
#[test]
fn test_global_float() {
    let mir = r#"
readonly global pi: float64 = 3.14159

function read(): float64 {
entry:
    v0: ref<float64, raw, readonly> = global.address pi
    v1: float64 = load v0
    return v1
}
"#;
    let output = run_mir_ok(mir, "read", &[]);
    let f = f64::try_from(&output).expect("expected float64 value");

    assert!((f - 3.14159).abs() < 0.0001);
}

/// Global bool with initial value.
#[test]
fn test_global_bool() {
    let mir = r#"
global flag: boolean = true

function toggle(): boolean {
entry:
    v0: ref<boolean, raw, mutable> = global.address flag
    v1: boolean = load v0
    v2: boolean = int.not v1
    store v0, v2
    v3: boolean = load v0
    return v3
}
"#;
    run_mir_expect(mir, "toggle", &[], Value::bool(false));
}
