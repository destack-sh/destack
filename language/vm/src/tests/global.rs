use crate::diagnostic::Error;
use crate::tests::{run_mir, run_mir_expect, run_mir_ok};
use destack_heap::Value;

/// Global constant can be read.
#[test]
fn test_global_get_constant() {
    let mir = r#"
global @value: i32 = 42i32 ; readonly

function @read() -> i32 {
block0:
    v0: i32 = global.const @value
    return v0
}"#;
    run_mir_expect(mir, "read", &[], Value::int32(42));
}

/// Mutable global can be written and read back.
#[test]
fn test_global_set() {
    let mir = r#"
global @counter: i32 = 0i32 ;

function @increment() -> i32 {
block0:
    v0: ref<raw addrspace(global) i32> = global.addr @counter
    v1: i32 = load v0
    v2: i32 = iconst 1i32
    v3: i32 = iadd v1, v2
    store v0, v3
    v4: i32 = load v0
    return v4
}"#;
    run_mir_expect(mir, "increment", &[], Value::int32(1));
}

/// Writing to immutable global via pointer produces an error.
#[test]
fn test_global_immutable_write() {
    let mir = r#"
global @CONST: i32 = 42i32 ; readonly

function @bad_write() -> void {
block0:
    v0: ref<raw addrspace(global) readonly i32> = global.addr @CONST
    v1: i32 = iconst 99i32
    store v0, v1
    return
}"#;
    let result = run_mir(mir, "bad_write", &[]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.error, Error::ImmutableGlobalWrite { .. }));
}

/// Global state persists across function calls.
#[test]
fn test_global_persists_across_calls() {
    let mir = r#"
global @counter: i32 = 0i32 ;

function @inc() -> void {
block0:
    v0: ref<raw addrspace(global) i32> = global.addr @counter
    v1: i32 = load v0
    v2: i32 = iconst 1i32
    v3: i32 = iadd v1, v2
    store v0, v3
    return
}

function @get() -> i32 {
block0:
    v0: ref<raw addrspace(global) i32> = global.addr @counter
    v1: i32 = load v0
    return v1
}

function @main() -> i32 {
block0:
    call @inc() -> fn() -> void
    call @inc() -> fn() -> void
    call @inc() -> fn() -> void
    v0: i32 = call @get() -> fn() -> i32
    return v0
}"#;
    run_mir_expect(mir, "main", &[], Value::int32(3));
}

/// Zero-initialized global starts at typed zero.
#[test]
fn test_global_zeroinit() {
    let mir = r#"
global @data: i32 = zeroinit ;

function @read() -> i32 {
block0:
    v0: ref<raw addrspace(global) i32> = global.addr @data
    v1: i32 = load v0
    return v1
}"#;
    run_mir_expect(mir, "read", &[], Value::int32(0));
}

/// Zero-initialized float global.
#[test]
fn test_global_zeroinit_float() {
    let mir = r#"
global @data: f64 = zeroinit ;

function @read() -> f64 {
block0:
    v0: ref<raw addrspace(global) f64> = global.addr @data
    v1: f64 = load v0
    return v1
}"#;
    run_mir_expect(mir, "read", &[], Value::float64(0.0));
}

/// Zero-initialized bool global.
#[test]
fn test_global_zeroinit_bool() {
    let mir = r#"
global @flag: bool = zeroinit ;

function @read() -> bool {
block0:
    v0: ref<raw addrspace(global) bool> = global.addr @flag
    v1: bool = load v0
    return v1
}"#;
    run_mir_expect(mir, "read", &[], Value::bool(false));
}

/// Aggregate global initializer with scalar values.
#[test]
fn test_global_aggregate() {
    let mir = r#"
global @pair: (i32, i32) = {10i32, 20i32} ; readonly

function @get_second() -> i32 {
block0:
    v0: (i32, i32) = global.const @pair
    v1: i32 = field.get v0, 1
    return v1
}"#;
    run_mir_expect(mir, "get_second", &[], Value::int32(20));
}

/// Multiple globals can coexist.
#[test]
fn test_multiple_globals() {
    let mir = r#"
global @a: i32 = 10i32 ; readonly
global @b: i32 = 20i32 ; readonly
global @c: i32 = 30i32 ;

function @sum() -> i32 {
block0:
    v0: i32 = global.const @a
    v1: i32 = global.const @b
    v2: ref<raw addrspace(global) i32> = global.addr @c
    v3: i32 = load v2
    v4: i32 = iadd v0, v1
    v5: i32 = iadd v4, v3
    return v5
}"#;
    run_mir_expect(mir, "sum", &[], Value::int32(60));
}

/// Global can be modified multiple times.
#[test]
fn test_global_multiple_writes() {
    let mir = r#"
global @value: i32 = 0i32 ;

function @test() -> i32 {
block0:
    v0: ref<raw addrspace(global) i32> = global.addr @value
    v1: i32 = iconst 10i32
    store v0, v1
    v2: i32 = iconst 20i32
    store v0, v2
    v3: i32 = iconst 30i32
    store v0, v3
    v4: i32 = load v0
    return v4
}"#;
    run_mir_expect(mir, "test", &[], Value::int32(30));
}

/// Global with negative initial value.
#[test]
fn test_global_negative_init() {
    let mir = r#"
global @neg: i32 = -42i32 ; readonly

function @read() -> i32 {
block0:
    v0: i32 = global.const @neg
    return v0
}"#;
    run_mir_expect(mir, "read", &[], Value::int32(-42));
}

/// Global float with initial value.
#[test]
#[allow(clippy::approx_constant)]
fn test_global_float() {
    let mir = r#"
global @pi: f64 = 3.14159f64 ; readonly

function @read() -> f64 {
block0:
    v0: f64 = global.const @pi
    return v0
}"#;
    let output = run_mir_ok(mir, "read", &[]);
    let f = output.value.as_float64().expect("expected Float64");
    assert!((f - 3.14159).abs() < 0.0001);
}

/// Global bool with initial value.
#[test]
fn test_global_bool() {
    let mir = r#"
global @flag: bool = true ;

function @toggle() -> bool {
block0:
    v0: ref<raw addrspace(global) bool> = global.addr @flag
    v1: bool = load v0
    v2: bool = bnot v1
    store v0, v2
    v3: bool = load v0
    return v3
}"#;
    run_mir_expect(mir, "toggle", &[], Value::bool(false));
}
