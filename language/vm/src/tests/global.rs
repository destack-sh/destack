use crate::Word;
use crate::diagnostic::Error;
use crate::tests::{
    assert_runtime_error_matches, assert_value_word, run_mir, run_mir_expect, run_mir_ok,
};

/// Global constant can be read.
#[test]
fn test_global_get_constant() {
    let mir = r#"
global value: int32, readonly = 42int32

function read(): int32 {
b0:
    v0: ref<int32, raw, readonly> = global.address value
    v1: int32 = load v0
    return v1
}"#;
    run_mir_expect(mir, "read", &[], Word::int32(42));
}

/// Mutable global can be written and read back.
#[test]
fn test_global_set() {
    let mir = r#"
global counter: int32 = 0int32

function increment(): int32 {
b0:
    v0: ref<int32, raw, space(local)> = global.address counter
    v1: int32 = load v0
    v2: int32 = 1int32
    v3: int32 = int.add v1, v2
    store v0, v3
    v4: int32 = load v0
    return v4
}"#;
    run_mir_expect(mir, "increment", &[], Word::int32(1));
}

/// Writing to immutable global via pointer produces an error.
#[test]
fn test_global_immutable_write() {
    let mir = r#"
global CONST: int32, readonly = 42int32

function badWrite(): void {
b0:
    v0: ref<int32, raw, readonly, space(local)> = global.address CONST
    v1: int32 = 99int32
    store v0, v1
    return
}"#;
    let result = run_mir(mir, "badWrite", &[]);

    assert_runtime_error_matches!(result, Error::ImmutableGlobalWrite { .. });
}

/// Global state persists across function calls.
#[test]
fn test_global_persists_across_calls() {
    let mir = r#"
global counter: int32 = 0int32

function inc(): void {
b0:
    v0: ref<int32, raw, space(local)> = global.address counter
    v1: int32 = load v0
    v2: int32 = 1int32
    v3: int32 = int.add v1, v2
    store v0, v3
    return
}
function get(): int32 {
b0:
    v0: ref<int32, raw, space(local)> = global.address counter
    v1: int32 = load v0
    return v1
}
function main(): int32 {
b0:
    call inc(): () -> void
    call inc(): () -> void
    call inc(): () -> void
    v0: int32 = call get(): () -> int32
    return v0
}"#;
    run_mir_expect(mir, "main", &[], Word::int32(3));
}

/// Zero-initialized global starts at typed zero.
#[test]
fn test_global_zeroinit() {
    let mir = r#"
global data: int32 = zeroInit

function read(): int32 {
b0:
    v0: ref<int32, raw, space(local)> = global.address data
    v1: int32 = load v0
    return v1
}"#;
    run_mir_expect(mir, "read", &[], Word::int32(0));
}

/// Zero-initialized float global.
#[test]
fn test_global_zeroinit_float() {
    let mir = r#"
global data: float64 = zeroInit

function read(): float64 {
b0:
    v0: ref<float64, raw, space(local)> = global.address data
    v1: float64 = load v0
    return v1
}"#;
    run_mir_expect(mir, "read", &[], Word::float64(0.0));
}

/// Zero-initialized bool global.
#[test]
fn test_global_zeroinit_bool() {
    let mir = r#"
global flag: boolean = zeroInit

function read(): boolean {
b0:
    v0: ref<boolean, raw, space(local)> = global.address flag
    v1: boolean = load v0
    return v1
}"#;
    run_mir_expect(mir, "read", &[], Word::bool(false));
}

/// Payload global initializer with scalar values.
#[test]
fn test_global_payload() {
    let mir = r#"
global pair: (int32, int32), readonly = {10int32, 20int32}

function getSecond(): int32 {
b0:
    v0: ref<(int32, int32), raw, readonly> = global.address pair
    v1: (int32, int32) = load v0
    v2: int32 = field.get v1, 1
    return v2
}"#;
    run_mir_expect(mir, "getSecond", &[], Word::int32(20));
}

/// String global initializer materializes as typed UTF-8 byte storage.
#[test]
fn test_global_string_bytes() {
    let mir = r#"
global message: uint8[4], readonly = b"boom"

function readSecond(): uint8 {
b0:
    v0: ref<uint8[4], raw, readonly> = global.address message
    v1: uint8[4] = load v0
    v2: uint64 = 1uint64
    v3: uint8 = element.get v1, v2
    return v3
}"#;
    run_mir_expect(mir, "readSecond", &[], Word::uint8(b'o'));
}

/// Multiple globals can coexist.
#[test]
fn test_multiple_globals() {
    let mir = r#"
global first: int32, readonly = 10int32
global second: int32, readonly = 20int32
global third: int32 = 30int32

function sum(): int32 {
b0:
    v0: ref<int32, raw, readonly> = global.address first
    v1: int32 = load v0
    v2: ref<int32, raw, readonly> = global.address second
    v3: int32 = load v2
    v4: ref<int32, raw, space(local)> = global.address third
    v5: int32 = load v4
    v6: int32 = int.add v1, v3
    v7: int32 = int.add v6, v5
    return v7
}"#;
    run_mir_expect(mir, "sum", &[], Word::int32(60));
}

/// Global can be modified multiple times.
#[test]
fn test_global_multiple_writes() {
    let mir = r#"
global value: int32 = 0int32

function test(): int32 {
b0:
    v0: ref<int32, raw, space(local)> = global.address value
    v1: int32 = 10int32
    store v0, v1
    v2: int32 = 20int32
    store v0, v2
    v3: int32 = 30int32
    store v0, v3
    v4: int32 = load v0
    return v4
}"#;
    run_mir_expect(mir, "test", &[], Word::int32(30));
}

/// Global with negative initial value.
#[test]
fn test_global_negative_init() {
    let mir = r#"
global neg: int32, readonly = -42int32

function read(): int32 {
b0:
    v0: ref<int32, raw, readonly> = global.address neg
    v1: int32 = load v0
    return v1
}"#;
    run_mir_expect(mir, "read", &[], Word::int32(-42));
}

/// Global float with initial value.
#[test]
#[allow(clippy::approx_constant)]
fn test_global_float() {
    let mir = r#"
global pi: float64, readonly = 3.14159float64

function read(): float64 {
b0:
    v0: ref<float64, raw, readonly> = global.address pi
    v1: float64 = load v0
    return v1
}"#;
    let output = run_mir_ok(mir, "read", &[]);
    let f = assert_value_word(&output.value).as_float64();
    assert!((f - 3.14159).abs() < 0.0001);
}

/// Global bool with initial value.
#[test]
fn test_global_bool() {
    let mir = r#"
global flag: boolean = true

function toggle(): boolean {
b0:
    v0: ref<boolean, raw, space(local)> = global.address flag
    v1: boolean = load v0
    v2: boolean = int.not v1
    store v0, v2
    v3: boolean = load v0
    return v3
}"#;
    run_mir_expect(mir, "toggle", &[], Word::bool(false));
}
