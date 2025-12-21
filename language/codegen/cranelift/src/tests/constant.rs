use super::compile_mir_to_normalized_clif;

/// String global lowers to a data section and global_value reference.
#[test]
fn test_string_global() {
    let mir = r#"
global @hello: rawptr<i8> = "hello" ; const

function @get_hello() -> i64 {
block0:
    v0 = global.addr @hello
    return v0
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    // string globals create a global_value instruction
    assert!(
        clif.contains("global_value"),
        "expected global_value in: {clif}"
    );
}

/// Empty string global.
#[test]
fn test_empty_string_global() {
    let mir = r#"
global @empty: rawptr<i8> = "" ; const

function @get_empty() -> i64 {
block0:
    v0 = global.addr @empty
    return v0
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);
    assert!(
        clif.contains("global_value"),
        "expected global_value in: {clif}"
    );
}

/// Integer global constant.
#[test]
fn test_integer_global_const() {
    let mir = r#"
global @magic: i32 = 42i32 ; const

function @get_magic() -> i32 {
block0:
    v0 = global.const @magic
    return v0
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);

    // global.const should produce global_value + load
    assert!(
        clif.contains("global_value"),
        "expected global_value in: {clif}"
    );
    assert!(clif.contains("load"), "expected load in: {clif}");
}

/// Mutable global variable with load/store.
#[test]
fn test_mutable_global() {
    let mir = r#"
global @counter: i32 = 0i32 ; mut

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
    let clif = compile_mir_to_normalized_clif(mir);

    // mutable global should produce global_value, load, and store
    assert!(
        clif.contains("global_value"),
        "expected global_value in: {clif}"
    );
    assert!(clif.contains("load"), "expected load in: {clif}");
    assert!(clif.contains("store"), "expected store in: {clif}");
}

/// Zero-initialized global.
#[test]
fn test_zeroinit_global() {
    let mir = r#"
global @data: i64 = zeroinit ; mut

function @get_data() -> i64 {
block0:
    v0 = global.addr @data
    v1 = load v0
    return v1
}
"#;
    let clif = compile_mir_to_normalized_clif(mir);
    assert!(
        clif.contains("global_value"),
        "expected global_value in: {clif}"
    );
    assert!(clif.contains("load"), "expected load in: {clif}");
}
