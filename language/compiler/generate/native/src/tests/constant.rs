use super::compile_mir_to_normalized_clif;

/// String global lowers to a data section and global_value reference.
#[test]
fn test_string_global() {
    let mir = r#"
readonly global hello: [uint8; 5], space(static) = b"hello";
function get_hello(): ref<[uint8; 5], raw, readonly, space(static)> {
b0:
    v0: ref<[uint8; 5], raw, readonly, space(static)> = global.address hello
    return v0
}"#;
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
readonly global empty: [uint8; 0], space(static) = b"";
function get_empty(): ref<[uint8; 0], raw, readonly, space(static)> {
b0:
    v0: ref<[uint8; 0], raw, readonly, space(static)> = global.address empty
    return v0
}"#;
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
readonly global magic: int32, space(static) = 42int32;
function get_magic(): int32 {
b0:
    v0: ref<int32, raw, readonly, space(static)> = global.address magic
    v1: int32 = load v0
    return v1
}"#;
    let clif = compile_mir_to_normalized_clif(mir);

    // global.address should produce global_value + load
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
global counter: int32, space(static) = 0int32 ;

function increment(): int32 {
b0:
    v0: ref<int32, raw, space(static)> = global.address counter
    v1: int32 = load v0
    v2: int32 = 1int32
    v3: int32 = int.add v1, v2
    store v0, v3
    v4: int32 = load v0
    return v4
}"#;
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
global data: int64, space(static) = zeroInit ;

function get_data(): int64 {
b0:
    v0: ref<int64, raw, space(static)> = global.address data
    v1: int64 = load v0
    return v1
}"#;
    let clif = compile_mir_to_normalized_clif(mir);
    assert!(
        clif.contains("global_value"),
        "expected global_value in: {clif}"
    );
    assert!(clif.contains("load"), "expected load in: {clif}");
}
