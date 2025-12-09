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
    v0 = global.get @value
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
    v0 = global.get @counter
    v1 = iconst 1i32
    v2 = iadd v0, v1
    global.set @counter, v2
    v3 = global.get @counter
    return v3
}
"#;
    run_mir_expect(mir, "increment", &[], Value::int32(1));
}

/// Writing to immutable global produces an error.
#[test]
fn test_global_immutable_write() {
    let mir = r#"
global @CONST: i32 = 42i32 ; const

function @bad_write() -> void {
block0:
    v0 = iconst 99i32
    global.set @CONST, v0
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
    v0 = global.get @counter
    v1 = iconst 1i32
    v2 = iadd v0, v1
    global.set @counter, v2
    return
}

function @get() -> i32 {
block0:
    v0 = global.get @counter
    return v0
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
    v0 = global.get @data
    return v0
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
    v0 = global.get @data
    return v0
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
    v0 = global.get @flag
    return v0
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
    v0 = global.get @pair
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
    v0 = global.get @a
    v1 = global.get @b
    v2 = global.get @c
    v3 = iadd v0, v1
    v4 = iadd v3, v2
    return v4
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
    v0 = iconst 10i32
    global.set @value, v0
    v1 = iconst 20i32
    global.set @value, v1
    v2 = iconst 30i32
    global.set @value, v2
    v3 = global.get @value
    return v3
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
    v0 = global.get @neg
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
    v0 = global.get @pi
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
    v0 = global.get @flag
    v1 = bnot v0
    global.set @flag, v1
    v2 = global.get @flag
    return v2
}
"#;
    run_mir_expect(mir, "toggle", &[], Value::Bool(false));
}
