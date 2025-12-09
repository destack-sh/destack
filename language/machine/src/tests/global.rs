use crate::diagnostic::Error;
use crate::memory::Value;
use crate::tests::{run_mir, run_mir_expect, run_mir_ok};

/// Global constant can be read.
#[test]
fn test_global_get_constant() {
    let mir = r#"
global @value: i32 = 42i32 ; const

function @read() -> i32 {
block0:
    v0 = global.const @value
    return v0
}
"#;
    run_mir_expect(mir, "read", &[], Value::int32(42));
}

/// Mutable global can be written and read back.
#[test]
fn test_global_set() {
    let mir = r#"
global @counter: i32 = 0i32 ; var

function @increment() -> i32 {
block0:
    v0 = global.addr @counter
    v1 = load v0
    v2 = iconst 1i32
    v3 = iadd v1, v2
    store v0, v3
    v4 = load v0
    return v4
}
"#;
    run_mir_expect(mir, "increment", &[], Value::int32(1));
}

/// Writing to immutable global via pointer produces an error.
#[test]
fn test_global_immutable_write() {
    let mir = r#"
global @CONST: i32 = 42i32 ; const

function @bad_write() -> void {
block0:
    v0 = global.addr @CONST
    v1 = iconst 99i32
    store v0, v1
    return
}
"#;
    let result = run_mir(mir, "bad_write", &[]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.error, Error::ImmutableGlobalWrite { .. }));
}

/// Global state persists across function calls.
#[test]
fn test_global_persists_across_calls() {
    let mir = r#"
global @counter: i32 = 0i32 ; var

function @inc() -> void {
block0:
    v0 = global.addr @counter
    v1 = load v0
    v2 = iconst 1i32
    v3 = iadd v1, v2
    store v0, v3
    return
}

function @get() -> i32 {
block0:
    v0 = global.addr @counter
    v1 = load v0
    return v1
}

function @main() -> i32 {
block0:
    call @inc()
    call @inc()
    call @inc()
    v0 = call @get()
    return v0
}
"#;
    run_mir_expect(mir, "main", &[], Value::int32(3));
}

/// Zero-initialized global starts at typed zero.
#[test]
fn test_global_zeroinit() {
    let mir = r#"
global @data: i32 = zeroinit ; var

function @read() -> i32 {
block0:
    v0 = global.addr @data
    v1 = load v0
    return v1
}
"#;
    run_mir_expect(mir, "read", &[], Value::int32(0));
}

/// Zero-initialized float global.
#[test]
fn test_global_zeroinit_float() {
    let mir = r#"
global @data: f64 = zeroinit ; var

function @read() -> f64 {
block0:
    v0 = global.addr @data
    v1 = load v0
    return v1
}
"#;
    run_mir_expect(mir, "read", &[], Value::float64(0.0));
}

/// Zero-initialized bool global.
#[test]
fn test_global_zeroinit_bool() {
    let mir = r#"
global @flag: bool = zeroinit ; var

function @read() -> bool {
block0:
    v0 = global.addr @flag
    v1 = load v0
    return v1
}
"#;
    run_mir_expect(mir, "read", &[], Value::Bool(false));
}

/// Aggregate global initializer with scalar values.
#[test]
fn test_global_aggregate() {
    let mir = r#"
global @pair: (i32, i32) = {10i32, 20i32} ; const

function @get_second() -> i32 {
block0:
    v0 = global.const @pair
    v1 = field.get v0, 1
    return v1
}
"#;
    run_mir_expect(mir, "get_second", &[], Value::int32(20));
}

/// Multiple globals can coexist.
#[test]
fn test_multiple_globals() {
    let mir = r#"
global @a: i32 = 10i32 ; const
global @b: i32 = 20i32 ; const
global @c: i32 = 30i32 ; var

function @sum() -> i32 {
block0:
    v0 = global.const @a
    v1 = global.const @b
    v2 = global.addr @c
    v3 = load v2
    v4 = iadd v0, v1
    v5 = iadd v4, v3
    return v5
}
"#;
    run_mir_expect(mir, "sum", &[], Value::int32(60));
}

/// Global can be modified multiple times.
#[test]
fn test_global_multiple_writes() {
    let mir = r#"
global @value: i32 = 0i32 ; var

function @test() -> i32 {
block0:
    v0 = global.addr @value
    v1 = iconst 10i32
    store v0, v1
    v2 = iconst 20i32
    store v0, v2
    v3 = iconst 30i32
    store v0, v3
    v4 = load v0
    return v4
}
"#;
    run_mir_expect(mir, "test", &[], Value::int32(30));
}

/// Global with negative initial value.
#[test]
fn test_global_negative_init() {
    let mir = r#"
global @neg: i32 = -42i32 ; const

function @read() -> i32 {
block0:
    v0 = global.const @neg
    return v0
}
"#;
    run_mir_expect(mir, "read", &[], Value::int32(-42));
}

/// Global float with initial value.
#[test]
fn test_global_float() {
    let mir = r#"
global @pi: f64 = 3.14159f64 ; const

function @read() -> f64 {
block0:
    v0 = global.const @pi
    return v0
}
"#;
    let output = run_mir_ok(mir, "read", &[]);
    match output.value {
        #[allow(clippy::approx_constant)]
        Value::Float64(f) => assert!((f - 3.14159).abs() < 0.0001),
        _ => panic!("expected Float64"),
    }
}

/// Global bool with initial value.
#[test]
fn test_global_bool() {
    let mir = r#"
global @flag: bool = true ; var

function @toggle() -> bool {
block0:
    v0 = global.addr @flag
    v1 = load v0
    v2 = bnot v1
    store v0, v2
    v3 = load v0
    return v3
}
"#;
    run_mir_expect(mir, "toggle", &[], Value::Bool(false));
}
