use super::assert_format;

/// Formats mutable globals and global address operations canonically.
#[test]
fn test_format_global_variable() {
    assert_format(
        r#"
global counter: int32 = zeroInit

function increment(): void {
entry:
    v0: ref<int32, raw> = global.address counter
    v1: int32 = load v0
    v2: int32 = 1
    v3: int32 = int.add v1, v2
    store v0, v3
    return
}
"#,
    );
}

/// Formats immutable globals and loads canonically.
#[test]
fn test_format_global_constant() {
    assert_format(
        r#"
readonly global MAGIC: int64 = 42

function getMagic(): int64 {
entry:
    v0: ref<int64, raw, readonly> = global.address MAGIC
    v1: int64 = load v0
    return v1
}
"#,
    );
}

/// Formats string constants and escapes canonically.
#[test]
fn test_format_string_constant() {
    assert_format(
        r#"
readonly global stringLiteralHelloWorldNl: [uint8; 11] = b"hello\nworld"

function escapeTest(): void {
entry:
    v0: ref<[uint8; 11], raw, readonly> = global.address stringLiteralHelloWorldNl
    v1: [uint8; 11] = load v0
    return
}
"#,
    );
}
